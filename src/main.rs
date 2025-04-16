use std::sync::OnceLock;

use clap::Parser;
use cli::Args;
use log::*;

mod cli;

static CLI_ARGUMENTS: OnceLock<Args> = OnceLock::new();

#[allow(clippy::expect_used)]
fn main() {
    env_logger::init();
    CLI_ARGUMENTS
        .set(Args::parse())
        .expect("illegal state: CLI_ARGUMENTS initialized before they have been parsed");
    println!("Hello, world!");
}
