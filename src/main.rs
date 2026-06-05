#![no_std]
#![no_main]

use panic_halt as _;
use cortex_m_rt::entry;
use cortex_m::peripheral::NVIC;
use stm32f4xx_hal::{
    pac::{self, interrupt, Interrupt},
    prelude::*,
    timer::{Event},
    serial::{Serial, config::Config as SerialConfig},
};
use core::sync::atomic::{AtomicU8, Ordering};
use core::cell::UnsafeCell;

mod oscillator;
mod filter;
mod envelope;
mod midi;
mod voice;
mod patch;

use midi::{MidiParser, MidiMessage};
use voice::Voice;
use patch::Patch;

// === Audio constants ===
const SAMPLE_RATE: u32 = 48000;
const NUM_VOICES: usize = 8;

// === Sine wave test ===
const TEST_FREQ: f32 = 440.0;
static mut SINE_PHASE: f32 = 0.0;

// === Voice array ===
static mut VOICES: [Voice; NUM_VOICES] = [const { Voice::new() }; NUM_VOICES];
static mut CURRENT_PATCH: Patch = Patch::init();

// === MIDI ring buffer (ISR produces, main consumes) ===
struct RingBuffer<T: Copy, const N: usize> {
    buf: UnsafeCell<[T; N]>,
    head: AtomicU8,
    tail: AtomicU8,
}

unsafe impl<T: Copy + Send, const N: usize> Sync for RingBuffer<T, N> {}

impl<T: Copy, const N: usize> RingBuffer<T, N> {
    const fn new(init: T) -> Self {
        RingBuffer {
            buf: UnsafeCell::new([init; N]),
            head: AtomicU8::new(0),
            tail: AtomicU8::new(0),
        }
    }

    fn push(&self, item: T) -> bool {
        let head = self.head.load(Ordering::Relaxed);
        let next = (head + 1) % (N as u8);
        if next == self.tail.load(Ordering::Acquire) {
            return false; // Full
        }
        unsafe {
            (*self.buf.get())[head as usize] = item;
        }
        self.head.store(next, Ordering::Release);
        true
    }

    fn pop(&self) -> Option<T> {
        let tail = self.tail.load(Ordering::Relaxed);
        if tail == self.head.load(Ordering::Acquire) {
            return None; // Empty
        }
        let item = unsafe { (*self.buf.get())[tail as usize] };
        self.tail.store((tail + 1) % (N as u8), Ordering::Release);
        Some(item)
    }
}

static MIDI_QUEUE: RingBuffer<MidiMessage, 32> = RingBuffer::new(
    MidiMessage::NoteOn { channel: 0, note: 0, velocity: 0 }
);

static mut MIDI_PARSER: MidiParser = MidiParser::new();

#[entry]
fn main() -> ! {
    // === Peripherals ===
    let dp = pac::Peripherals::take().unwrap();
    let _cp = cortex_m::Peripherals::take().unwrap();

    // === Clocks: 168 MHz from 8 MHz HSE ===
    let rcc = dp.RCC.constrain();
    let clocks = rcc.cfgr
        .use_hse(8.MHz())
        .sysclk(168.MHz())
        .hclk(168.MHz())
        .pclk1(42.MHz())
        .pclk2(84.MHz())
        .freeze();

    // === GPIO ===
    let gpioa = dp.GPIOA.split();
    let gpiob = dp.GPIOB.split();

    // === UART2 for MIDI (PA2=TX, PA3=RX) ===
    let tx_pin = gpioa.pa2.into_alternate();
    let rx_pin = gpioa.pa3.into_alternate();

    let uart_config = SerialConfig::default()
        .baudrate(31250.bps())
        .parity_none()
        .stopbits(stm32f4xx_hal::serial::config::StopBits::STOP1);

    let serial: Serial<stm32f4xx_hal::pac::USART2, u8> = Serial::new(
        dp.USART2,
        (tx_pin, rx_pin),
        uart_config,
        &clocks
    ).unwrap();

    let (mut _tx, mut rx) = serial.split();

    // Enable RXNE interrupt
    rx.listen();

    // === I2S2 for PCM5102A DAC ===
    // PB12 = WS (LRCK), PB13 = CK (BCLK), PB15 = SD (DATA)
    let _i2s_ws = gpiob.pb12.into_alternate::<5>();
    let _i2s_ck = gpiob.pb13.into_alternate::<5>();
    let _i2s_sd = gpiob.pb15.into_alternate::<5>();

    setup_i2s2(&dp.SPI2, &clocks);

    // === TIM2: 48kHz audio ISR ===
    let mut timer = dp.TIM2.counter_hz(&clocks);
    timer.start(48.kHz()).unwrap();
    timer.listen(Event::Update);

    // === Enable interrupts ===
    unsafe {
        NVIC::unmask(Interrupt::TIM2);
        NVIC::unmask(Interrupt::USART2);
    }

    // === Main loop ===
    loop {
        // Process MIDI messages
        while let Some(msg) = MIDI_QUEUE.pop() {
            handle_midi(msg);
        }

        // TODO v0.2.0: Parameter updates, display, patch I/O

        cortex_m::asm::wfi();
    }
}

