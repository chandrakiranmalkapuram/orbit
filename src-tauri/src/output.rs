use std::{
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

static COUNTER: AtomicU64 = AtomicU64::new(1);

pub fn unique_path(path: &Path) -> PathBuf {
    if fs::symlink_metadata(path).is_err() {
        return path.to_path_buf();
    }
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
    let ext = path.extension().and_then(|s| s.to_str());

    for i in 2..100_000 {
        let name = match ext {
            Some(e) if !e.is_empty() => format!("{stem} ({i}).{e}"),
            _ => format!("{stem} ({i})"),
        };
        let candidate = parent.join(name);
        if fs::symlink_metadata(&candidate).is_err() {
            return candidate;
        }
    }
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    let fallback = match ext {
        Some(e) if !e.is_empty() => format!("{stem}-{}-{count}.{e}", std::process::id()),
        _ => format!("{stem}-{}-{count}", std::process::id()),
    };
    parent.join(fallback)
}

pub struct AtomicOutput {
    pub temp_path: PathBuf,
    pub target_path: PathBuf,
    completed: bool,
}

impl AtomicOutput {
    pub fn new(target_path: PathBuf) -> Result<Self, String> {
        let parent = target_path.parent().unwrap_or_else(|| Path::new("."));
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| format!("Cannot create parent directory: {e}"))?;
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let count = COUNTER.fetch_add(1, Ordering::Relaxed);
        let temp_name = format!(".orbit-{}-{}-{}.tmp", std::process::id(), now, count);
        let temp_path = parent.join(temp_name);

        Ok(Self {
            temp_path,
            target_path,
            completed: false,
        })
    }

    pub fn create_file(&self) -> Result<File, String> {
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&self.temp_path)
            .map_err(|e| format!("Cannot safely create temporary file {}: {e}", self.temp_path.display()))
    }

    pub fn finish(mut self) -> Result<PathBuf, String> {
        // The temporary file is created in the destination directory, so a
        // hard link is an atomic, same-volume publish operation on macOS.
        // Unlike rename(), hard_link() never replaces an existing destination.
        let desired_path = self.target_path.clone();

        loop {
            let candidate = unique_path(&desired_path);
            match fs::hard_link(&self.temp_path, &candidate) {
                Ok(()) => {
                    self.target_path = candidate;
                    self.completed = true;
                    // The destination now owns the completed file. Removing
                    // the temporary name is best-effort cleanup only.
                    let _ = fs::remove_file(&self.temp_path);
                    return Ok(self.target_path.clone());
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    // Another process won the name between lookup and publish.
                    // Resolve a fresh name and retry without clobbering it.
                    continue;
                }
                Err(error) => {
                    return Err(format!(
                        "Cannot safely save final output to {}: {error}",
                        candidate.display()
                    ));
                }
            }
        }
    }
}

impl Drop for AtomicOutput {
    fn drop(&mut self) {
        if !self.completed && self.temp_path.exists() {
            let _ = fs::remove_file(&self.temp_path);
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(1);

    fn test_path(label: &str) -> PathBuf {
        let id = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "orbit-output-test-{}-{id}-{label}",
            std::process::id()
        ))
    }

    #[test]
    fn finish_preserves_existing_destination_and_publishes_new_file() {
        let target = test_path("preserve.txt");
        fs::write(&target, b"original").expect("create original");

        let writer = AtomicOutput::new(target.clone()).expect("create writer");
        {
            let mut file = writer.create_file().expect("create temp output");
            file.write_all(b"converted").expect("write output");
        }

        let published = writer.finish().expect("publish output");
        assert_ne!(published, target);
        assert_eq!(fs::read(&target).expect("read original"), b"original");
        assert_eq!(fs::read(&published).expect("read published"), b"converted");

        let _ = fs::remove_file(target);
        let _ = fs::remove_file(published);
    }

    #[test]
    fn finish_allocates_distinct_names_for_concurrent_outputs() {
        let target = test_path("concurrent.txt");
        let first = AtomicOutput::new(target.clone()).expect("first writer");
        let second = AtomicOutput::new(target.clone()).expect("second writer");

        {
            let mut file = first.create_file().expect("first temp file");
            file.write_all(b"first").expect("write first");
        }
        {
            let mut file = second.create_file().expect("second temp file");
            file.write_all(b"second").expect("write second");
        }

        let first_path = first.finish().expect("publish first");
        let second_path = second.finish().expect("publish second");
        assert_ne!(first_path, second_path);
        assert_eq!(fs::read(&first_path).expect("read first"), b"first");
        assert_eq!(fs::read(&second_path).expect("read second"), b"second");

        let _ = fs::remove_file(first_path);
        let _ = fs::remove_file(second_path);
    }

    #[test]
    fn dropping_incomplete_output_removes_temporary_file() {
        let target = test_path("cleanup.txt");
        let writer = AtomicOutput::new(target).expect("create writer");
        let temporary = writer.temp_path.clone();
        {
            let mut file = writer.create_file().expect("create temp file");
            file.write_all(b"partial").expect("write partial output");
        }

        drop(writer);
        assert!(!temporary.exists());
    }

    #[test]
    fn create_file_does_not_truncate_an_existing_temporary_path() {
        let target = test_path("exclusive.txt");
        let writer = AtomicOutput::new(target).expect("create writer");
        fs::write(&writer.temp_path, b"do not replace").expect("plant existing temp file");

        assert!(writer.create_file().is_err());
        assert_eq!(
            fs::read(&writer.temp_path).expect("read existing temp"),
            b"do not replace"
        );

        let temporary = writer.temp_path.clone();
        drop(writer);
        let _ = fs::remove_file(temporary);
    }
}
