use std::ffi::OsStr;
use std::io::Read;
use std::path::{Path, PathBuf};

use flate2::bufread::GzDecoder;

use crate::StdErrorS;

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
