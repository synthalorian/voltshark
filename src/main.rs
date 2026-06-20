#![no_std]
#![no_main]

use cortex_m_rt::entry;
use panic_halt as _;
use stm32f4xx_hal as hal;

use hal::{
    gpio::{GpioExt, Output, PushPull, AF5},
    i2s::I2s,
    pac,
    prelude::*,
    rcc::RccExt,
    serial::{config::Config, Serial},
    time::Hertz,
};

mod audio;
mod synth;
mod midi;
mod board;

use audio::i2s::I2SAudioEngine;
use synth::engine::SynthEngine;
use midi::parser::MidiParser;

#[entry]
fn main() -> ! {
    // Get peripherals
    let dp = pac::Peripherals::take().unwrap();
    let cp = cortex_m::Peripherals::take().unwrap();
    
    // Configure clocks
    let rcc = dp.RCC.constrain();
    let clocks = rcc
        .cfgr
        .use_hse(Hertz::MHz(8))
        .sysclk(Hertz::MHz(168))
        .hclk(Hertz::MHz(168))
        .pclk1(Hertz::MHz(42))
        .pclk2(Hertz::MHz(84))
        .i2s_clk(Hertz::MHz(96))
        .freeze();
    
    // Configure GPIO
    let gpioa = dp.GPIOA.split();
    let gpiob = dp.GPIOB.split();
    let gpioc = dp.GPIOC.split();
    
    // I2S pins for audio output (PCM5102 or similar DAC)
    // PA4 - I2S_WS (LRCK)
    // PA5 - I2S_CK (BCLK)
    // PA7 - I2S_SD (DATA)
    // PC6 - I2S_MCK (Master Clock)
    let i2s_ws = gpioa.pa4.into_alternate::<5>();
    let i2s_ck = gpioa.pa5.into_alternate::<5>();
    let i2s_sd = gpioa.pa7.into_alternate::<5>();
    let i2s_mck = gpioc.pc6.into_alternate::<5>();
    
    // Initialize I2S audio at 48kHz
    let i2s = I2s::new(dp.SPI1, (i2s_ck, i2s_ws, i2s_sd, i2s_mck));
    let mut audio_engine = I2SAudioEngine::new(i2s, Hertz::Hz(48000), &clocks);
    
    // UART for MIDI input (USART2: PA2=TX, PA3=RX)
    let midi_tx = gpioa.pa2.into_alternate();
    let midi_rx = gpioa.pa3.into_alternate();
    let midi_serial = Serial::new(
        dp.USART2,
        (midi_tx, midi_rx),
        Config::default().baudrate(31250.Hz()), // Standard MIDI baud rate
        &clocks,
    ).unwrap();
    
    let mut midi_parser = MidiParser::new(midi_serial);
    
    // Initialize synth engine
    let mut synth = SynthEngine::new(48000);
    
    // LED for status (PC13 on most STM32F4 dev boards)
    let mut led = gpioc.pc13.into_push_pull_output();
    
    // Main loop
    let mut last_tick = 0u32;
    loop {
        // Process MIDI messages
        while let Some(event) = midi_parser.poll() {
            match event {
                midi::MidiEvent::NoteOn { channel, note, velocity } => {
                    synth.note_on(channel, note, velocity);
                }
                midi::MidiEvent::NoteOff { channel, note, .. } => {
                    synth.note_off(channel, note);
                }
                midi::MidiEvent::ControlChange { channel, controller, value } => {
                    synth.control_change(channel, controller, value);
                }
                midi::MidiEvent::PitchBend { channel, value } => {
                    synth.pitch_bend(channel, value);
                }
                _ => {}
            }
        }
        
        // Generate audio if buffer needs more samples
        if audio_engine.needs_samples() {
            let sample = synth.render();
            audio_engine.push_sample(sample);
        }
        
        // Blink LED to show we're alive
        let tick = cortex_m::peripheral::SYST::get_current();
        if tick.wrapping_sub(last_tick) > 16_000_000 {
            led.toggle();
            last_tick = tick;
        }
    }
}
