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
use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};

/// An iterator for jittered random 2D points over a unit square
pub struct JitterSampler {
    samples: u32,
    count: u32,
    count_order: Vec<u32>,
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
            count_order: (0..samples).collect::<Vec<u32>>(),
            size,
            width: 1.0 / (size as f64),
            height: 1.0 / (size as f64),
            rng: thread_rng(),
        }
    }

    pub fn shuffle(&mut self) -> &JitterSampler {
        self.count_order.shuffle(&mut self.rng);
        self
    }

    pub fn index(&self) -> u32 {
        self.count_order[(self.count % self.samples) as usize]
    }
}

impl Iterator for JitterSampler {
    type Item = (f64, f64);

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.index();
        let item = if self.count >= self.samples {
            None
        } else if index < self.size * self.size {
            let x = index % self.size;
            let y = index / self.size;

            Some((
                self.width * (self.rng.gen::<f64>() + x as f64),
                self.height * (self.rng.gen::<f64>() + y as f64),
            ))
        } else {
            Some((self.rng.gen::<f64>(), self.rng.gen::<f64>()))
        };
        self.count += 1;
        item
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.samples as usize, Some(self.samples as usize))
    }
}

impl ExactSizeIterator for JitterSampler {}

// Integer "square root"
fn squirt(n: u32) -> u32 {
    let sqrt = (n as f64).sqrt() as u32;
    sqrt
}
