use stm32f4xx_hal as hal;
use hal::{
    i2s::{I2s, I2sStandard, I2sDataFormat, I2sFreqConfig},
    pac::SPI1,
    rcc::Clocks,
    time::Hertz,
};
use cortex_m::peripheral::NVIC;

/// I2S audio engine for PCM output
pub struct I2SAudioEngine {
    i2s: I2s<SPI1>,
    sample_rate: u32,
    buffer: [i16; 256],
    write_index: usize,
    read_index: usize,
}

impl I2SAudioEngine {
    pub fn new(i2s: I2s<SPI1>, sample_rate: Hertz, clocks: &Clocks) -> Self {
        // Configure I2S for Philips standard, 16-bit data
        let mut engine = Self {
            i2s,
            sample_rate: sample_rate.raw(),
            buffer: [0; 256],
            write_index: 0,
            read_index: 0,
        };
        
        engine.init_i2s(clocks);
        engine
    }
    
    fn init_i2s(&mut self, clocks: &Clocks) {
        // Configure I2S peripheral
        // In real implementation, this would set up DMA for continuous playback
        // For now, we set up the basic I2S configuration
        
        let freq_config = I2sFreqConfig::from_clocks(
            clocks,
            self.sample_rate,
            I2sStandard::Philips,
            I2sDataFormat::Data16Channel32,
        );
        
        // Enable I2S
        self.i2s.enable();
    }
    
    pub fn needs_samples(&self) -> bool {
        // Check if we need more audio samples
        let available = (self.write_index + self.buffer.len() - self.read_index) % self.buffer.len();
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
        // Start I2S DMA transfer
        // In real implementation, this would trigger continuous DMA
        self.i2s.enable();
    }
    
    pub fn stop(&mut self) {
        self.i2s.disable();
    }
}

/// Simple software I2S output for testing without hardware
pub struct SoftwareI2S {
    sample_rate: u32,
    buffer: [i16; 512],
    write_pos: usize,
}

impl SoftwareI2S {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
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
