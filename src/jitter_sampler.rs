//! Jittered random sampling involves dividing space into equally sized regions, and selecting
//! uniformly from each region.
//! When using random samples for stochastic processes, using jittered sampling spreads out the
//! selected samples and avoids clumping.
//!
//! Other techniques for stochastic processes exist; notably "blue noise" or
//! "poisson-disk sampling", which better guarantees minimum distances between samples. But they can
//! be quite costly to implement.
//! Jittered sampling has fewer guarantees, but is extremely cheap and simple to implement in
//! comparison.

use rand::prelude::ThreadRng;
use rand::{thread_rng, Rng};

/// An iterator for jittered random 2D points over a unit square
pub struct JitterSampler {
    samples: u32,
    count: u32,
    size: u32,
    width: f64,
    height: f64,
    rng: ThreadRng,
}

impl JitterSampler {
    /// Create a JitterSampler that will output `samples` samples in a "jittered" manner.
    /// If `samples` is not a square number, excess samples will be picked over the full unit
    /// square.
    pub fn new(samples: u32) -> JitterSampler {
        let size = squirt(samples);
        JitterSampler {
            samples,
            count: 0,
            size,
            width: 1.0 / (size as f64),
            height: 1.0 / (size as f64),
            rng: thread_rng(),
        }
    }
}

impl Iterator for JitterSampler {
    type Item = (f64, f64);

    fn next(&mut self) -> Option<Self::Item> {
        let item = if self.count < self.size * self.size {
            let x = self.count % self.size;
            let y = self.count / self.size;

            Some((
                self.width * (self.rng.gen::<f64>() + x as f64),
                self.height * (self.rng.gen::<f64>() + y as f64),
            ))
        } else if self.count < self.samples {
            Some((self.rng.gen::<f64>(), self.rng.gen::<f64>()))
        } else {
            None
        };
        self.count += 1;
        item
    }
}

// Integer "square root"
fn squirt(n: u32) -> u32 {
    let sqrt = (n as f64).sqrt() as u32;
    sqrt
}
