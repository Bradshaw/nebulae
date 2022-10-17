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
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use jitter_sampler::JitterSampler;
use std::cmp::min;
use std::error::Error;
use std::fs::File;
use std::io::BufWriter;
use std::num::NonZeroU32;
use std::path::Path;
use std::sync::{mpsc, Arc};
use std::thread;
use std::thread::JoinHandle;

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

    println!("Rendering with settings:\n{render_settings}");

    let progress_bar_style = ProgressStyle::with_template(
        "{spinner:.black.on_blue.bold}{wide_bar:.blue/white} [eta:{eta_precise}] {msg:>40.white.bold}",
    )
    .unwrap();

    let progress_bars = Arc::new(MultiProgress::new());
    let intermediate_function = if render_intermediates {
        Some(|data: &Vec<u32>, maximum: u32| {
            write_image(
                render_settings,
                &output_path,
                &data,
                maximum,
                progress_bar_style.clone(),
                Some(&progress_bars),
            );
        })
    } else {
        None
    };

    let (data, maximum) = render_nebulabrot(
        render_settings,
        threads,
        &intermediate_function,
        Some(&progress_bars),
    )?;

    write_image(
        render_settings,
        &output_path,
        &data,
        maximum,
        progress_bar_style.clone(),
        None,
    )
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
    progress_bars: Option<&Arc<MultiProgress>>,
) -> Result<(Vec<u32>, u32), Box<dyn Error>>
where
    F: Fn(&Vec<u32>, u32),
{
    let progress_bar_style = ProgressStyle::with_template(
        "{spinner:.black.on_blue.bold}{wide_bar:.blue/white} [eta:{eta_precise}] {msg:>40.white.bold}",
    ).unwrap();

    let progress_bars = match progress_bars {
        None => Arc::new(MultiProgress::new()),
        Some(progress_bars) => Arc::clone(&progress_bars),
    };

    let mut raw_image = RawImage::new(settings.size, settings.size);
    let main_progress = progress_bars.add(ProgressBar::new(settings.passes.into()));
    main_progress.set_style(progress_bar_style.clone());
    main_progress.set_message(format!("Rendering..."));
    main_progress.tick();

    let mut pass = 0;
    while pass < settings.passes {
        main_progress.inc(1);
        pass += 1;
        main_progress.set_message(format!(
            "Rendering pass {pass}/{} on {} threads",
            settings.passes, threads
        ));
        let mut handles = vec![];
        let (transfer, receive) = mpsc::channel();

        for thread in 0..threads {
            let thread_progress =
                progress_bars.add(ProgressBar::new((settings.samples * CHANNELS) as u64));
            thread_progress.set_style(progress_bar_style.clone());
            thread_progress.set_message(format!("Thread {thread}"));
            let transfer = transfer.clone();
            let handle = thread::spawn(move || {
                for channel in 0..CHANNELS {
                    thread_progress.set_message(format!("Thread {thread} - Channel {channel}"));
                    thread_progress.tick();
                    let mut zss: Vec<Complex> = Vec::new();
                    let limit = settings.limits[channel as usize];
                    let sampler = JitterSampler::new(settings.samples);
                    for (iteration, x, y) in sampler {
                        let z = Complex { re: 0.0, im: 0.0 };
                        let c = Complex {
                            re: x * 5.0 - 2.5,
                            im: y * 5.0 - 2.5,
                        };
                        let (zs, bailed) = mandelbrot::iterate(z, c, limit, 2.0, 3.0);
                        if bailed {
                            zss.extend(zs);
                        }
                        if iteration % (settings.samples / 100) == 0 {
                            thread_progress.inc((settings.samples / 100) as u64);
                        }
                    }
                    transfer.send((thread, channel, zss)).unwrap();
                }
                thread_progress.finish_and_clear();
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }
        let channel_progress =
            progress_bars.add(ProgressBar::new((threads as u32 * CHANNELS) as u64));
        channel_progress.set_message(format!("Gathering..."));
        channel_progress.set_style(progress_bar_style.clone());
        channel_progress.tick();
        while let Ok((thread, channel, zs)) = receive.try_recv() {
            channel_progress
                .set_message(format!("Gathering channel {channel} from thread {thread}"));
            channel_progress.tick();
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
            channel_progress.inc(1);
        }
        channel_progress.finish_and_clear();
        if let Some(intermediates) = intermediates {
            intermediates(&raw_image.get_data(), raw_image.get_maximum());
        }
    }
    main_progress.finish();

    progress_bars.clear()?;
    Ok((raw_image.get_data(), raw_image.get_maximum()))
}

fn write_image(
    settings: RenderSettings,
    output_path: &str,
    data: &Vec<u32>,
    maximum: u32,
    progress_bar_style: ProgressStyle,
    progress_bars: Option<&Arc<MultiProgress>>,
) -> JoinHandle<()> {
    let image_progress = match progress_bars {
        None => ProgressBar::new(3),
        Some(progress_bars) => progress_bars.add(ProgressBar::new(3)),
    };
    image_progress.set_message("Cloning data");
    image_progress.set_style(progress_bar_style.clone());
    image_progress.tick();
    let data = data.clone();
    image_progress.inc(1);
    image_progress.set_message("Preparing image values");
    image_progress.tick();
    let output_path = String::from(output_path);
    thread::spawn(move || {
        let path = Path::new(output_path.as_str());
        let prep = map_to_color(data, maximum, settings.curve);
        image_progress.inc(1);
        image_progress.set_message("Writing file");
        image_progress.tick();
        match data_to_png(prep, settings.size as u32, settings.size as u32, path) {
            Ok(_) => {}
            Err(_) => {
                return;
            }
        };
        image_progress.inc(1);
        image_progress.finish_and_clear();
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
