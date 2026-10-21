use crate::output::unique_path;
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};
use zip::ZipArchive;

pub const MAX_ENTRIES: usize = 10_000;
pub const MAX_TOTAL_SIZE: u64 = 2 * 1024 * 1024 * 1024; // 2 GB
pub const MAX_SINGLE_SIZE: u64 = 512 * 1024 * 1024; // 512 MB

pub fn open_zip(path: &Path) -> Result<ZipArchive<File>, String> {
    let file = File::open(path).map_err(|e| format!("Cannot open archive: {e}"))?;
    ZipArchive::new(file).map_err(|e| format!("This ZIP archive is invalid or unsupported: {e}"))
}

pub fn list_zip_contents(path: &Path) -> Result<Vec<String>, String> {
    let mut archive = open_zip(path)?;
    if archive.len() > MAX_ENTRIES {
        return Err(format!(
            "This archive has too many entries to preview safely (limit: {MAX_ENTRIES})."
        ));
    }
    let mut result = Vec::with_capacity(archive.len());
    for i in 0..archive.len() {
        let entry = archive
            .by_index(i)
            .map_err(|e| format!("Cannot read archive entry: {e}"))?;
        result.push(entry.name().to_string());
    }
    Ok(result)
}

pub fn extract_zip_safe(input_path: &Path) -> Result<PathBuf, String> {
    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Archive");
    let desired = input_path.with_file_name(stem);
    let output_dir = unique_path(&desired);

    fs::create_dir(&output_dir).map_err(|e| format!("Cannot create extraction folder: {e}"))?;

    let extraction_result = (|| -> Result<(), String> {
        let mut archive = open_zip(input_path)?;
        if archive.len() > MAX_ENTRIES {
            return Err(format!(
                "Archive contains more than {MAX_ENTRIES} entries (limit: {MAX_ENTRIES})."
            ));
        }

        let mut total_uncompressed = 0u64;

        for i in 0..archive.len() {
            let mut entry = archive
                .by_index(i)
                .map_err(|e| format!("Archive entry {i} is damaged: {e}"))?;

            if entry.is_symlink() {
                return Err(
                    "Extraction stopped: Symbolic links are not allowed in archives.".into(),
                );
            }

            // Path Traversal Security: enclosed_name prevents directory escape
            let safe_name = entry
                .enclosed_name()
                .ok_or_else(|| format!("Path traversal attempt rejected in entry: {}", entry.name()))?
                .to_path_buf();

            if safe_name.as_os_str().is_empty() {
                continue;
            }

            let size = entry.size();
            if size > MAX_SINGLE_SIZE {
                return Err(format!(
                    "Archive item {} exceeds the {} MB limit.",
                    entry.name(),
                    MAX_SINGLE_SIZE / (1024 * 1024)
                ));
            }

            total_uncompressed = total_uncompressed
                .checked_add(size)
                .ok_or_else(|| "Archive size overflow occurred.".to_string())?;

            if total_uncompressed > MAX_TOTAL_SIZE {
                return Err(format!(
                    "Archive exceeds the maximum {} GB extraction limit.",
                    MAX_TOTAL_SIZE / (1024 * 1024 * 1024)
                ));
            }

            let out_path = output_dir.join(safe_name);

            if entry.is_dir() {
                fs::create_dir_all(&out_path)
                    .map_err(|e| format!("Cannot create directory: {e}"))?;
                continue;
            }

            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Cannot create parent directory: {e}"))?;
            }

            let mut target = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&out_path)
                .map_err(|e| format!("Cannot safely create extracted file {}: {e}", out_path.display()))?;

            io::copy(&mut entry, &mut target)
                .map_err(|e| format!("Failed extracting {}: {e}", out_path.display()))?;
            target
                .flush()
                .map_err(|e| format!("Could not finish extracted file: {e}"))?;
        }
        Ok(())
    })();

    if let Err(e) = extraction_result {
        let _ = fs::remove_dir_all(&output_dir);
        return Err(e);
    }

    Ok(output_dir)
}
