use std::fs;
use std::path::Path;

use cargo_toml::Manifest;
use log::{debug, error, info, trace};

use crate::StdErrorS;

pub(crate) fn add_build_meta_info(
    full_crate_path: &Path,
    verifying_key: &str,
) -> Result<(), StdErrorS> {
    let mut crate_manifest = match Manifest::from_path(full_crate_path.join("Cargo.toml")) {
        Ok(file) => file,
        Err(e) => {
            error!(
                "Could not find or open Cargo.toml at path {}. Error: {e}",
                full_crate_path.join("Cargo.toml").to_string_lossy()
            );
            return Err(String::from("encountered file read error").into());
        }
    };

    trace!("Successfully loaded Cargo.toml manifest");

    // Read the file content to work with the TOML structure directly
    let cargo_toml_path = full_crate_path.join("Cargo.toml");
    let toml_content = match fs::read_to_string(&cargo_toml_path) {
        Ok(content) => content,
        Err(e) => {
            error!("Failed to read Cargo.toml: {}", e);
            return Err(String::from("failed to read Cargo.toml").into());
        }
    };

    // Parse the TOML content
    let mut toml_value: toml::Value = match toml::from_str(&toml_content) {
        Ok(value) => value,
        Err(e) => {
            error!("Failed to parse Cargo.toml: {}", e);
            return Err(String::from("failed to parse Cargo.toml").into());
        }
    };

    debug!("Successfully parsed Cargo.toml content");

    // Get the package table
    let root_table = match toml_value.as_table_mut() {
        Some(table) => table,
        None => {
            error!("Cargo.toml is not a valid TOML table");
            return Err(String::from("invalid Cargo.toml format").into());
        }
    };

    let package_table = match root_table.get_mut("package") {
        Some(value) => match value.as_table_mut() {
            Some(table) => table,
            None => {
                error!("Package is not a table in Cargo.toml");
                return Err(String::from("invalid package format").into());
            }
        },
        None => {
            error!("Cargo.toml is missing [package] section");
            return Err(String::from("invalid Cargo.toml format").into());
        }
    };

    // Get or create the metadata table
    if !package_table.contains_key("metadata") {
        debug!("Creating [package.metadata] section");
        package_table.insert(
            "metadata".to_string(),
            toml::Value::Table(toml::value::Table::new()),
        );
    }

    let metadata_table = match package_table.get_mut("metadata") {
        Some(value) => match value.as_table_mut() {
            Some(table) => table,
            None => {
                error!("Metadata is not a table in Cargo.toml");
                return Err(String::from("invalid metadata format").into());
            }
        },
        None => {
            error!("Failed to access metadata section");
            return Err(String::from("failed to access metadata").into());
        }
    };

    // Get or create the binstall table
    if !metadata_table.contains_key("binstall") {
        debug!("Creating [package.metadata.binstall] section");
        metadata_table.insert(
            "binstall".to_string(),
            toml::Value::Table(toml::value::Table::new()),
        );
    }

    let binstall_table = match metadata_table.get_mut("binstall") {
        Some(value) => match value.as_table_mut() {
            Some(table) => table,
            None => {
                error!("Binstall is not a table in Cargo.toml");
                return Err(String::from("invalid binstall format").into());
            }
        },
        None => {
            error!("Failed to access binstall section");
            return Err(String::from("failed to access binstall").into());
        }
    };

    // Get or create the signing table
    if !binstall_table.contains_key("signing") {
        debug!("Creating [package.metadata.binstall.signing] section");
        binstall_table.insert(
            "signing".to_string(),
            toml::Value::Table(toml::value::Table::new()),
        );
    }

    let signing_table = match binstall_table.get_mut("signing") {
        Some(value) => match value.as_table_mut() {
            Some(table) => table,
            None => {
                error!("Signing is not a table in Cargo.toml");
                return Err(String::from("invalid signing format").into());
            }
        },
        None => {
            error!("Failed to access signing section");
            return Err(String::from("failed to access signing").into());
        }
    };

    // Check if the section already exists
    let existing = signing_table.contains_key("algorithm") && signing_table.contains_key("pubkey");

    // Set algorithm and pubkey
    signing_table.insert(
        "algorithm".to_string(),
        toml::Value::String("minisign".to_string()),
    );
    signing_table.insert(
        "pubkey".to_string(),
        toml::Value::String(verifying_key.to_string()),
    );

    if existing {
        info!("Updated existing [package.metadata.binstall.signing] section");
    } else {
        info!("Created new [package.metadata.binstall.signing] section");
    }

    debug!("Algorithm set to 'minisign'");
    debug!("Pubkey set to '{}'", verifying_key);

    // Write the modified TOML back to the file
    let new_toml_content = match toml::to_string(&toml_value) {
        Ok(content) => content,
        Err(e) => {
            error!("Failed to serialize Cargo.toml: {}", e);
            return Err(String::from("failed to serialize Cargo.toml").into());
        }
    };

    trace!("Serialized updated TOML content");

    match fs::write(&cargo_toml_path, new_toml_content) {
        Ok(_) => {
            info!("Successfully wrote updated Cargo.toml to disk");
            Ok(())
        }
        Err(e) => {
            error!("Failed to write Cargo.toml: {}", e);
            Err(String::from("failed to write Cargo.toml").into())
        }
    }
}
