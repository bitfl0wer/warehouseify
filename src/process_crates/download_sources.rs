use std::collections::HashMap;

use cargo_toml::Dependency;
use log::{debug, error, info, trace, warn};

use crate::StdErrorS;
use crate::process_crates::unpack_gzip_archive;

use super::{CrateGitInformation, ExternalCrateSource, SortedCrates};

// TODO
// BUG
// The download directory specified by the workspace path in the config MUST be empty before
// this function is called for SECURITY REASONS! We should
// also check AFTER downloading, ensuring that only the directories and files exist, which
// we should have created.
pub(crate) fn download_sources(
    sources: SortedCrates,
) -> Result<HashMap<String, Vec<u8>>, StdErrorS> {
    debug!("Starting download of external crate sources");
    let mut downloaded_sources = HashMap::new();
    let mut crates_io_sources = Vec::new();
    let mut git_sources = Vec::new();

    debug!(
        "Categorizing {} unavailable crates by source type",
        sources.locally_unavailable_crates.len()
    );
    for crate_to_download in sources.locally_unavailable_crates.into_iter() {
        trace!(
            "Processing crate '{}' from source {:?}",
            crate_to_download.0, crate_to_download.1
        );
        match crate_to_download.1 {
            ExternalCrateSource::CratesIo => {
                trace!(
                    "Adding '{}' to crates.io download queue",
                    crate_to_download.0
                );
                crates_io_sources.push((crate_to_download.0, crate_to_download.2))
            }
            ExternalCrateSource::Git(ref info) => {
                trace!(
                    "Adding '{}' to git download queue with info {:?}",
                    crate_to_download.0, info
                );
                git_sources.push(crate_to_download)
            }
        }
    }

    match crates_io_sources.len() {
        0 => info!("No crates.io sources to download."),
        num => {
            info!("Downloading {num} crates from crates.io...",);
            match download_crates_io_sources(&crates_io_sources) {
                Ok(sources) => {
                    debug!(
                        "Successfully downloaded {} crates from crates.io!",
                        sources.len()
                    );
                    sources.into_iter().for_each(|(name, data)| {
                        trace!(
                            "Adding crates.io source '{}' ({} bytes) to results",
                            name,
                            data.len()
                        );
                        _ = downloaded_sources.insert(name, data)
                    });
                }
                Err(e) => {
                    error!("Failed to download crates.io sources: {e}");
                    return Err(e);
                }
            }
        }
    }

    match git_sources.len() {
        0 => info!("No git sources to download."),
        num => {
            info!("Downloading {num} crates from git sources...");
            match download_git_sources(&git_sources) {
                Ok(sources) => {
                    debug!("Successfully downloaded {} crates from git!", sources.len());
                    sources.into_iter().for_each(|(name, data)| {
                        trace!(
                            "Adding git source '{}' ({} bytes) to results",
                            name,
                            data.len()
                        );
                        _ = downloaded_sources.insert(name, data)
                    });
                }
                Err(e) => {
                    error!("Failed to download git sources: {e}");
                    return Err(e);
                }
            }
        }
    }

    match downloaded_sources.len() {
        0 => info!("Took a nap (nothing downloaded)"),
        num => {
            info!("Successfully downloaded all {num} external sources!");
        }
    }

    Ok(downloaded_sources)
}

