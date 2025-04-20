use std::collections::HashMap;
use std::path::PathBuf;

use cargo_toml::{Dependency, DepsSet};
use log::error;

use crate::config::DependenciesConfig;
use crate::{Crate, StdError, StdErrorS};

mod build_sources;
#[cfg(feature = "http-client")]
mod download_sources;
mod edit_sources;

fn process_crates(
    build_dependencies: &[DependenciesConfig],
    crate_information: DepsSet,
) -> Result<HashMap<Crate, Vec<u8>>, StdError<'static>> {
    let sorted_crates = sort_crates_into_buckets(crate_information)?;
    todo!()
}

/// Whether an http client is available in the current runtime environment.
fn http_client_available() -> bool {
    #[cfg(not(feature = "http-client"))]
    return false;
    #[cfg(feature = "http-client")]
    return true;
}

pub(crate) struct SortedCrates {
    pub(crate) locally_unavailable_crates: Vec<(String, Dependency)>,
    pub(crate) locally_available_crates: Vec<(String, Dependency)>,
}

fn sort_crates_into_buckets(crates: DepsSet) -> Result<SortedCrates, StdErrorS> {
    let mut locally_available_crates = Vec::new();
    let mut locally_unavailable_crates = Vec::new();
    for a_crate in crates.into_iter() {
        if a_crate.1.is_crates_io() {
            if !http_client_available() {
                error!(
                    "Crate {} specified in the configuration file points to a git repository, but this binary has been compiled without an http client dependency.",
                    a_crate.0
                );
                return Err(String::from("Invalid crate reference in configuration").into());
            }
            locally_unavailable_crates.push(a_crate);
            continue;
        }
        if let Some(crate_detail) = a_crate.1.detail() {
            if let Some(_git_path) = crate_detail.git.clone() {
                if !http_client_available() {
                    error!(
                        "Crate {} specified in the configuration file points to a git repository, but this binary has been compiled without an http client dependency.",
                        a_crate.0
                    );
                    return Err(String::from("Invalid crate reference in configuration").into());
                }
                locally_unavailable_crates.push(a_crate);
                continue;
            }
            if let Some(local_path) = crate_detail.path.clone() {
                let path_buf = PathBuf::from(local_path);
                if path_buf.exists()
                    && path_buf.is_dir()
                    && path_buf.join("Cargo.toml").exists()
                    && path_buf.join("Cargo.toml").is_file()
                {
                    locally_available_crates.push(a_crate);
                    continue;
                } else {
                    error!(
                        "Crate {} specified in the configuration file points to a local path, but that path either does not exist, is not or a directory, or does not hold a Config.toml.",
                        a_crate.0
                    );
                    return Err(String::from("Invalid crate reference in configuration").into());
                }
            }
        } else {
            error!(
                "Crate {} specified in the configuration file is invalid. The crate seems to point to an alternative crates registry, which is unsupported behavior at this time.",
                a_crate.0
            );
            return Err(String::from("Invalid crate reference in configuration").into());
        }
    }
    Ok(SortedCrates {
        locally_available_crates,
        locally_unavailable_crates,
    })
}
