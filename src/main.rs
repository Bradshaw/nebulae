//! Render a beautiful Nebulabrot.
//!
//! The Nebulabrot is an alternate way to render the Mandelbrot set
//!
//! Output of calling `nebulae` without arguments:
//! <div style="margin:auto;">
//!     <img style="margin:auto;"
//!         src="https://raw.githubusercontent.com/Bradshaw/nebulae/main/example.png"
//!         alt="A render of a Nebulabrot. A nebulous-looking version of a Mandelbrot fractal."
//!     >
//! </div>
//!
//! # Usage:
//!
//! When installed via `cargo install` as `nebulae`:
//! ```text
//! USAGE:
//!     nebulae [OPTIONS] [SUBCOMMAND]
//!
//! OPTIONS:
//!     -c, --config <CONFIG>     Configuration file
//!     -h, --help                Print help information
//!     -n, --no-intermediates    Do not write intermediate files
//!     -o, --output <OUTPUT>     File to write to [default: image.png]
//!     -V, --version             Print version information
//!
//! SUBCOMMANDS:
//!     help             Print this message or the help of the given subcommand(s)
//!     wizard           Display configuration wizard
//!     write-default    Write the default configuration to TOML
//! ```
//!
//! ## Subcommands:
//!
//! ### `nebulae wizard`
//! ```text
//! Display configuration wizard
//!
//! USAGE:
//!     nebulae wizard [OPTIONS]
//!
//! OPTIONS:
//!     -h, --help                         Print help information
//! ```
//!
//! ### `nebulae write-default`
//! ```text
//! Write the default configuration to TOML
//!
//! USAGE:
//!     nebulae write-default [OPTIONS]
//!
//! OPTIONS:
//!     -h, --help                         Print help information
//!     -s, --save-config <SAVE_CONFIG>    Path to write the default configuration to (writes to stdout
//! ```
//!
//! ## Recipes:
//!
//! * Just render a Nebulabrot with default settings:
//!     * `nebulae`
//! * Render a Nebulabrot using a configuration file:
//!     * `nebulae my_config.toml`
//! * Use the wizard to render a custom Nebulabrot, and save the configuration for future use:
//!     * `nebulae wizard -c my_config.toml`
//! * Render a default Nebulabrot with a custom filename:
//!     * `nebulae -o my_render.png`

use crate::mandelbrot::Complex;
use crate::program_options::ProgramOptions;
use crate::raw_image::RawImage;
use crate::render_settings::*;
use dialoguer::console::Term;
use jitter_sampler::JitterSampler;
use std::cmp::min;
use std::error::Error;
use std::fs::File;
use std::io::BufWriter;
use std::num::NonZeroU32;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use tqdm::tqdm;

mod jitter_sampler;
mod mandelbrot;
mod program_options;
mod raw_image;
mod render_settings;

/// This program is hard-coded to output an RGB-encoded PNG file, so 3 channels are used throughout.
const CHANNELS: u32 = 3;

/// Main function that will hopefully give you a nice picture by the end
fn main() -> Result<(), Box<dyn Error>> {
    let ProgramOptions {
        render_settings,
        output_path,
        render_intermediates,
    } = program_options::get_options()?;

    let threads = match render_settings.threads {
        None => NonZeroU32::try_from(thread::available_parallelism()?)?.get(),
        Some(threads) => threads,
    };

    let intermediate_function = if render_intermediates {
        Some(|data: &Vec<u32>, maximum: u32| {
            write_image(render_settings, &output_path, &data, maximum);
        })
    } else {
        None
    };

    let (data, maximum) = render_nebulabrot(render_settings, threads, &intermediate_function)?;

    write_image(render_settings, &output_path, &data, maximum)
        .join()
        .expect("Problem writing data to file");
    Ok(())
}

