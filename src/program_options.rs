//! Utility for program configuration arguments

use crate::{RenderSettings, DEFAULT_RENDER_SETTINGS};
use clap::{Parser, Subcommand};
use std::process::exit;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// File to write to
    #[clap(short, long, value_parser, default_value = "image.png")]
    output: String,

    /// Do not write intermediate files
    #[clap(short, long, value_parser)]
    no_intermediates: bool,

    /// Configuration file
    #[clap(short, long, value_parser)]
    config: Option<String>,

    /// Alternate behaviours for the program
    #[clap(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Display configuration wizard
    Wizard {
        /// Path to write the selected configuration to
        #[clap(short, long, value_parser)]
        save_config: Option<String>,
    },
    /// Write the default configuration to TOML
    WriteDefault {
        /// Path to write the default configuration to (writes to stdout if unset)
        #[clap(short, long, value_parser)]
        save_config: Option<String>,
    },
}

/// How to run the program
pub struct ProgramOptions {
    /// Rendering settings
    pub render_settings: RenderSettings,

    /// Filepath for output (png image format)
    pub output_path: String,

    /// Output intermediate renders at the end of each pass?
    pub render_intermediates: bool,
}

/// Get options from program arguments
pub fn get_options() -> Result<ProgramOptions, Box<dyn std::error::Error>> {
    let args: Args = Args::parse();
    let render_settings = match &args.command {
        Some(Commands::WriteDefault {
            save_config: config,
        }) => {
            match config {
                Some(path) => {
                    DEFAULT_RENDER_SETTINGS.to_file(path)?;
                }
                None => {
                    println!("{}", DEFAULT_RENDER_SETTINGS.serialize()?);
                }
            };
            exit(0);
        }
        Some(Commands::Wizard {
            save_config: config,
        }) => match RenderSettings::from_wizard()? {
            Some(settings) => {
                if let Some(config) = config {
                    settings.to_file(config)?;
                }
                Ok(settings)
            }
            None => Err("User canceled..."),
        },
        None => {
            if let Some(config_path) = args.config.as_deref() {
                Ok(RenderSettings::from_file(config_path)?)
            } else {
                Ok(DEFAULT_RENDER_SETTINGS)
            }
        }
    }?;

    let render_intermediates = !args.no_intermediates;
    let output_path = args.output.clone();
    Ok(ProgramOptions {
        render_settings,
        output_path,
        render_intermediates,
    })
}
