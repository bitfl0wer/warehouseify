use std::path::PathBuf;

#[derive(Debug, clap::Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(short, long)]
    config: PathBuf,
    #[arg(short, long)]
    signing_key: Option<minisign::SecretKeyBox>,
}
