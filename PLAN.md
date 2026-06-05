# VoltShark — Development Plan

Polyphonic synthesizer firmware. Rust `no_std`. STM32 or RP2040. The shark runs on voltage.

---

## v0.1.0 — DSP Core (Now)

- [ ] Pick target: STM32F407 (power) or RP2040 (cost, dual-core)
- [ ] Add HAL crate: `stm32f4xx-hal` or `rp2040-hal`
- [ ] Implement audio ISR: timer-triggered at 48kHz
- [ ] Wire DAC: SPI/I2S for PCM5102A
- [ ] Verify sine wave output on scope/ears
- [ ] MIDI UART: receive NoteOn/Off, trigger voices

## v0.2.0 — Voice

- [ ] Full voice allocation (8-note polyphony)
- [ ] Voice stealing (oldest note)
- [ ] All 5 waveforms: sine, triangle, saw, square, pulse
- [ ] Filter cutoff via ADC (potentiometer)
- [ ] Filter resonance via ADC
- [ ] ADSR envelope per voice

## v0.3.0 — Patch + UI

- [ ] 128 patch memory in flash
- [ ] Save/load patches via MIDI SysEx
- [ ] OLED display: patch name, parameters
- [ ] Rotary encoders for parameter editing
- [ ] Menu system

## v1.0.0 — Hardware

- [ ] Design PCB (KiCad)
- [ ] Order PCB + components
- [ ] Solder prototype
- [ ] Calibrate filter range
- [ ] Build enclosure (Eurorack or desktop)
- [ ] Document BOM, schematics, build guide

---

## Architecture

```
MIDI UART → MIDI Parser → Voice Allocator → 8× Voice
                                               ↓
Audio ISR ← Mixer ← OSC → Filter → Envelope ←┘
   ↓
DAC (PCM5102A) → Audio Out
```

## Key Files

| File | Responsibility |
|------|---------------|
| `src/main.rs` | Init, main loop, hardware setup |
| `src/oscillator.rs` | Waveform generation |
| `src/filter.rs` | 24dB resonant low-pass |
| `src/envelope.rs` | ADSR |
| `src/voice.rs` | Voice management |
| `src/midi.rs` | MIDI message parsing |
| `src/patch.rs` | Patch memory |

## Hardware

| Component | Part | Cost |
|-----------|------|------|
| MCU | STM32F407VGT6 or RP2040 | $4-8 |
| DAC | PCM5102A | $3 |
| Filter | 2N3904/2N3906 + passives | $5 |
| Display | 128x64 OLED | $5 |
| Controls | 8 encoders + 16 buttons | $10 |
| Power | 12V DC, LM7805/LM317 | $2 |
| **Total** | | **~$40-50** |

## Local Dev

```bash
# For host testing (no hardware):
cargo check

# For target (install target first):
rustup target add thumbv7em-none-eabihf
cargo check --target thumbv7em-none-eabihf
```

## Flashing

```bash
# STM32 via probe-rs:
probe-rs download --chip STM32F407VG target/thumbv7em-none-eabihf/release/voltshark

# RP2040 via UF2:
# Copy target/thumbv6m-none-eabi/release/voltshark.uf2 to mounted Pico
```

---

*The shark swims in circuits.* ⚡
