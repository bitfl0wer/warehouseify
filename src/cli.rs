use std::path::PathBuf;

#[derive(Debug, clap::Parser)]
#[command(name = "warehouseify")]
#[command(version, long_about = None)]
#[command(about = "⌂ Manage your own cargo-binstall repository.")]
pub struct Args {
    #[arg(short, long, value_name = "FILE")]
    /// Path to a warehouseify config file. If not specified, will use default values.
    pub(crate) config: Option<PathBuf>,
    /// Minisign secret key, used to sign the resulting binstall-ready crate.
    pub(crate) signing_key: Option<minisign::SecretKeyBox>,
    #[arg(short, long, action = clap::ArgAction::Count)]
    /// Turn on verbose logging. The default log level is "WARN".
    /// Each instance of "v" in "-v" will increase the logging level by one. Available, additional
    /// logging levels are INFO (-v), DEBUG (-vv) and TRACE (-vvv)
    pub(crate) verbose: u8,
    #[arg(short, long, default_value_t = false)]
    /// Assume "yes" to all questions asked.
    pub(crate) no_confirm: bool,
}
