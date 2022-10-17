use crate::CHANNELS;
use std::cmp::max;

/// A structure to hold unscaled, integer "photo-counting" style images.
pub struct RawImage {
    height: u32,
    data: Vec<u32>,
    maximum: u32,
}

impl RawImage {
    /// Construct a new [`RawImage`] with a given width and height, initialized to 0
    pub fn new(width: u32, height: u32) -> RawImage {
        RawImage {
            height,
            data: vec![0; (width * height * CHANNELS) as usize],
            maximum: 0,
        }
    }

    /// Increment the value of a given `channel` at `x` - `y` coordinates
    pub fn bump(&mut self, x: u32, y: u32, channel: u32) -> Option<u32> {
        let index = ((x * self.height + y) * 3 + channel) as usize;
        self.data[index] += 1;
        self.maximum = max(self.maximum, self.data[index]);
        Some(self.data[index])
    }

    /// Get a copy of the internal data
    pub fn get_data(&self) -> Vec<u32> {
        self.data.clone()
    }

    /// Get the maximum value (brightest pixel)
    pub fn get_maximum(&self) -> u32 {
        self.maximum
    }
}
