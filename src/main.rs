use std::sync::OnceLock;

use clap::Parser;
use cli::Args;
use config::ConfigFile;
use dependencies::list_missing_dependencies;
use log::*;

mod cli;
mod config;
mod dependencies;

static CLI_ARGUMENTS: OnceLock<Args> = OnceLock::new();
pub(crate) type StdError<'a> = Box<dyn std::error::Error + 'a>;

#[allow(clippy::expect_used)]
fn main() -> Result<(), StdError<'static>> {
    #[cfg(debug_assertions)]
    CLI_ARGUMENTS
        .set(Args {
            config: None,
            signing_key: None,
            verbose: 4,
        })
        .expect("You messed up.");
    for _ in 0..10 {
        println!(
            "!!! This crate has been compiled in DEBUG mode and will use trace level logging and DEBUG-only behaviors. To disable this, compile and install the crate in release mode."
        )
    }

    #[cfg(not(debug_assertions))]
    CLI_ARGUMENTS
        .set(Args::parse())
        .expect("illegal state: CLI_ARGUMENTS initialized before they have been parsed");
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
    list_missing_dependencies(&config.dependencies)?;
    Ok(())
}
