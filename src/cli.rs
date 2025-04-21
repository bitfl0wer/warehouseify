use std::path::PathBuf;

#[derive(Debug, clap::Parser)]
#[command(name = "warehouseify")]
#[command(version, long_about = None)]
#[command(about = "⌂ Manage your own cargo-binstall repository.")]
pub struct Args {
    #[arg(short, long, value_name = "FILE")]
    /// Path to a warehouseify config file. If not specified, will use default values.
    pub(crate) config: Option<PathBuf>,
    #[arg(long, value_name = "MINISIGN_KEY")]
    /// Minisign secret key, used to sign the resulting binstall-ready crate. Provide it here, or in the warehousify config file under options.signing_key.
    pub(crate) signing_key: Option<minisign::SecretKeyBox>,
    #[arg(short, long, action = clap::ArgAction::Count)]
    /// Turn on verbose logging. The default log level is "WARN".
    /// Each instance of "v" in "-v" will increase the logging level by one. Logging levels are
    /// INFO (-v), DEBUG (-vv) and TRACE (-vvv)
    pub(crate) verbose: u8,
    #[arg(short, long, default_value_t = false)]
    /// Assume "yes" to all questions asked.
    pub(crate) no_confirm: bool,
    #[arg(long, default_value_t = false)]
    /// When installing dependencies, pass the "--locked" argument to cargo.
    pub(crate) locked: bool,
    /// When installing dependencies, pass the "--force" argument to cargo.
    #[arg(long, default_value_t = false)]
    pub(crate) force: bool,
}
