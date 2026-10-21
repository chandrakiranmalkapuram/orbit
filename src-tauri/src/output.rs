use std::{
    fs::{self, File},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

static COUNTER: AtomicU64 = AtomicU64::new(1);

pub fn unique_path(path: &Path) -> PathBuf {
    if !path.exists() {
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
        if !candidate.exists() {
            return candidate;
        }
    }
    let fallback = format!("{stem}-{}", std::process::id());
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
        File::create(&self.temp_path)
            .map_err(|e| format!("Cannot create temporary file {}: {e}", self.temp_path.display()))
    }

    pub fn finish(mut self) -> Result<PathBuf, String> {
        if self.target_path.exists() {
            // Target was created in the meantime, get a new unique path
            self.target_path = unique_path(&self.target_path);
        }

        fs::rename(&self.temp_path, &self.target_path).map_err(|e| {
            let _ = fs::remove_file(&self.temp_path);
            format!("Cannot save final output to {}: {e}", self.target_path.display())
        })?;

        self.completed = true;
        Ok(self.target_path.clone())
    }
}

impl Drop for AtomicOutput {
    fn drop(&mut self) {
        if !self.completed && self.temp_path.exists() {
            let _ = fs::remove_file(&self.temp_path);
        }
    }
}
