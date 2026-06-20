# Voltshark ⚡🦈

Polyphonic synthesizer firmware for STM32F4. Rust + embedded-hal + I2S audio.

## Features

- **16-voice polyphony**: Full ADSR envelopes per voice
- **Dual oscillators**: Detuned for thick sounds
- **Resonant low-pass filter**: With filter envelope modulation
- **MIDI over UART**: Standard 31250 baud MIDI input
- **Effects**: Delay with feedback and mix
- **Real-time control**: CC mapping for cutoff, resonance, modulation
- **Pitch bend**: +/- 2 semitones

## Hardware Requirements

- STM32F407 (or compatible F4 series)
- PCM5102 or similar I2S DAC
- MIDI input circuit (optocoupler + UART)
- Optional: Status LED on PC13

## Pinout

| Function | Pin | Description |
|----------|-----|-------------|
| I2S_WS | PA4 | Word Select (LRCK) |
| I2S_CK | PA5 | Bit Clock |
| I2S_SD | PA7 | Serial Data |
| I2S_MCK | PC6 | Master Clock |
| MIDI_TX | PA2 | UART TX (unused) |
| MIDI_RX | PA3 | UART RX (MIDI input) |
| LED | PC13 | Status LED |

## Building

```bash
# Install dependencies
rustup target add thumbv7em-none-eabihf
cargo install probe-rs-tools

# Build
./scripts/build.sh

# Flash to device
cargo run --target thumbv7em-none-eabihf --release
```

## Architecture

```
voltshark/
├── src/
│   ├── main.rs           # Entry point, hardware init
│   ├── audio/
│   │   └── i2s.rs        # I2S audio engine
│   ├── midi/
│   │   └── parser.rs     # MIDI message parser
│   └── synth/
│       ├── dsp.rs        # Oscillators, filters, effects
│       └── engine.rs     # Polyphonic voice management
├── memory/
│   └── link.x            # Linker script for STM32F4
└── scripts/
    └── build.sh          # Build helper
```

## MIDI Implementation

- **Note On/Off**: Standard MIDI note messages
- **Control Change**: CC 1 (Mod), 7 (Volume), 10 (Pan), 71 (Res), 74 (Cutoff)
- **Pitch Bend**: 14-bit pitch bend messages
- **Channel Pressure**: Aftertouch support

## License

MIT
