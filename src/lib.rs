//! Voltshark — polyphonic synthesizer core for STM32F4.
//!
//! This library crate contains the pure, hardware-independent synthesis logic:
//! MIDI parsing, DSP building blocks, and the polyphonic voice engine.
//! It is `no_std` so it can be linked into the firmware binary, while remaining
//! fully testable on the host with `cargo test`.

#![no_std]
#![allow(dead_code)]

#[cfg(test)]
extern crate std;

pub mod midi;
pub mod synth;
