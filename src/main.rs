use std::sync::OnceLock;

use clap::Parser;
use cli::Args;
use config::ConfigFile;
use log::*;

mod cli;
mod config;

static CLI_ARGUMENTS: OnceLock<Args> = OnceLock::new();
pub(crate) type StdError<'a> = Box<dyn std::error::Error + 'a>;

#[allow(clippy::expect_used)]
fn main() -> Result<(), StdError<'static>> {
    env_logger::init();
    CLI_ARGUMENTS
        .set(Args::parse())
        .expect("illegal state: CLI_ARGUMENTS initialized before they have been parsed");
    println!("Hello, world!");
    println!("{:#?}", ConfigFile::try_parse("config.toml".into())?);
    Ok(())
}
