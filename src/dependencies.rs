use std::collections::{HashMap, HashSet};
use std::process::{Command, ExitStatus, Output, Stdio};

use semver::{Version, VersionReq};

use crate::StdError;
use crate::config::DependenciesConfig;

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub(crate) struct Crate {
    pub(crate) name: String,
    pub(crate) version: String,
}

/// Executes `cargo install --list` on the host, collects its' output and checks if the command
/// executed successfully.
fn execute_cargo_install_list() -> Result<(ExitStatus, Output), StdError<'static>> {
    let installed_crates_output = Command::new("cargo")
        .arg("install")
        .arg("--list")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
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
        Err(String::from(
            "Executing 'cargo install --list' failed on the host. Is cargo available on PATH?",
        )
        .into())
    } else {
        Ok((installed_crates_output.status, installed_crates_output))
    }
}

/// Checks if the dependencies listed in the [DependenciesConfig] are available on the system. Returns
/// a list of missing dependencies as a [Vec] of [DependencyProperties], if successful. If an error is
/// encountered, it is returned as an [StdError].
///
/// ## Requirements
///
/// Requires `cargo` to be installed on the host system and on the user's `PATH`.
pub(crate) fn list_missing_dependencies(
    dependency_requirements: &DependenciesConfig,
) -> Result<HashSet<Crate>, StdError<'static>> {
    let installed_crates = get_installed_crates_on_host()?;
    log::trace!("List of crates installed on host: {installed_crates:?}");

    let mut crates_not_found = HashSet::new();
    for (required_dependency_name, required_dependency_info) in
        dependency_requirements.properties.iter()
    {
        log::trace!(
            "Now processing dependency {required_dependency_name} in config file"
        );
        if !required_dependency_info.enabled {
            log::debug!(
                "Dependency found but disabled in config; skipping: {} v{}",
                required_dependency_name,
                required_dependency_info.version
            );
            continue;
        }
        let crateified_dependency = Crate {
            name: required_dependency_name.clone(),
            version: format_crate_version(&required_dependency_info.version),
        };
        if let Some(is_installed) = installed_crates.get(required_dependency_name.as_str()) {
            log::trace!(
                "Found crate {} to be installed on host. Determining if version requirements are fulfilled...",
                is_installed.name
            );
            let installed_version_semver = match Version::parse(&is_installed.version) {
                Ok(v) => v,
                Err(e) => {
                    log::warn!(
                        "Misformated dependency found on host. This is likely a bug with warehouseify. Please report this exception: {e}"
                    );
                    continue;
                }
            };
            let config_required_version_semver = match VersionReq::parse(
                &crateified_dependency.version,
            ) {
                Ok(v) => v,
                Err(e) => {
                    log::error!(
                        "Misformated dependency found in your configuration file. This is likely not a bug with warehouseify. Exception: {e}"
                    );
                    panic!("Aborting: Dependency in config file is misformated")
                }
            };
            if !config_required_version_semver.matches(&installed_version_semver) {
                log::info!(
                    "Host-installed crate dependency {} at version {} does not match semver requirements laid out in configuration file, which calls for version {}.",
                    &is_installed.name,
                    &installed_version_semver,
                    config_required_version_semver
                );
                crates_not_found.insert(crateified_dependency);
                continue;
            } else {
                log::debug!(
                    r#"Host-installed crate dependency "{}" matches the requirements laid out in the config file."#,
                    is_installed.name
                );
                continue;
            }
        }
        log::debug!(
            r#"Could not find crate "{}" on host system."#,
            crateified_dependency.name
        );
        crates_not_found.insert(crateified_dependency);
    }
    log::debug!(
        "Determined the following crates to be missing on the host: {crates_not_found:?}"
    );
    Ok(crates_not_found)
}

/// Takes a version string outputted by the `cargo install --list` command and tries to normalize it
/// by removing stray leading/tailing characters. This method expects output of the above mentioned
/// command and does not work on any random version-like string.
fn format_crate_version(cargo_install_version: &str) -> String {
    let mut version_string = String::from(cargo_install_version);
    version_string = version_string.trim().to_string();
    if version_string.starts_with('v') {
        version_string = version_string.split_off(1);
    }
    if version_string.ends_with(':') {
        version_string.pop();
    }
    version_string
}

/// Tries to retrieve a structured set of crates installed via `cargo install` or `cargo-binstall`
/// on the host system. Notably, this function invokes the `cargo install --list` command on the
/// host and as such, requires `cargo` to be available on the users' `PATH`.
fn get_installed_crates_on_host() -> Result<HashMap<String, Crate>, StdError<'static>> {
    let cargo_install_list_output = execute_cargo_install_list()?;
    let stdout = String::from_utf8(cargo_install_list_output.1.stdout)?;
    if stdout.is_empty() {
        // Fast path to hell, baby
        return Err(String::from(
            "received empty stdout when calling cargo install --list; assuming command failed",
        )
        .into());
    }

    let mut installed_crates = HashMap::new();
    for line in stdout.lines() {
        // cargo install --list not only outputs the installed crates and their versions, but what
        // commands these crates come with too. This check filters the latter out, since we do not
        // need it.
        if line.starts_with(' ') {
            continue;
        }

        let parts = line
            .split_whitespace()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();

        let crate_name = match parts.first().cloned() {
            Some(name) => name,
            None => {
                return Err(String::from(
                    "no crate name found, is the output malformed? file a bug report",
                )
                .into());
            }
        };

        let crate_version = match parts.get(1).cloned() {
            Some(version) => format_crate_version(&version),
            None => {
                return Err(String::from(
                    "no crate version found, is the output malformed? file a bug report",
                )
                .into());
            }
        };

        log::trace!(
            r#"Inserted crate "{crate_name}" version "{crate_version}" into installed_crates map"#
        );
        installed_crates.insert(
            crate_name.clone(),
            Crate {
                name: crate_name,
                version: crate_version,
            },
        );
    }
    Ok(installed_crates)
}
