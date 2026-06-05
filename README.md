# ⚡ VoltShark

Polyphonic synthesizer firmware for STM32/RP2040. Digital oscillators, analog filter control, MIDI, patch memory.

The shark runs on voltage. 8 voices of analog-modeled synthesis in a firmware you can flash to a $4 microcontroller.

## Features

- **8-voice polyphony** — Full MIDI voice allocation
- **5 waveforms** — Sine, triangle, saw, square, variable pulse
- **24dB resonant filter** — Moog ladder approximation
- **ADSR envelope** — Per-voice
- **Patch memory** — 128 patches stored in flash
- **MIDI** — Note on/off, CC, pitch bend, program change
- **Zero dependencies** — Bare metal, no HAL bloat

## Building

### STM32
```bash
cargo build --target thumbv7em-none-eabihf --release
# Flash with probe-rs or OpenOCD
probe-rs download --chip STM32F407VG target/thumbv7em-none-eabihf/release/synth-firmware
```

### RP2040
```bash
cargo build --target thumbv6m-none-eabi --release
# Copy UF2 to mounted Pico
```

## Architecture

```
┌─────────────────────────────────────────┐
│           Audio ISR (48kHz)             │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐ │
│  │ Voice 1 │  │ Voice 2 │  │ Voice...│ │
│  │ OSC→FIL │  │ OSC→FIL │  │ OSC→FIL │ │
│  │ →ENV    │  │ →ENV    │  │ →ENV    │ │
│  └────┬────┘  └────┬────┘  └────┬────┘ │
│       └─────────────┴─────────────┘     │
│                    │                     │
│              ┌─────┴─────┐               │
│              │   DAC     │               │
│              │  PCM5102A │               │
│              └───────────┘               │
└─────────────────────────────────────────┘
┌─────────────────────────────────────────┐
│           Main Loop                     │
│  MIDI parse → Voice alloc → Param update│
│  Display → Encoder scan → Patch I/O     │
└─────────────────────────────────────────┘
```

## Voice Architecture

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│ Oscillator  │────▶│   Filter    │────▶│  Envelope   │
│ (Digital)   │     │ (Analog CV) │     │  (Digital)  │
└─────────────┘     └─────────────┘     └──────┬──────┘
                                               │
                                        ┌──────┴──────┐
                                        │   Output    │
                                        └─────────────┘
```

## Hardware

See `hardware/` for full schematics, BOM, and build guide.

## License

MIT — Build your own sound. 🎹🦈