fn handle_midi(msg: MidiMessage) {
    match msg {
        MidiMessage::NoteOn { note, velocity, .. } => {
            if velocity > 0 {
                unsafe {
                    trigger_voice(note, velocity);
                }
            } else {
                unsafe {
                    release_voice(note);
                }
            }
        }
        MidiMessage::NoteOff { note, .. } => {
            unsafe {
                release_voice(note);
            }
        }
        MidiMessage::ControlChange { controller, value, .. } => {
            let _ = (controller, value);
            // TODO v0.2.0: Map CC to parameters
        }
        _ => {}
    }
}

unsafe fn trigger_voice(note: u8, velocity: u8) {
    // Simple round-robin voice allocation for v0.1.0
    let voices = core::ptr::addr_of_mut!(VOICES);
    for i in 0..NUM_VOICES {
        let voice = &mut (*voices)[i];
        if !voice.active {
            voice.trigger(note, velocity);
            return;
        }
    }
    // Steal oldest (first in array for now)
    (*voices)[0].trigger(note, velocity);
}

unsafe fn release_voice(note: u8) {
    let voices = core::ptr::addr_of_mut!(VOICES);
    for i in 0..NUM_VOICES {
        let voice = &mut (*voices)[i];
        if voice.active && voice.note == note {
            voice.release();
        }
    }
}

fn setup_i2s2(spi2: &pac::SPI2, _clocks: &stm32f4xx_hal::rcc::Clocks) {
    use stm32f4xx_hal::pac::spi1::i2scfgr::{I2SCFG_A, I2SSTD_A, DATLEN_A};

    // Enable SPI2 clock
    unsafe {
        (*pac::RCC::ptr()).apb1enr.modify(|_, w| w.spi2en().set_bit());
    }

    // Disable I2S before config
    spi2.i2scfgr.modify(|_, w| w.i2se().clear_bit());

    // I2S config: Master Transmit, Philips, 16-bit data, 16-bit channel
    spi2.i2scfgr.modify(|_, w| {
        w.i2smod().set_bit()
         .i2scfg().variant(I2SCFG_A::MasterTx)
         .i2sstd().variant(I2SSTD_A::Philips)
         .ckpol().clear_bit()
         .datlen().variant(DATLEN_A::SixteenBit)
         .chlen().clear_bit()    // 16-bit channel length
    });

    // Prescaler for ~48kHz (exact value depends on I2S clock source)
    // With typical PLLI2S setup, I2SDIV=6, ODD=0 gives close to 48kHz
    spi2.i2spr.modify(|_, w| {
        w.i2sdiv().variant(6)
         .odd().clear_bit()
         .mckoe().clear_bit() // PCM5102A does not need MCK
    });

    // Enable I2S
    spi2.i2scfgr.modify(|_, w| w.i2se().set_bit());
}

fn send_i2s_sample(left: i16, right: i16) {
    let spi2 = unsafe { &*pac::SPI2::ptr() };

    // Wait for TX empty
    while spi2.sr.read().txe().bit_is_clear() {}

    // Write 32-bit frame: right << 16 | left
    let frame = ((right as u32) << 16) | ((left as u32) & 0xFFFF);
    spi2.dr.write(|w| w.dr().variant(frame as u16));

    // For 32-bit write on some variants, we might need to write twice
    // But stm32f4xx typically has 16-bit DR for I2S
}

// === TIM2 Audio ISR (48kHz) ===
#[interrupt]
fn TIM2() {
    unsafe {
        // Generate 440Hz sine wave for verification
        let phase_inc = TEST_FREQ / (SAMPLE_RATE as f32);
        SINE_PHASE += phase_inc;
        if SINE_PHASE >= 1.0 {
            SINE_PHASE -= 1.0;
        }

        let sample_f = libm::sinf(SINE_PHASE * 2.0 * core::f32::consts::PI);
        let sample_i16 = (sample_f * 32767.0) as i16;

        // Send stereo sample to DAC
        send_i2s_sample(sample_i16, sample_i16);

        // Clear TIM2 update flag
        (*pac::TIM2::ptr()).sr.modify(|_, w| w.uif().clear_bit());
    }
}

// === USART2 MIDI ISR ===
#[interrupt]
fn USART2() {
    unsafe {
        let usart2 = &*pac::USART2::ptr();
        let sr = usart2.sr.read();

        if sr.rxne().bit() {
            let byte = usart2.dr.read().bits() as u8;
            let parser = &mut *core::ptr::addr_of_mut!(MIDI_PARSER);
            if let Some(msg) = parser.parse_byte(byte) {
                let _ = MIDI_QUEUE.push(msg);
            }
        }

        // Clear overrun error if present
        if sr.ore().bit() {
            let _ = usart2.dr.read();
            let _ = usart2.sr.read();
        }
    }
}
