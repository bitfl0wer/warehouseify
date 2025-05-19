use std::process::Command;

use crate::{ConfigFile, StdErrorS};

use super::build_dir;

// TODO
// BUG
// The output directory specified by the workspace path in the config MUST be empty before
// stuff is built for SECURITY REASONS! we do NOT want to sign arbitrary binaries! We should
// also check AFTER building, ensuring that only the directories and files exist, which
// we should have created.

/// Returns `true` if `cargo-auditable` is a specified and enabled dependency within the config file.
#[must_use]
fn check_auditable(config: &ConfigFile) -> bool {
    config
        .dependencies
        .properties
        .get("cargo-auditable")
        .is_some_and(|dep_props| dep_props.enabled)
}

/// Create the build command for a given crate.
fn create_build_command(config: &ConfigFile, crate_name: &str) -> Command {
    let crate_name = crate_name.trim().to_lowercase();
    let mut base_cmd = Command::new("cargo");
    match check_auditable(config) {
        true => {
            base_cmd.arg("auditable");
        }
        false => (),
    };
    base_cmd
        .arg("build")
        .arg(build_dir(config).join(crate_name))
        .arg("--release");
    base_cmd
}

/// Sign all binaries created in the output dir specified in the [ConfigFile]. Will error if any
/// errors occur during signing.
fn sign_binaries(config: &ConfigFile) -> Result<(), StdErrorS> {
    crate::check_minisign();
    todo!()
}

fn verify_signatures(config: &ConfigFile) -> Result<(), StdErrorS> {
    crate::check_minisign();
    todo!()
}
