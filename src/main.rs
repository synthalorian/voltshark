#![no_std]
#![no_main]

use panic_halt as _;
use rp2040_hal as hal;
use rp2040_hal::pac;
use cortex_m_rt::entry;
use hal::clocks::Clock;
use hal::pio::PIOExt;
use hal::timer::Alarm;
use fugit::RateExtU32;
use fugit::ExtU32;
use pio;

mod oscillator;
mod filter;
mod envelope;
mod midi;
mod voice;
mod patch;

use midi::{MidiParser, MidiMessage};
use voice::Voice;
use patch::Patch;

// === Bootloader ===
#[link_section = ".boot2"]
#[used]
pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

// === Audio constants ===
const SAMPLE_RATE: u32 = 48000;
const NUM_VOICES: usize = 8;
const XTAL_FREQ_HZ: u32 = 12_000_000; // 12 MHz crystal on Pico

// === Voice array ===
static mut VOICES: [Voice; NUM_VOICES] = [const { Voice::new() }; NUM_VOICES];
static mut CURRENT_PATCH: Patch = Patch::init();

// === MIDI ring buffer (ISR produces, main consumes) ===
struct RingBuffer<T: Copy, const N: usize> {
    buf: [T; N],
    head: u8,
    tail: u8,
}

impl<T: Copy, const N: usize> RingBuffer<T, N> {
    const fn new(init: T) -> Self {
        RingBuffer {
            buf: [init; N],
            head: 0,
            tail: 0,
        }
    }

    fn push(&mut self, item: T) -> bool {
        let head = self.head;
        let next = (head + 1) % (N as u8);
        if next == self.tail {
            return false; // Full
        }
        self.buf[head as usize] = item;
        self.head = next;
        true
    }

    fn pop(&mut self) -> Option<T> {
        let tail = self.tail;
        if tail == self.head {
            return None; // Empty
        }
        let item = self.buf[tail as usize];
        self.tail = (tail + 1) % (N as u8);
        Some(item)
    }
}

static mut MIDI_QUEUE: RingBuffer<MidiMessage, 32> = RingBuffer::new(
    MidiMessage::NoteOn { channel: 0, note: 0, velocity: 0 }
);

static mut MIDI_PARSER: MidiParser = MidiParser::new();

// === I2S PIO program ===
// Outputs 16-bit stereo I2S for PCM5102A.
// Side-set: bit 0 = BCLK, bit 1 = LRCK
// OUT base = DIN (serial data)
// One 32-bit FIFO word = {left 16, right 16}
fn i2s_pio_program() -> pio::Program<{ pio::RP2040_MAX_PROGRAM_SIZE }> {
    let mut a = pio::Assembler::<{ pio::RP2040_MAX_PROGRAM_SIZE }>::new_with_side_set(
        pio::SideSet::new(false, 2, false)
    );

    let mut left_loop = a.label();
    let mut right_loop = a.label();

    // Left channel: LRCK = 0
    a.set_with_delay_and_side_set(pio::SetDestination::X, 14, 0, 0b01);
    a.bind(&mut left_loop);
    a.out_with_side_set(pio::OutDestination::PINS, 1, 0b00);
    a.jmp_with_side_set(pio::JmpCondition::XDecNonZero, &mut left_loop, 0b01);
    a.out_with_side_set(pio::OutDestination::PINS, 1, 0b00);

    // Right channel: LRCK = 1
    a.set_with_delay_and_side_set(pio::SetDestination::X, 14, 0, 0b11);
    a.bind(&mut right_loop);
    a.out_with_side_set(pio::OutDestination::PINS, 1, 0b10);
    a.jmp_with_side_set(pio::JmpCondition::XDecNonZero, &mut right_loop, 0b11);
    a.out_with_side_set(pio::OutDestination::PINS, 1, 0b10);

    a.assemble_program()
}

