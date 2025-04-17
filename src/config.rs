use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::os::unix::prelude::MetadataExt;
use std::path::PathBuf;

use cargo_toml::DepsSet;
use serde::Deserialize;

use crate::StdError;

#[derive(Deserialize, Debug)]
pub(crate) struct ConfigFile {
    crates: CratesConfig,
    options: OptionsConfig,
    dependencies: DependenciesConfig,
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
struct CratesConfig {
    #[serde(flatten)]
    pub(crate) crates: DepsSet,
}

#[derive(Deserialize, Debug)]
struct OptionsConfig {
    pub(crate) architectures: Vec<String>,
}

#[derive(Deserialize, Debug)]
struct DependenciesConfig {
    #[serde(flatten)]
    pub(crate) properties: HashMap<String, DependencyProperties>,
}

#[derive(Debug, Deserialize)]
struct DependencyProperties {
    pub(crate) enabled: bool,
    pub(crate) version: String,
}
