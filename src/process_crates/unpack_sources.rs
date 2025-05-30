use std::ffi::OsStr;
use std::io::Read;
use std::path::{Path, PathBuf};

use flate2::bufread::GzDecoder;
use log::error;

use crate::{ConfigFile, StdErrorS};

pub(crate) fn unpack_gzip_archive(gzip_archive: Vec<u8>) -> Result<Vec<u8>, StdErrorS> {
    let mut gzip_decoder = GzDecoder::new(gzip_archive.as_slice());
    let mut data_buffer = Vec::with_capacity(gzip_archive.len());
    gzip_decoder.read_to_end(&mut data_buffer)?;
    Ok(data_buffer)
}

/// Finds or creates a folder called `build/` at the target [Path] and unpacks the given `tarball`
/// into a new directory which is named after the last element of that path.
///
/// ## Example
///
/// A given [Path] of `"./my_path/go_here/crate_name"` would result in the tarball being extracted to
/// `"./my_path/go_here/build/crate_name"`
pub(crate) fn write_tar_to_build_dir(
    tarball: Vec<u8>,
    path_to_package: &Path,
) -> Result<(), StdErrorS> {
    let destination_path = match path_to_package.ends_with("build") {
        true => path_to_package,
        false => {
            // Insert "build" before the package name
            let mut path_vec = path_to_package.iter().collect::<Vec<&OsStr>>();
            path_vec.insert(path_vec.len().saturating_sub(1), OsStr::new("build"));
            &PathBuf::from_iter(path_vec.iter())
        }
    };
    let mut tarball_reader = tar::Archive::new(tarball.as_slice());
    tarball_reader.unpack(destination_path)?;
    Ok(())
}

/// Get the path to the directory containing the source files of the crates to compile.
/// Does NOT panic if the path is unsafe (e.g. `/`, `/etc`, `/var`, ...)
pub(crate) fn build_dir(config: &ConfigFile) -> PathBuf {
    config.options.workspace_path.join("build/")
}

/// Get the path to the directory where the compiled binaries are supposed to be located.
/// Does NOT panic if the path is unsafe (e.g. `/`, `/etc`, `/var`, ...)
pub(crate) fn artifact_dir(config: &ConfigFile) -> PathBuf {
    config.options.workspace_path.join("artifacts/")
}

/// Panics, if the path is unsafe (e.g. `/`, `/etc`, `/var`, `/etc/.../` ...)
#[allow(clippy::expect_used)]
#[must_use]
pub(crate) fn panic_on_dangerous_path(path: &Path) -> PathBuf {
    let path = if path.is_relative() {
        &match path.canonicalize() {
            Ok(path) => path,
            Err(e) => {
                error!(
                    r#"Error when attempting to canonicalize path {}: {e}"#,
                    path.to_string_lossy()
                );
                panic!("Error canonicalizing path");
            }
        }
    } else {
        path
    };
    if path.is_absolute() && path.components().collect::<Vec<_>>().len() < 4 {
        panic!(
            "Given path {} looks too dangerous. Aborting.",
            path.to_string_lossy()
        )
    } else {
        path.to_owned()
    }
}

#[cfg(test)]
mod test {
    use std::path::Path;

    use super::panic_on_dangerous_path;

    #[test]
    #[should_panic = "looks too dangerous"]
    fn panic_on_root_dir() {
        panic_on_dangerous_path(Path::new("/"));
    }

    #[test]
    #[should_panic = "looks too dangerous"]
    fn panic_on_etc_dir() {
        panic_on_dangerous_path(Path::new("/etc/"));
    }

    #[test]
    #[should_panic = "looks too dangerous"]
    fn panic_on_etc_subdir() {
        panic_on_dangerous_path(Path::new("/etc/subdir/"));
    }

    #[test]
    fn ok_on_some_homedir() {
        panic_on_dangerous_path(Path::new("/home/runner/repo/"));
    }

    #[test]
    #[should_panic = "looks too dangerous"]
    fn panic_on_relative_root_dir() {
        panic_on_dangerous_path(Path::new("../../../../../../"));
    }

    #[test]
    #[should_panic = "looks too dangerous"]
    fn panic_on_relative_etc_dir() {
        panic_on_dangerous_path(Path::new("../../../../../../etc"));
    }

    #[test]
    #[should_panic = "looks too dangerous"]
    fn panic_on_relative_etc_subdir() {
        panic_on_dangerous_path(Path::new("../../../../../../etc/systemd/"));
    }
}
