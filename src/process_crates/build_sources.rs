use std::path::Path;
use std::process::Command;
use std::time::SystemTime;

use build_command::create_build_command;
use cargo_toml::Manifest;
use log::{debug, error, info, trace, warn};
use minisign::PublicKey;

use crate::process_crates::panic_on_dangerous_path;
use crate::{ConfigFile, SECRET, StdErrorS};

// TODO
// BUG
// The output directory specified by the workspace path in the config MUST be empty before
// stuff is built for SECURITY REASONS! we do NOT want to sign arbitrary binaries! We should
// also check AFTER building, ensuring that only the directories and files exist, which
// we should have created.

mod build_command {
    use log::trace;

    use super::*;

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
    #[must_use]
    pub(super) fn create_build_command(
        config: &ConfigFile,
        crate_path: &Path,
        crate_name: &str,
    ) -> Command {
        let crate_name = crate_name.trim().to_lowercase();
        trace!("Creating build command for {crate_name}");
        let mut base_cmd = Command::new("cargo");
        if check_auditable(config) {
            base_cmd.arg("auditable");
        };
        base_cmd
            .arg("build")
            .arg("--manifest-path")
            .arg(crate_path.join("Cargo.toml"))
            .arg("--release");
        base_cmd
    }
}

/// Sign all binaries created in the output dir specified in the [ConfigFile]. Will error if any
/// errors occur during signing.
pub(crate) fn sign_file(config: &ConfigFile, file: &[u8]) -> Result<Vec<u8>, StdErrorS> {
    crate::check_minisign();
    Ok(minisign::sign(
        Some(
            &match PublicKey::from_base64(config.options.verifying_key.as_str()) {
                Ok(key) => key,
                Err(e) => {
                    error!(
                        "The public/verifying key provided in the config file is not valid Base64: {e}"
                    );
                    panic!("Malformed public key");
                }
            },
        ),
        SECRET.get().expect("SECRET not set!"),
        file,
        None,
        None,
    )?.to_bytes())
}

/// Builds a crate source, signs it and verifies the signature.
///
/// `name` is the name of the folder of the crate source on disk.
///
/// Outputs a triple `(String, Vec<u8>, Vec<u8>)` on success, where
///
/// - `(String, ` is the name of the binary, including the file suffix. The name is formatted as
///   `[crate_name]-[crate-version]-[ISO 8601 build-timestamp]<.[suffix]>`.
/// - ` Vec<u8>)` contains the entire binary, as bytes.
///
/// Will error, if
///
/// - The signature could not be produced
/// - The produced signature somehow doesn't match the computed signature
/// - The crate fails to build
/// - There is an I/O error
pub(crate) fn build_crate(
    config: &ConfigFile,
    crate_path: &Path,
) -> Result<(String, Vec<u8>), StdErrorS> {
    let manifest_path = crate_path.join("Cargo.toml");
    trace!("Locating manifest at {manifest_path:?}");
    let manifest = Manifest::from_path(manifest_path)?;
    let name = &manifest.package().name;
    info!("Building crate {name}...");
    let build_result = match create_build_command(config, crate_path, name).output() {
        Ok(out) => out,
        Err(e) => {
            error!("cargo process died unexpectedly: {e}");
            panic!("Couldn't build binary");
        }
    };
    if build_result.status.code() != Some(0) {
        error!(
            "cargo returned exit code {} when building crate {name}",
            build_result.status
        );
        return Err(format!(
            "cargo returned exit code {}: {}",
            build_result.status,
            String::from_utf8_lossy(build_result.stderr.as_slice())
        )
        .into());
    }
    let release_binary_path = match crate_path
        .join("target")
        .join("release")
        .join(name)
        .canonicalize()
    {
        Ok(path) => {
            trace!("Found release binary!");
            path
        }
        Err(e) => {
            error!(
                r#"Release binary "{name}" not found. Does the crate have a bin target under a different name? {e}"#
            );
            todo!(
                "Cargo projects with multiple targets or target binaries with names different from the crate name not yet supported. This is a planned feature, though."
            );
        }
    };
    assert!(release_binary_path.exists());
    assert!(release_binary_path.is_file());
    debug!("Trying to open release binary file at path {release_binary_path:?}");

    let file_buf = match std::fs::read(release_binary_path) {
        Ok(contents) => contents,
        Err(e) => {
            error!("Reading the binary file failed: {e}");
            panic!("I/O error");
        }
    };
    let timestamp = iso8601_timestamp::Timestamp::from(SystemTime::now()).to_string();

    if config.options.autodelete_sources {
        match std::fs::remove_dir_all(panic_on_dangerous_path(crate_path)) {
            Ok(_) => (),
            Err(e) => warn!(
                "Unable to delete the sources for {name}; You will have to clean it up manually: {e}"
            ),
        };
    }
    info!("Done!");
    Ok((
        format!("{name}-{}-{timestamp}", manifest.package().version.get()?),
        file_buf,
    ))
}
