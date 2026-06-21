#![no_std]
#![no_main]
#![allow(dead_code)]

use cortex_m_rt::entry;
use panic_halt as _;
use stm32f4xx_hal as hal;

use hal::{
    gpio::GpioExt,
    i2s::I2s,
    pac,
    prelude::*,
    rcc::RccExt,
    serial::{config::Config, Serial},
    time::Hertz,
};

mod audio;
mod board;
mod midi;
mod synth;

use audio::i2s::I2SAudioEngine;
use midi::parser::MidiEvent;
use synth::engine::SynthEngine;

#[entry]
fn main() -> ! {
    // Get peripherals
    let dp = pac::Peripherals::take().unwrap();
    let _cp = cortex_m::Peripherals::take().unwrap();

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

    // I2S pins for audio output (PCM5102 or similar DAC) on SPI2
    // PB12 - I2S_WS (LRCK)  (AF5 for SPI2)
    // PB13 - I2S_CK (BCLK)  (AF5 for SPI2)
    // PB15 - I2S_SD (DATA)  (AF5 for SPI2)
    // PC6  - I2S_MCK (Master Clock) (AF5 for SPI2)
    let i2s_ws = gpiob.pb12.into_alternate::<5>();
    let i2s_ck = gpiob.pb13.into_alternate::<5>();
    let i2s_mck = gpioc.pc6.into_alternate::<5>();
    let i2s_sd = gpiob.pb15.into_alternate::<5>();

    // Initialize I2S audio at 48kHz using SPI2
    // I2s::new takes (ws, ck, mck, sd) in that order per HAL signature.
    let i2s = I2s::new(dp.SPI2, (i2s_ws, i2s_ck, i2s_mck, i2s_sd), &clocks);
    let mut audio_engine = I2SAudioEngine::new(i2s, Hertz::Hz(48000));

    // UART for MIDI input (USART2: PA2=TX, PA3=RX)
    let midi_tx = gpioa.pa2.into_alternate();
    let midi_rx = gpioa.pa3.into_alternate();
    let midi_serial = Serial::new(
        dp.USART2,
        (midi_tx, midi_rx),
        Config::default().baudrate(31250.bps()), // Standard MIDI baud rate
        &clocks,
    )
    .unwrap();

    let mut midi_parser = midi::parser::MidiParser::new(midi_serial);

    // Initialize synth engine with Bass patch
    let mut synth = SynthEngine::new(48000);
    synth.set_patch(0); // Bass patch

    // LED for status (PC13 on most STM32F4 dev boards)
    let mut led = gpioc.pc13.into_push_pull_output();
    let mut patch_change_timer = 0u32;
    let mut current_patch = 0usize;

    // Main loop
    let mut last_tick = 0u32;
    loop {
        // Process MIDI messages
        while let Some(event) = midi_parser.poll() {
            match event {
                MidiEvent::NoteOn {
                    channel,
                    note,
                    velocity,
                } => {
                    synth.note_on(channel, note, velocity);
                }
                MidiEvent::NoteOff { channel, note, .. } => {
                    synth.note_off(channel, note);
                }
                MidiEvent::ControlChange {
                    channel,
                    controller,
                    value,
                } => {
                    synth.control_change(channel, controller, value);
                }
                MidiEvent::PitchBend { channel, value } => {
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
            patch_change_timer += 1;
            // Cycle through patches every ~10 seconds (at ~2Hz blink)
            if patch_change_timer >= 20 {
                patch_change_timer = 0;
                current_patch = (current_patch + 1) % 5;
                synth.set_patch(current_patch);
                let _name = synth.get_patch_name(); // Could log to debug if available
            }
        }
    }
}
