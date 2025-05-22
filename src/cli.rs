use std::path::PathBuf;

#[derive(Debug, clap::Parser, zeroize::Zeroize, zeroize::ZeroizeOnDrop)]
#[command(name = "warehouseify")]
#[command(version, long_about = None)]
#[command(about = "⌂ Manage your own cargo-binstall repository.")]
pub struct Args {
    #[arg(short, long, value_name = "FILE")]
    #[zeroize(skip)]
    /// Path to a warehouseify config file. If not specified, will use default values.
    pub(crate) config: Option<PathBuf>,
    #[arg(long, value_name = "MINISIGN_KEY")]
    /// Minisign secret key, used to sign the resulting binstall-ready crate. Provide it here, or in the warehousify config file under options.signing_key. Only supports encrypted secret keys.
    pub(crate) signing_key: Option<String>,
    #[arg(short = 'p', long, value_name = "MINISIGN_KEY")]
    /// Minisign secret key password, used to unlock the signing key.
    pub(crate) signing_key_password: String,
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    /// Turn on verbose logging. The default log level is "INFO".
    /// Each instance of "v" in "-v" will increase the logging level by one. Logging levels are
    /// DEBUG (-v) and TRACE (-vv).
    /// "Quiet" settings override "verbose" settings.
    pub(crate) verbose: u8,
    #[arg(short = 'q', long, action = clap::ArgAction::Count)]
    /// Configure "quiet" mode. The default log level is "INFO".
    /// Each instance of "q" in "-q" will decrease the logging level by one. Logging levels are
    /// WARN (-q), ERROR (-qq) and None (completely silent, except for regular stdout) (-qqq).
    /// "Quiet" settings override "verbose" settings.
    pub(crate) quiet: u8,
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
