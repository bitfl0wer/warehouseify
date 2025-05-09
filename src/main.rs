use std::collections::HashSet;
use std::path::PathBuf;
use std::process::{Command, exit};
use std::sync::OnceLock;

#[cfg(not(debug_assertions))]
use clap::Parser;
use cli::Args;
use config::ConfigFile;
use dependencies::{Crate, list_missing_dependencies};
use log::*;
use process_crates::{add_build_meta_info, sort_crates_into_buckets, write_tar_to_build_dir};
use semver::VersionReq;

pub(crate) mod cli;
pub(crate) mod config;
pub(crate) mod dependencies;
pub(crate) mod output;
pub(crate) mod process_crates;

static CLI_ARGUMENTS: OnceLock<Args> = OnceLock::new();
static SECRET: OnceLock<minisign::SecretKeyBox> = OnceLock::new();
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
        .filter(None, LevelFilter::Off)
        .filter(Some("warehouseify"), log_level)
        .try_init()?;
    debug!("Hello, world!");
    let config = ConfigFile::try_parse("config.toml".into())?;
    if cli_arguments.signing_key.is_none() && config.options.signing_key.is_none() {
        error!(
            r#"You must supply a minisign signing key. Either set the "options.signing_key" variable in your configuration file, or provide it through the cli using the "--signing-key" flag."#
        );
        exit(1);
    } else if let Some(secret) = &cli_arguments.signing_key {
        SECRET.set(secret.clone()).unwrap()
    } else if let Some(secret) = &config.options.signing_key {
        SECRET
            .set(
                minisign::SecretKeyBox::from_string(secret)
                    .expect("invalid or malformed minisign signing key supplied"),
            )
            .unwrap()
    }
    trace!("Parsed config: {:#?}", &config);
    let missing_dependencies = list_missing_dependencies(&config.dependencies)?;
    if !cli_arguments.no_confirm && !missing_dependencies.is_empty() {
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
            error!(
                "Cannot proceed without installing missing dependencies. Either manually install them or disable them in your configuration file."
            );
            exit(1)
        }
    } else if !missing_dependencies.is_empty() {
        install_missing_dependencies(
            missing_dependencies
                .iter()
                .cloned()
                .collect::<Vec<Crate>>()
                .as_slice(),
        )?;
    }
    let sorted_crates = sort_crates_into_buckets(config.crates.crates)?;
    let mut size = 0u128;
    #[cfg(feature = "http-client")]
    {
        let downloaded_crates = crate::process_crates::download_sources(sorted_crates.clone())?;
        for item in downloaded_crates.into_iter() {
            size = match size.checked_add(item.1.len() as u128) {
                Some(v) => v,
                None => u128::MAX,
            };
            write_tar_to_build_dir(item.1, &config.options.workspace_path.join(item.0))?;
        }
    }
    // At this point, we have all "remote" crates downloaded in the build directory
    // We can now edit the sources and compile them

    debug!("Received {} kilobytes in crate source code", size / 1000);

    let mut all_crate_paths = Vec::new();
    for item in sorted_crates.locally_unavailable_crates.iter() {
        let crate_path = config
            .options
            .workspace_path
            .join("build")
            .join(item.0.clone());
        trace!("Discovering crates in folder {:?}", crate_path);
        // Add all subdirectories using cool iterator methods
        match std::fs::read_dir(&crate_path) {
            Ok(entries) => {
                let additional_entries = entries
                    .filter_map(Result::ok)
                    .map(|entry| entry.path())
                    .filter(|path| path.is_dir())
                    .collect::<Vec<PathBuf>>();
                debug!("Found subcrate entries: {:?}", additional_entries);
                all_crate_paths.extend(additional_entries.into_iter());
            }
            Err(e) => debug!("Error: {e}"),
        };
    }

    for item in sorted_crates.locally_available_crates.iter() {
        all_crate_paths.push(config.options.workspace_path.join(item.0.clone()));
    }
    for crate_path in all_crate_paths.iter() {
        trace!("Modifying Cargo.toml of {:?}", crate_path);
        add_build_meta_info(crate_path, &config.options.verifying_key)?;
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
