use stm32f4xx_hal as hal;
use hal::{
    i2s::I2s,
    pac::SPI2,
    time::Hertz,
};

/// I2S audio engine for PCM output
pub struct I2SAudioEngine {
    i2s: I2s<SPI2>,
    sample_rate: u32,
    buffer: [i16; 256],
    write_index: usize,
    read_index: usize,
}

impl I2SAudioEngine {
    pub fn new(i2s: I2s<SPI2>, sample_rate: Hertz) -> Self {
        let mut engine = Self {
            i2s,
            sample_rate: sample_rate.raw(),
            buffer: [0; 256],
            write_index: 0,
            read_index: 0,
        };
        engine.start();
        engine
    }

    pub fn needs_samples(&self) -> bool {
        // Check if we need more audio samples
        let available =
            (self.write_index + self.buffer.len() - self.read_index) % self.buffer.len();
        available < 64 // Need more when buffer is running low
    }

    pub fn push_sample(&mut self, sample: (f32, f32)) {
        // Convert f32 stereo to i16 and push to buffer
        let left = (sample.0 * 32767.0) as i16;
        let right = (sample.1 * 32767.0) as i16;

        self.buffer[self.write_index] = left;
        self.write_index = (self.write_index + 1) % self.buffer.len();

        self.buffer[self.write_index] = right;
        self.write_index = (self.write_index + 1) % self.buffer.len();
    }

    pub fn start(&mut self) {
        // In a real implementation, this would trigger DMA.
        // The I2s wrapper from stm32f4xx-hal v0.21 doesn't expose enable/disable directly.
    }

    pub fn stop(&mut self) {
        // Placeholder for stopping DMA.
    }
}

/// Simple software I2S output for testing without hardware
#[allow(dead_code)]
pub struct SoftwareI2S {
    _sample_rate: u32,
    buffer: [i16; 512],
    write_pos: usize,
}

#[allow(dead_code)]
impl SoftwareI2S {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            _sample_rate: sample_rate,
            buffer: [0; 512],
            write_pos: 0,
        }
    }

    pub fn push_sample(&mut self, sample: (f32, f32)) {
        let left = (sample.0 * 32767.0) as i16;
        let right = (sample.1 * 32767.0) as i16;

        if self.write_pos < self.buffer.len() - 1 {
            self.buffer[self.write_pos] = left;
            self.buffer[self.write_pos + 1] = right;
            self.write_pos += 2;
        }
    }

    pub fn clear(&mut self) {
        self.write_pos = 0;
        self.buffer = [0; 512];
    }

    pub fn get_buffer(&self) -> &[i16] {
        &self.buffer[..self.write_pos]
    }
}
