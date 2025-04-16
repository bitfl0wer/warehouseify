use clap::Parser;
use cli::Args;

mod cli;

fn main() {
    let _args = Args::parse();
    println!("Hello, world!");
}
