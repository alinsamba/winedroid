use std::fs::File;
use std::io::Read;
use std::path::Path;

use zip::ZipArchive;

use crate::error::{Result, WinedroidError};

#[derive(Debug, Clone)]
pub struct ApkEntry {
    pub name: String,
    pub compressed_size: u64,
    pub uncompressed_size: u64,
    pub compression_method: String,
}

pub struct ApkArchive {
    archive: ZipArchive<File>,
}

impl ApkArchive {
    pub fn open(path: &Path) -> Result<Self> {
        let file = File::open(path).map_err(|error| {
            WinedroidError::with_source(format!("failed to open APK `{}`", path.display()), error)
        })?;

        let archive = ZipArchive::new(file).map_err(|error| {
            WinedroidError::with_source(
                format!("failed to read APK ZIP structure `{}`", path.display()),
                error,
            )
        })?;

        Ok(Self { archive })
    }

    pub fn entries(&mut self) -> Result<Vec<ApkEntry>> {
        let mut entries = Vec::with_capacity(self.archive.len());

        for index in 0..self.archive.len() {
            let file = self.archive.by_index(index)?;

            entries.push(ApkEntry {
                name: file.name().to_owned(),
                compressed_size: file.compressed_size(),
                uncompressed_size: file.size(),
                compression_method: format!("{:?}", file.compression()),
            });
        }

        Ok(entries)
    }

    pub fn read_required_file(&mut self, name: &str) -> Result<Vec<u8>> {
        let mut file = self.archive.by_name(name).map_err(|error| {
            WinedroidError::with_source(
                format!("APK does not contain required file `{name}`"),
                error,
            )
        })?;

        let mut data = Vec::with_capacity(file.size() as usize);
        file.read_to_end(&mut data)?;

        Ok(data)
    }

    pub fn read_optional_file(&mut self, name: &str) -> Result<Option<Vec<u8>>> {
        let mut file = match self.archive.by_name(name) {
            Ok(file) => file,
            Err(zip::result::ZipError::FileNotFound) => return Ok(None),
            Err(error) => return Err(error.into()),
        };

        let mut data = Vec::with_capacity(file.size() as usize);
        file.read_to_end(&mut data)?;

        Ok(Some(data))
    }
}
