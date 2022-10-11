use crate::CHANNELS;
use std::cmp::max;

pub struct RawImage {
    height: u32,
    data: Vec<u32>,
    maximum: u32,
}

impl RawImage {
    pub fn new(width: u32, height: u32) -> RawImage {
        RawImage {
            height,
            data: vec![0; (width * height * CHANNELS) as usize],
            maximum: 0,
        }
    }

    pub fn bump(&mut self, x: u32, y: u32, channel: u32) {
        let index = ((x * self.height + y) * 3 + channel) as usize;
        self.data[index] += 1;
        self.maximum = max(self.maximum, self.data[index]);
    }

    pub fn get_data(&self) -> Vec<u32> {
        self.data.clone()
    }

    pub fn get_maximum(&self) -> u32 {
        self.maximum
    }
}