#[entry]
fn main() -> ! {
    // === Peripherals ===
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();

    // === Clocks: 125 MHz sysclk from 12 MHz XOSC ===
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);
    let clocks = hal::clocks::init_clocks_and_plls(
        XTAL_FREQ_HZ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    // === GPIO ===
    let sio = hal::Sio::new(pac.SIO);
    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // === UART0 for MIDI (GPIO 0 = TX, GPIO 1 = RX) ===
    let uart_pins = (
        pins.gpio0.into_function::<hal::gpio::FunctionUart>(),
        pins.gpio1.into_function::<hal::gpio::FunctionUart>(),
    );

    let mut uart = hal::uart::UartPeripheral::new(pac.UART0, uart_pins, &mut pac.RESETS)
        .enable(
            hal::uart::UartConfig::new(
                31250.Hz(),
                hal::uart::DataBits::Eight,
                None,
                hal::uart::StopBits::One,
            ),
            clocks.peripheral_clock.freq(),
        )
        .unwrap();

    // Enable RX interrupt
    uart.enable_rx_interrupt();

    // === PIO I2S for PCM5102A DAC ===
    // GPIO 2 = BCLK, GPIO 3 = LRCK, GPIO 4 = DIN
    let _i2s_bclk = pins.gpio2.into_function::<hal::gpio::FunctionPio0>();
    let _i2s_lrck = pins.gpio3.into_function::<hal::gpio::FunctionPio0>();
    let _i2s_din  = pins.gpio4.into_function::<hal::gpio::FunctionPio0>();

    let (mut pio, sm0, _, _, _) = pac.PIO0.split(&mut pac.RESETS);

    let program = i2s_pio_program();
    let installed = pio.install(&program).unwrap();

    let sys_clk_hz = clocks.system_clock.freq().to_Hz();
    // 64 PIO cycles per sample frame; desired 48 kHz
    // divisor = sys_clk / (64 * 48000)
    let div_int = sys_clk_hz / (64 * SAMPLE_RATE);
    let div_frac = ((sys_clk_hz % (64 * SAMPLE_RATE)) * 256) / (64 * SAMPLE_RATE);

    let (sm, _, tx) = hal::pio::PIOBuilder::from_installed_program(installed)
        .out_pins(4, 1)          // DIN = GPIO 4
        .side_set_pin_base(2)    // BCLK = GPIO 2, LRCK = GPIO 3
        .autopull(true)
        .pull_threshold(32)
        .buffers(hal::pio::Buffers::OnlyTx)
        .clock_divisor_fixed_point(div_int as u16, div_frac as u8)
        .build(sm0);

    sm.start();

    // Store TX FIFO handle for ISR
    unsafe {
        I2S_TX = Some(tx);
    }

    // === Timer alarm 0: 48 kHz audio ISR ===
    let mut timer = hal::timer::Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);
    let mut alarm = timer.alarm_0().unwrap();
    alarm.schedule(21.micros()).unwrap(); // ~47.6 kHz (closest integer µs)
    alarm.enable_interrupt();

    unsafe {
        pac::NVIC::unmask(pac::Interrupt::TIMER_IRQ_0);
        pac::NVIC::unmask(pac::Interrupt::UART0_IRQ);
    }

    // === Main loop ===
    loop {
        // Process MIDI messages
        while let Some(msg) = unsafe { MIDI_QUEUE.pop() } {
            handle_midi(msg);
        }

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

// === I2S PIO TX FIFO handle (ISR writes here) ===
static mut I2S_TX: Option<hal::pio::Tx<(pac::PIO0, hal::pio::SM0)>> = None;

// === TIM2 Audio ISR (48kHz) ===
#[allow(non_snake_case)]
#[no_mangle]
fn TIMER_IRQ_0() {
    unsafe {
        // Mix active voices
        let mut mix: i32 = 0;
        let voices = core::ptr::addr_of_mut!(VOICES);
        for i in 0..NUM_VOICES {
            let voice = &mut (*voices)[i];
            if voice.active {
                mix += voice.next_sample() as i32;
            }
        }

        let sample_i16 = mix.clamp(-32768, 32767) as i16;
        let stereo_word = ((sample_i16 as u32) << 16) | (sample_i16 as u32 & 0xFFFF);

        if let Some(ref mut tx) = I2S_TX {
            tx.write(stereo_word);
        }

        // Clear timer alarm 0 interrupt (write 1 to clear)
        (*pac::TIMER::ptr()).intr().modify(|_, w| w.alarm_0().bit(true));

        // Re-arm alarm for next sample (21 µs ≈ 47.6 kHz)
        let now = (*pac::TIMER::ptr()).timerawl().read().bits();
        (*pac::TIMER::ptr()).alarm0().write(|w| w.bits(now + 21));
    }
}

// === USART2 MIDI ISR ===
#[allow(non_snake_case)]
#[no_mangle]
fn UART0_IRQ() {
    unsafe {
        let uart0 = &*pac::UART0::ptr();

        // Check RX FIFO not empty
        if uart0.uartfr().read().rxfe().bit_is_clear() {
            let byte = uart0.uartdr().read().data().bits() as u8;
            let parser = &mut *core::ptr::addr_of_mut!(MIDI_PARSER);
            if let Some(msg) = parser.parse_byte(byte) {
                let _ = MIDI_QUEUE.push(msg);
            }
        }

        // Clear any overrun error by reading flags + data
        if uart0.uartrsr().read().oe().bit() {
            let _ = uart0.uartdr().read();
        }
    }
}
