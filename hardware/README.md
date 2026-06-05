# ⚡ VoltShark Hardware

## Overview

Open-source polyphonic synthesizer. Build it yourself. The shark swims in circuits.

## Specifications

## Specifications

- **Voices**: 8-note polyphony
- **Oscillators**: Digital (sine, triangle, saw, square, pulse)
- **Filter**: 24dB/octave resonant low-pass (analog ladder)
- **Envelope**: ADSR per voice
- **MIDI**: In/Out/Thru (DIN + USB)
- **Audio**: 48kHz, 16-bit stereo DAC
- **Display**: 128x64 OLED
- **Controls**: 8 encoders + 16 buttons

## Bill of Materials

### MCU
| Part | Qty | Price | Source |
|------|-----|-------|--------|
| STM32F407VGT6 | 1 | $8 | Mouser |
| OR RP2040 (Raspberry Pi Pico) | 1 | $4 | Adafruit |

### Audio
| Part | Qty | Price | Source |
|------|-----|-------|--------|
| PCM5102A DAC | 1 | $3 | AliExpress |
| TL074 Op-Amp | 2 | $1 | Mouser |
| 3.5mm TRS jacks | 3 | $2 | Mouser |

### Filter (Analog)
| Part | Qty | Price | Source |
|------|-----|-------|--------|
| 2N3904 NPN | 8 | $0.50 | Mouser |
| 2N3906 PNP | 8 | $0.50 | Mouser |
| 10nF film caps | 16 | $2 | Mouser |
| 100k pots (alpha) | 4 | $4 | Thonk |

### Power
| Part | Qty | Price | Source |
|------|-----|-------|--------|
| LM7805 | 1 | $0.50 | Mouser |
| LM317 | 1 | $0.50 | Mouser |
| 12V DC jack | 1 | $1 | Mouser |

### UI
| Part | Qty | Price | Source |
|------|-----|-------|--------|
| 128x64 OLED (I2C) | 1 | $5 | Adafruit |
| Rotary encoders | 8 | $8 | Adafruit |
| Tactile buttons | 16 | $2 | Adafruit |

**Total: ~$40-50**

## Schematics

See `schematics/` directory for KiCad files.

## PCB

- 2-layer board, 100x80mm
- Through-hole where possible for DIY
- SMT for DAC and MCU (can use breakout)

## Enclosure

- Eurorack compatible (20HP)
- OR standalone desktop case (3D printable files included)

## Build Guide

1. Solder power section, test voltages
2. Solder MCU + crystal
3. Solder MIDI circuits
4. Solder DAC + audio output
5. Solder filter section (analog)
6. Solder UI board (separate PCB)
7. Flash firmware
8. Calibrate filter cutoff range
9. Make noise

## License

Hardware: CERN-OHL-S-2.0
Firmware: MIT
