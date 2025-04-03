use std::fs::{DirEntry, File};
use std::io::BufWriter;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use srtemplate::SrTemplate;

use crate::dist::build_package;
use crate::metadata::Config;
use crate::pkgbuild::pkgbuild;
use crate::CargoAurResult;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(next_line_help = true)]
pub struct CargoAurArgs {
    /// Don't actually build anything.
    #[clap(short, long)]
    dryrun: bool,

    #[clap(subcommand)]
    pub(crate) action: CargoAurActions,

    #[clap(short, long, default_value = "target/cargo-aur")]
    pub(crate) output_folder: PathBuf,
}

#[derive(Clone, Debug, Subcommand)]
pub enum CargoAurActions {
    #[clap(alias = "b")]
    Build {
        /// Use the MUSL build target to produce a static binary.
        #[clap(long, short, default_value = "false")]
        musl: bool,
    },
    #[clap(alias = "g")]
    Generate { input: PathBuf },
}

pub fn get_args() -> CargoAurArgs {
    CargoAurArgs::parse()
}

impl CargoAurActions {
    pub fn exec(&self, output: &PathBuf, config: &Config, licenses: &[DirEntry]) -> CargoAurResult {
        let generated_file = match self {
            CargoAurActions::Build { musl } => build_package(*musl, output, config, licenses)?,
            CargoAurActions::Generate { input } => {
                let output_file = output.join(input.file_name().ok_or(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Invalid input filename",
                ))?);

                if input.canonicalize()? == output_file.canonicalize()? {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "Cannot copy file to itself - would cause data loss!",
                    )
                    .into());
                }

                std::fs::copy(input, &output_file)?;
                output_file.to_str().unwrap().to_string()
            }
        };

        let ctx_template = SrTemplate::default();
        config.package.fill_template(&ctx_template);
        let pkgbuild_path = output.join("PKGBUILD");
        let file = BufWriter::new(File::create(pkgbuild_path)?);
        let sha256 = config.package.sha256sum(generated_file)?;

        pkgbuild(ctx_template, file, config, &sha256, licenses)
    }
}
