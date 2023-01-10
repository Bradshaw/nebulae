use crate::CHANNELS;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering::SeqCst;

/// A structure to hold unscaled, integer "photo-counting" style images.
pub struct RawImage {
    height: u32,
    data: Vec<AtomicU32>,
    maximum: AtomicU32,
}

impl RawImage {
    /// Construct a new [`RawImage`] with a given width and height, initialized to 0
    pub fn new(width: u32, height: u32) -> RawImage {
        RawImage {
            height,
            data: vec![(); (width * height * CHANNELS) as usize].iter().map(|_| AtomicU32::new(0)).collect(),
            maximum: AtomicU32::new(0),
        }
    }

    /// Increment the value of a given `channel` at `x` - `y` coordinates
    pub fn bump(&self, x: u32, y: u32, channel: u32) {
        let index = ((x * self.height + y) * 3 + channel) as usize;
        let new_value = self.data[index].fetch_add(1, SeqCst);
        self.maximum.fetch_max(new_value, SeqCst);
    }

    /// Get a copy of the internal data
    pub fn get_data(&self) -> Vec<u32> {
        self.data.iter().map(|a| a.load(SeqCst)).collect()
    }

    /// Get the maximum value (brightest pixel)
    pub fn get_maximum(&self) -> u32 {
        self.maximum.load(SeqCst)
    }
}
