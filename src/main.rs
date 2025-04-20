use std::collections::HashSet;
use std::process::{Command, exit};
use std::sync::OnceLock;

#[cfg(not(debug_assertions))]
use clap::Parser;
use cli::Args;
use config::ConfigFile;
use dependencies::{Crate, list_missing_dependencies};
use log::*;
use semver::VersionReq;

mod cli;
mod config;
mod dependencies;
mod output;
mod process_crates;

static CLI_ARGUMENTS: OnceLock<Args> = OnceLock::new();
pub(crate) type StdError<'a> = Box<dyn std::error::Error + 'a>;
/// [StdError] with a `'static` lifetime.
pub(crate) type StdErrorS = StdError<'static>;

#[allow(clippy::expect_used)]
fn main() -> Result<(), StdErrorS> {
    #[cfg(debug_assertions)]
    CLI_ARGUMENTS
        .set(Args {
            config: None,
            signing_key: None,
            verbose: 4,
            no_confirm: false,
            locked: false,
            force: true,
        })
        .expect("You messed up.");
    #[cfg(debug_assertions)]
    for _ in 0..10 {
        println!(
            "!!! This crate has been compiled in DEBUG mode and will use trace level logging and DEBUG-only behaviors. To disable this, compile and install the crate in release mode."
        )
    }

    #[cfg(not(debug_assertions))]
    CLI_ARGUMENTS
        .set(Args::parse())
        .expect("illegal state: CLI_ARGUMENTS initialized before they have been parsed");
    let cli_arguments = CLI_ARGUMENTS.get().expect("cli arguments are missing");
    let log_level = match CLI_ARGUMENTS
        .get()
        .expect("cli args have not been parsed")
        .verbose
    {
        0 => LevelFilter::Warn,
        1 => LevelFilter::Info,
        2 => LevelFilter::Debug,
        3 => LevelFilter::Trace,
        _ => {
            println!(
                r#"Woah there! You don't need to supply a bajillion "-v"'s. 3 is the limit! Enabling trace logs anyways, because I'm nice :3"#
            );
            LevelFilter::Trace
        }
    };
    env_logger::Builder::new()
        .filter_level(log_level)
        .try_init()?;
    debug!("Hello, world!");
    let config = ConfigFile::try_parse("config.toml".into())?;
    trace!("Parsed config: {:#?}", &config);
    let missing_dependencies = list_missing_dependencies(&config.dependencies)?;
    if !cli_arguments.no_confirm {
        println!(
            r#"The following dependencies have been determined to be missing on the host system: {}. Would you like to install them by using "cargo install"? [y/N]"#,
            fmt_missing_dependencies(&missing_dependencies)
        );
        let mut buffer = String::new();
        let stdin = std::io::stdin();
        stdin.read_line(&mut buffer)?;
        if buffer.trim().to_lowercase().starts_with('y') {
            install_missing_dependencies(
                missing_dependencies
                    .iter()
                    .cloned()
                    .collect::<Vec<Crate>>()
                    .as_slice(),
            )?;
        } else {
            eprintln!(
                "Cannot proceed without installing missing dependencies. Either manually install them or disable them in your configuration file."
            );
            exit(1)
        }
    }
    Ok(())
}

#[allow(clippy::expect_used)]
/// Takes in a list of [Crate]s and tries to install them on the host with `cargo install`.
/// Will panic the program if CLI args cannot be found. Will return an error, if the specified dependencies
/// are malformed or if they cannot be found on crates.io. Will obviously also error, if `cargo install`
/// returns an error or if the command invocation fails altogether.
fn install_missing_dependencies(deps: &[Crate]) -> Result<(), StdErrorS> {
    let mut command = Command::new("cargo");
    command.arg("install");
    if CLI_ARGUMENTS.get().expect("cli arguments not set").locked {
        command.arg("--locked");
    }
    if CLI_ARGUMENTS.get().expect("cli arguments not set").force {
        command.arg("--force");
    }
    for dependency in deps {
        let version = match VersionReq::parse(&dependency.version) {
            Ok(v) => v,
            Err(e) => {
                log::error!(
                    "Error when parsing version of dependency {} in configuration file: {}",
                    dependency.name,
                    e
                );
                return Err(String::from(
                    "Malformed dependency or dependencies in configuration file",
                )
                .into());
            }
        };
        command.arg(format!(r#"{}@{}"#, dependency.name, version));
    }
    let install_result = command.spawn()?.wait()?;
    log::debug!("{:?}", install_result);
    match install_result.success() {
        true => Ok(()),
        false => Err(format!(
            "the installation of dependencies failed; cannot continue, cargo install exited with {}",
            install_result
        )
        .into()),
    }
}

#[allow(clippy::arithmetic_side_effects)]
/// Basically, a [std::fmt::Display] for `HashSet<Crate>` ordered on wish.com. I can't impl
/// Display for HashSet<Crate> unless I make it a newtype, and I don't want to deal with that.
///
/// Returns a comma delimited list of crates which are missing, like this:
///
/// `cargo_auditable, my_crate, amazing-other-crate`
fn fmt_missing_dependencies(deps: &HashSet<Crate>) -> String {
    let mut missing = String::new();
    for elem in deps.iter() {
        trace!("name of missing crate: {}", elem.name);
        missing += &(elem.name.clone() + ", ");
    }
    if missing.ends_with(", ") {
        let _ = missing.split_off(missing.len().saturating_sub(2));
    }
    trace!("{}", missing);
    missing
}