fn download_crates_io_sources(
    sources: &[(String, Dependency)],
) -> Result<HashMap<String, Vec<u8>>, StdErrorS> {
    debug!("Starting download of {} crates.io sources", sources.len());
    let mut downloaded: HashMap<String, Vec<u8>> = HashMap::new();

    for (name, dependency) in sources {
        debug!("Processing crates.io dependency '{name}'");

        // Extract package name (might be different from dependency name)
        let package_name = match dependency {
            Dependency::Detailed(detail) => {
                let pkg_name = detail.package.clone().unwrap_or_else(|| name.clone());
                if pkg_name != *name {
                    debug!("Dependency '{name}' uses package name '{pkg_name}'");
                }
                pkg_name
            }
            _ => name.clone(),
        };

        // Extract version from dependency
        let version = match dependency {
            Dependency::Simple(version) => {
                debug!("Using simple version '{version}' for dependency '{name}'");
                version.clone()
            }
            Dependency::Detailed(detail) => match &detail.version {
                Some(v) => {
                    debug!("Using detailed version '{v}' for dependency '{name}'");
                    v.clone()
                }
                None => {
                    error!("No version specified for crates.io dependency '{name}'");
                    return Err(
                        format!("No version specified for crates.io dependency '{name}'").into(),
                    );
                }
            },
            Dependency::Inherited(_) => {
                error!("Cannot deduce crate version for crate {name} from inherented dependency!");
                return Err(
                    String::from("Unable to parse crate version: Malformed configuration").into(),
                );
            }
        };

        // First try static.crates.io URL
        let url = format!(
            "https://static.crates.io/crates/{package_name}/{package_name}-{version}.crate"
        );
        trace!("Attempting download from static URL: {url}");

        match minreq::get(&url).send() {
            Ok(response) => {
                if response.status_code == 200 {
                    debug!("Successfully downloaded '{name}' v{version} from static.crates.io");
                    downloaded.insert(name.clone(), unpack_gzip_archive(response.into_bytes())?);
                    continue;
                } else {
                    warn!(
                        "Static URL download failed for '{}' with status {}, falling back to API",
                        name, response.status_code
                    );
                }
            }
            Err(e) => {
                warn!("Static URL request failed for '{name}': {e}, falling back to API");
            }
        }

        // Fall back to API endpoint if static URL fails
        let api_url = format!("https://crates.io/api/v1/crates/{package_name}/{version}/download");
        trace!("Attempting download from API URL: {api_url}");

        match minreq::get(&api_url).send() {
            Ok(api_response) => {
                if api_response.status_code == 200 {
                    debug!("Successfully downloaded '{name}' v{version} from crates.io API");
                    downloaded.insert(
                        name.clone(),
                        unpack_gzip_archive(api_response.into_bytes())?,
                    );
                } else {
                    error!(
                        "Failed to download crate '{}': HTTP status {}",
                        name, api_response.status_code
                    );
                    return Err(format!(
                        "Failed to download crate '{}': HTTP status {}",
                        name, api_response.status_code
                    )
                    .into());
                }
            }
            Err(e) => {
                error!("API request failed for '{name}': {e}");
                return Err(format!("API request failed for '{name}': {e}").into());
            }
        }
    }

    info!(
        "Successfully downloaded {} crates.io sources",
        downloaded.len()
    );
    Ok(downloaded)
}