/// Render a Nebulabrot on multiple threads
/// Returns a vector of values that represent an RGB-encoded grid of
fn render_nebulabrot<F>(
    settings: RenderSettings,
    threads: u32,
    intermediates: &Option<F>,
) -> Result<(Vec<u32>, u32), Box<dyn Error>>
where
    F: Fn(&Vec<u32>, u32),
{
    let raw_image = RawImage::new(settings.size, settings.size);

    let mutex_image: Arc<Mutex<RawImage>> = Arc::new(Mutex::new(raw_image));

    for _pass in tqdm(1..settings.passes + 1) {
        let mut handles = vec![];

        for _ in 0..threads {
            for channel in 0..CHANNELS {
                let mutex_image = mutex_image.clone();
                let handle = thread::spawn(move || {
                    let mut xys: Vec<(usize, usize)> = Vec::new();
                    let limit = settings.limits[channel as usize];
                    let mut sampler = JitterSampler::new(settings.samples);
                    sampler.shuffle();
                    let progress = tqdm(sampler.enumerate());
                    for (_iteration, (x, y)) in progress {
                        let z = Complex { re: 0.0, im: 0.0 };
                        let c = Complex {
                            re: x * 5.0 - 2.5,
                            im: y * 5.0 - 2.5,
                        };
                        let (zs, bailed) = mandelbrot::iterate(z, c, limit, 2.0, 3.0);
                        if bailed {
                            for z in zs {
                                let x = f64_to_index(z.re, -2.0, 2.0, settings.size);
                                let y = f64_to_index(z.im, -2.0, 2.0, settings.size);
                                match x.zip(y) {
                                    None => {}
                                    Some((x, y)) => {
                                        xys.push((x, y));
                                    }
                                }
                            }
                        }
                    }
                    let mut lock = mutex_image.lock().expect("image is locked");
                    for (x, y) in xys {
                        lock.bump(x as u32, y as u32, channel);
                    }
                });
                handles.push(handle);
            }
        }

        for handle in handles {
            handle.join().unwrap();
        }
        // while let Ok((_thread, channel, zs)) = receive.try_recv() {
        //     for z in tqdm(zs.into_iter()) {
        //         let x = f64_to_index(z.re, -2.0, 2.0, settings.size);
        //         let y = f64_to_index(z.im, -2.0, 2.0, settings.size);
        //         match x.zip(y) {
        //             None => {}
        //             Some((x, y)) => {
        //                 raw_image.bump(x as u32, y as u32, channel);
        //             }
        //         }
        //     }
        // }
        if let Some(intermediates) = intermediates {
            let lock = mutex_image.lock().expect("image is locked");
            intermediates(&lock.get_data(), lock.get_maximum());
        }
    }
    let lock = mutex_image.lock().expect("image is locked");
    Ok((lock.get_data(), lock.get_maximum()))
}

fn write_image(
    settings: RenderSettings,
    output_path: &str,
    data: &Vec<u32>,
    maximum: u32,
) -> JoinHandle<()> {
    let data = data.clone();
    let output_path = String::from(output_path);
    thread::spawn(move || {
        let path = Path::new(output_path.as_str());
        let prep = map_to_color(data, maximum, settings.curve);
        match data_to_png(prep, settings.size as u32, settings.size as u32, path) {
            Ok(_) => {}
            Err(_) => {
                return;
            }
        };
    })
}

fn map_to_color(data: Vec<u32>, maximum: u32, curve: f64) -> Vec<u8> {
    let multiplier = 1.0 / maximum as f64;
    data.into_iter()
        .map(|p| min(255, ((p as f64 * multiplier).powf(curve) * 256.0) as u8))
        .collect()
}

fn data_to_png(
    data: Vec<u8>,
    width: u32,
    height: u32,
    path: &Path,
) -> Result<(), png::EncodingError> {
    let file = File::create(path).unwrap();
    let ref mut w = BufWriter::new(file);
    let mut encoder = png::Encoder::new(w, width as u32, height as u32);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(&data)
}

fn f64_to_index(point: f64, min: f64, max: f64, size: u32) -> Option<usize> {
    if min == max {
        return None;
    };
    let interp = (point - min) / (max - min);
    let pixel = (size as f64 * interp) as usize;
    if interp >= 0.0 && pixel < (size as usize) {
        Some(pixel)
    } else {
        None
    }
}
