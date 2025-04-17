use std::io::Read;
use std::process::{ChildStdout, Command};

use crate::StdError;
use crate::config::{DependenciesConfig, DependencyProperties};

/// Checks if the dependencies listed in the [DependenciesConfig] are available on the system. Returns
/// a list of missing dependencies as a [Vec] of [DependencyProperties], if successful. If an error is
/// encountered, it is returned as an [StdError].
///
/// ## Requirements
///
/// Requires `cargo` to be installed on the host system and on the user's `PATH`.
pub(crate) fn list_missing_dependencies(
    dependencies: &DependenciesConfig,
) -> Result<Vec<DependencyProperties>, StdError<'static>> {
    let mut installed_crates_output = Command::new("cargo")
        .arg("install")
        .arg("--list")
        .spawn()?
        .wait_with_output()?;

    let exit_code = installed_crates_output
        .status
        .code()
        .map(|code| code.to_string())
        .unwrap_or_else(|| String::from("UNKNOWN"));
    if exit_code != *"0" {
        log::error!("cargo install --list exited with error code {exit_code}");
        let err_output = String::from_utf8_lossy(&installed_crates_output.stderr).into_owned();
        if !err_output.is_empty() {
            log::error!("{err_output}");
        } else {
            log::error!("process provided no further output in STDERR");
        }
    }

    let stdout = String::from_utf8(installed_crates_output.stdout)?;

    if stdout.is_empty() {
        // Fast path to hell, baby
        return Err(String::from(
            "received empty stdout when calling cargo install --list; assuming command failed",
        )
        .into());
    }
    todo!()
}
