use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::os::unix::prelude::MetadataExt;
use std::path::PathBuf;

use cargo_toml::DepsSet;
use serde::Deserialize;

use crate::StdError;

#[derive(Deserialize, Debug)]
/// Represents the structure of the `config.toml` configuration file.
pub(crate) struct ConfigFile {
    /// Crates, which are supposed to be built and signed
    pub(crate) crates: CratesConfig,
    /// Additional configuration options, such as architectures to build for
    pub(crate) options: OptionsConfig,
    /// A pre-set list of dependencies, which will be used before, during or after compilation. The
    /// user cannot add their own dependencies, but can toggle whether they are used and if so, which
    /// version to use.
    pub(crate) dependencies: DependenciesConfig,
}

impl ConfigFile {
    /// Tries to parse the file given at [PathBuf] as [Self]. Will refuse to read files over 10mb
    /// in size. If your config file is over 10mb in size, then you need to get checked out by doctors.
    /// The actual reason behind this is trying to avert denial-of-service via memory exhaustion,
    /// if a gigantic file is passed on accident or on purpose, by an adversary.
    pub(crate) fn try_parse(path: PathBuf) -> Result<Self, StdError<'static>> {
        let mut contents = String::new();
        let mut file = File::open(path)?;
        if file.metadata()?.size() > 10_000_000u64 {
            return Err(String::from(
                "stubbornly refusing to parse a config file that is over 10mb in size",
            )
            .into());
        }
        file.read_to_string(&mut contents)?;
        Ok(toml::from_str(&contents)?)
    }
}

#[derive(Deserialize, Debug)]
pub(crate) struct CratesConfig {
    #[serde(flatten)]
    /// The list of crates to compile. Must be in the same format as a `Cargo.toml` would expect.
    pub(crate) crates: DepsSet,
}

#[derive(Deserialize, Debug)]
pub(crate) struct OptionsConfig {
    /// A list of architectures which all crates are being built for.
    pub(crate) architectures: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct DependenciesConfig {
    #[serde(flatten)]
    /// Pre- and post-compilation dependencies which the user can toggle and adjust.
    pub(crate) properties: HashMap<String, DependencyProperties>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DependencyProperties {
    /// Whether this dependency is enabled
    pub(crate) enabled: bool,
    /// The version identifier of the dependency
    pub(crate) version: String,
}
