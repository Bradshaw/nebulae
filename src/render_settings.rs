//! Utility for rendering settings

use crate::{Term, CHANNELS};
use dialoguer::theme::ColorfulTheme;
use dialoguer::{Confirm, Select};
use serde::{Deserialize, Serialize};
use std::io::Error;
use std::num::NonZeroU32;
use std::{fmt, fs, thread};

/// Configuration Settings for the main function
#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct RenderSettings {
    /// Iteration multiplier for each of the red, green, and blue channels
    /// Must be an array of CHANNELS integers
    pub limits: [u32; CHANNELS as usize],
    /// Number of random samples to take, per channel, per pass
    pub samples: u32,
    #[serde(skip)]
    pub threads: Option<u32>,
    /// Number of passes to run
    pub passes: u16,
    /// Resolution of the rendered image (size × size pixels)
    pub size: u32,
    /// Colour correction curve to apply (value between 0 and 1, raised to this power)
    pub curve: f64,
}

/// Default settings (Equivalent to selecting the default values in the configuration wizard)
pub const DEFAULT_RENDER_SETTINGS: RenderSettings = RenderSettings {
    limits: [7_740, 2_580, 860],
    size: 1 << 11,
    samples: 1_000_000,
    threads: None,
    passes: 100,
    curve: 0.5,
};

impl fmt::Display for RenderSettings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Escape limits:\t{},{},{}\nRuns per pass:\t{}\nPasses:\t\t{}\n{}Resolution:\t{}x{}\nCorrection\t{}",
            self.limits[0],
            self.limits[1],
            self.limits[2],
            self.samples,
            self.passes,
            match self.threads {
                None => String::from(""),
                Some(threads) => {
                    format!("Threads:\t\t{threads}\n")
                }
            },
            self.size,
            self.size,
            self.curve,
        )
    }
}

impl RenderSettings {
    /// Serializes and writes the configuration in TOML format to a file
    pub fn to_file(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        fs::write(path, self.serialize()?)?;
        Ok(())
    }

    /// Serializes the configuration to TOML
    pub fn serialize(&self) -> Result<String, Box<dyn std::error::Error>> {
        Ok(toml::to_string_pretty(self)?)
    }

    /// Opens a TOML file to a [`RenderSettings`]
    pub fn from_file(path: &str) -> Result<RenderSettings, Box<dyn std::error::Error>> {
        let data: String = fs::read_to_string(path)?;
        Ok(toml::from_str(data.as_str())?)
    }

    /// Generates a [`RenderSettings`] from a TUI in the terminal
    pub fn from_wizard() -> Result<Option<RenderSettings>, Box<dyn std::error::Error>> {
        let threads = NonZeroU32::try_from(thread::available_parallelism()?)?.get();

        let color_palette = match select(
            "Palette",
            vec![
                ("Nebulous", &[2, 1, 0]),
                ("Blue-ish", &[0, 1, 2]),
                ("Cyber-pink", &[2, 0, 1]),
                ("Cyber-purple", &[1, 0, 2]),
            ],
            0,
        )? {
            Some(val) => val,
            None => return Ok(None),
        };

        let intensity = match select(
            "Saturation",
            vec![
                ("Cloudy (x2)", &[400, 800, 1_600]),
                ("Warm (x3)", &[215, 645, 1_935]),
                ("Intense (x10)", &[25, 250, 2_500]),
            ],
            1,
        )? {
            Some(val) => val,
            None => return Ok(None),
        };

        let definition = match select(
            "Definition",
            vec![("Faded", &2), ("Bright", &4), ("Harsh", &8)],
            1,
        )? {
            Some(val) => val,
            None => return Ok(None),
        };

        let limits: [u32; CHANNELS as usize] = [
            (intensity[color_palette[0]] * definition),
            (intensity[color_palette[1]] * definition),
            (intensity[color_palette[2]] * definition),
        ];

        if limits.len() as u32 != CHANNELS {
            return Err(format!("This program expects {CHANNELS} channels, but the limits array was set up with {} values", limits.len()).into());
        }

        let resolution = match select(
            "Resolution",
            vec![
                ("Small (1024)", &(1 << 10)),
                ("Medium (2048)", &(1 << 11)),
                ("Large (4096)", &(1 << 12)),
                ("Massive (8096)", &(1 << 13)),
                ("Love knows no bounds (16 384)", &(1 << 14)),
            ],
            1,
        )? {
            Some(val) => *val,
            None => return Ok(None),
        };

        let iterations = match select(
            "Quality",
            vec![
                ("Draft", &100_000),
                ("Normal", &1_000_000),
                ("Smooooth", &10_000_000),
            ],
            1,
        )? {
            Some(val) => *val,
            None => return Ok(None),
        };

        let settings = RenderSettings {
            limits,
            samples: iterations,
            threads: Some(threads),
            passes: DEFAULT_RENDER_SETTINGS.passes,
            size: resolution,
            curve: DEFAULT_RENDER_SETTINGS.curve,
        };

        if Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt(format!("Render like this?"))
            .default(true)
            .interact()?
        {
            Ok(Some(settings))
        } else {
            Err("Canceled".into())
        }
    }
}

fn select<'a, T>(
    prompt: &str,
    items: Vec<(&str, &'a T)>,
    default: usize,
) -> Result<Option<&'a T>, Error> {
    let (selections, values): (Vec<&str>, Vec<&T>) = items.into_iter().unzip();
    match Select::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .items(&selections)
        .default(default)
        .interact_on_opt(&Term::stderr())?
    {
        Some(index) => Ok(Some(values[index])),
        None => Ok(None),
    }
}