// TODO this function is fucking huge but i really want to get this project done, refactoring can be
// done later
fn download_git_sources(
    sources: &[(String, ExternalCrateSource, Dependency)],
) -> Result<HashMap<String, Vec<u8>>, StdErrorS> {
    debug!("Starting download of {} git sources", sources.len());
    let mut downloaded: HashMap<String, Vec<u8>> = HashMap::new();

    for (name, source, dependency) in sources {
        debug!("Processing git dependency '{name}'");

        if let ExternalCrateSource::Git(git_info) = source {
            trace!("Git information for '{name}': {git_info:?}");

            // Extract git URL from dependency
            let git_url = match dependency {
                Dependency::Detailed(detail) => match &detail.git {
                    Some(url) => {
                        debug!("Using git URL: {url} for '{name}'");
                        url.clone()
                    }
                    None => {
                        error!("No git URL specified for dependency '{name}'");
                        return Err(format!("No git URL specified for dependency '{name}'").into());
                    }
                },
                _ => {
                    error!("Invalid dependency format for git source '{name}'");
                    return Err(format!("Invalid dependency format for git source '{name}'").into());
                }
            };

            // Handle different git hosts
            if git_url.contains("github.com") {
                debug!("Processing GitHub repository for '{name}'");

                // Parse GitHub repository information
                let repo_parts: Vec<&str> = git_url.trim_end_matches(".git").split('/').collect();
                let owner = repo_parts[repo_parts.len() - 2];
                let repo = repo_parts[repo_parts.len() - 1];
                trace!("Parsed GitHub repo: owner='{owner}', repo='{repo}'");

                // Construct download URL based on git reference type
                let download_url = match git_info {
                    CrateGitInformation::Branch(branch) => {
                        trace!("Using branch '{branch}' for GitHub repo {owner}/{repo}");
                        format!("https://github.com/{owner}/{repo}/archive/refs/heads/{branch}.zip")
                    }
                    CrateGitInformation::Commit(commit) => {
                        trace!("Using commit '{commit}' for GitHub repo {owner}/{repo}");
                        format!("https://github.com/{owner}/{repo}/archive/{commit}.zip")
                    }
                    CrateGitInformation::Tag(tag) => {
                        trace!("Using tag '{tag}' for GitHub repo {owner}/{repo}");
                        format!("https://github.com/{owner}/{repo}/archive/refs/tags/{tag}.zip")
                    }
                    CrateGitInformation::None => {
                        warn!(
                            "No specific git reference provided for '{name}', trying main branch first"
                        );
                        // Try main branch first
                        let main_url = format!(
                            "https://github.com/{owner}/{repo}/archive/refs/heads/main.zip"
                        );
                        trace!("Attempting to download from main branch: {main_url}");
                        match minreq::get(&main_url).send() {
                            Ok(main_response) => {
                                if main_response.status_code == 200 {
                                    debug!("Successfully downloaded '{name}' from main branch");
                                    downloaded.insert(
                                        name.clone(),
                                        unpack_gzip_archive(main_response.into_bytes())?,
                                    );
                                    continue;
                                } else {
                                    warn!(
                                        "Main branch not found for '{name}', falling back to stable branch"
                                    );
                                }
                            }
                            Err(e) => {
                                warn!("Failed to request main branch for '{name}': {e}");
                            }
                        }

                        // Fall back to stable branch
                        info!("Trying stable branch for GitHub repo {owner}/{repo}");
                        format!("https://github.com/{owner}/{repo}/archive/refs/heads/stable.zip")
                    }
                };

                trace!("Downloading from URL: {download_url}");
                match minreq::get(&download_url).send() {
                    Ok(response) => {
                        if response.status_code != 200 {
                            error!(
                                "Failed to download git source '{}': HTTP status {}",
                                name, response.status_code
                            );
                            return Err(format!(
                                "Failed to download git source '{}': HTTP status {}",
                                name, response.status_code
                            )
                            .into());
                        }

                        debug!(
                            "Successfully downloaded git source '{}' ({} bytes)",
                            name,
                            response.as_bytes().len()
                        );
                        downloaded
                            .insert(name.clone(), unpack_gzip_archive(response.into_bytes())?);
                    }
                    Err(e) => {
                        error!("Request failed for git source '{name}': {e}");
                        return Err(format!("Request failed for git source '{name}': {e}").into());
                    }
                }
            } else if git_url.contains("gitlab.com") {
                debug!("Processing GitLab repository for '{name}'");

                // Parse GitLab repository information
                let repo_parts: Vec<&str> = git_url.trim_end_matches(".git").split('/').collect();
                let owner = repo_parts[repo_parts.len() - 2];
                let repo = repo_parts[repo_parts.len() - 1];
                trace!("Parsed GitLab repo: owner='{owner}', repo='{repo}'");

                // Construct download URL based on git reference type
                let download_url = match git_info {
                    CrateGitInformation::Branch(branch) => {
                        info!("Using branch '{branch}' for GitLab repo {owner}/{repo}");
                        format!(
                            "https://gitlab.com/api/v4/projects/{owner}%2F{repo}/repository/archive.zip?sha={branch}"
                        )
                    }
                    CrateGitInformation::Commit(commit) => {
                        info!("Using commit '{commit}' for GitLab repo {owner}/{repo}");
                        format!(
                            "https://gitlab.com/api/v4/projects/{owner}%2F{repo}/repository/archive.zip?sha={commit}"
                        )
                    }
                    CrateGitInformation::Tag(tag) => {
                        info!("Using tag '{tag}' for GitLab repo {owner}/{repo}");
                        format!(
                            "https://gitlab.com/api/v4/projects/{owner}%2F{repo}/repository/archive.zip?sha={tag}"
                        )
                    }
                    CrateGitInformation::None => {
                        warn!(
                            "No specific git reference provided for '{name}', using default branch"
                        );
                        format!(
                            "https://gitlab.com/api/v4/projects/{owner}%2F{repo}/repository/archive.zip"
                        )
                    }
                };

                trace!("Downloading from URL: {download_url}");
                match minreq::get(&download_url).send() {
                    Ok(response) => {
                        if response.status_code != 200 {
                            error!(
                                "Failed to download git source '{}': HTTP status {}",
                                name, response.status_code
                            );
                            return Err(format!(
                                "Failed to download git source '{}': HTTP status {}",
                                name, response.status_code
                            )
                            .into());
                        }

                        info!(
                            "Successfully downloaded git source '{}' ({} bytes)",
                            name,
                            response.as_bytes().len()
                        );
                        downloaded
                            .insert(name.clone(), unpack_gzip_archive(response.into_bytes())?);
                    }
                    Err(e) => {
                        error!("Request failed for git source '{name}': {e}");
                        return Err(format!("Request failed for git source '{name}': {e}").into());
                    }
                }
            } else {
                warn!("Git host not supported: {git_url}");
                error!("Unsupported git host for dependency '{name}': {git_url}");
                return Err(
                    format!("Unsupported git host for dependency '{name}': {git_url}").into(),
                );
            }
        }
    }

    info!("Successfully downloaded {} git sources", downloaded.len());
    Ok(downloaded)
}
