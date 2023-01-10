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
use console::style;
use dialoguer::console::Term;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rand::Rng;
use rayon::prelude::*;
use std::cmp::min;
use std::error::Error;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

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

    let intermediate_function = if render_intermediates {
        Some(|data: &Vec<u32>, maximum: u32| {
            write_image(render_settings, &output_path, &data, maximum);
        })
    } else {
        None
    };

    let (data, maximum) = render_nebulabrot(render_settings, &intermediate_function)?;

    write_image(render_settings, &output_path, &data, maximum)
        .join()
        .unwrap();
    Ok(())
}

/// Render a Nebulabrot
/// Returns a vector of values that represent an RGB-encoded grid
fn render_nebulabrot<F>(
    settings: RenderSettings,
    intermediates: &Option<F>,
) -> Result<(Vec<u32>, u32), Box<dyn Error>>
where
    F: Fn(&Vec<u32>, u32),
{
    let template = format!(
        "{{spinner:.reverse}}{{wide_bar}}{}",
        style(" {elapsed:<4} {percent:>4}% ").reverse()
    );
    let m = MultiProgress::new();
    let sty = ProgressStyle::with_template(template.as_str())
        .unwrap()
        .progress_chars("██▉▊▋▌▍▎▏ ");

    let pb = m.add(ProgressBar::new(settings.passes as u64));
    pb.set_style(sty.clone());
    pb.enable_steady_tick(Duration::from_millis(100));

    let raw_image = Arc::new(RawImage::new(settings.size, settings.size));

    let mut last_render = Instant::now();

    for _pass in 0..settings.passes {
        let pb2 = m.insert_after(&pb, ProgressBar::new((CHANNELS * settings.samples) as u64));
        pb2.set_style(sty.clone());
        pb2.enable_steady_tick(Duration::from_millis(100));
        (0..CHANNELS).into_par_iter().for_each(|channel| {
            (0..settings.samples).into_par_iter().for_each(|_| {
                pb2.inc(1);
                let mut rng = rand::thread_rng();
                let limit = settings.limits[channel as usize];
                let z = Complex { re: 0.0, im: 0.0 };
                let c = Complex {
                    re: rng.gen::<f64>() * 5.0 - 2.5,
                    im: rng.gen::<f64>() * 5.0 - 2.5,
                };
                let (zs, bailed) = mandelbrot::iterate(z, c, limit, 2.0, 3.0);
                if bailed {
                    for z in zs {
                        let x = f64_to_index(z.re, -2.0, 2.0, settings.size);
                        let y = f64_to_index(z.im, -2.0, 2.0, settings.size);
                        match x.zip(y) {
                            None => {}
                            Some((x, y)) => {
                                raw_image.bump(x as u32, y as u32, channel);
                            }
                        }
                    }
                }
            });
        });

        pb.inc(1);
        if last_render.elapsed() >= Duration::from_secs(60) {
            if let Some(intermediates) = intermediates {
                intermediates(&raw_image.get_data(), raw_image.get_maximum());
                last_render = Instant::now();
            }
        }
    }
    Ok((raw_image.get_data(), raw_image.get_maximum()))
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
        data_to_png(prep, settings.size as u32, settings.size as u32, path)
            .expect("data to be saved as png");
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
