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
    env_logger::init();
    CLI_ARGUMENTS
        .set(Args::parse())
        .expect("illegal state: CLI_ARGUMENTS initialized before they have been parsed");
    println!("Hello, world!");
    let config = ConfigFile::try_parse("config.toml".into())?;
    // TODO: Debug statement v
    list_missing_dependencies(&config.dependencies)?;
    Ok(())
}
