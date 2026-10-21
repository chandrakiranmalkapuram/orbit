use image::ImageEncoder;
use serde::Serialize;
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::Command,
};
use zip::ZipArchive;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DetectedFile {
    path: String,
    name: String,
    extension: String,
    kind: String,
    mime: String,
    size: u64,
    actions: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OperationResult {
    output_paths: Vec<String>,
    message: String,
}

fn path_string(path: &Path) -> String { path.to_string_lossy().into_owned() }
fn file_name(path: &Path) -> String { path.file_name().and_then(|s| s.to_str()).unwrap_or("file").to_string() }
fn extension(path: &Path) -> String { path.extension().and_then(|s| s.to_str()).unwrap_or("").to_ascii_lowercase() }

fn unique_path(path: &Path) -> PathBuf {
    if !path.exists() { return path.to_path_buf(); }
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
    let ext = path.extension().and_then(|s| s.to_str());
    for i in 2..100_000 {
        let name = match ext { Some(e) if !e.is_empty() => format!("{stem} ({i}).{e}"), _ => format!("{stem} ({i})") };
        let candidate = parent.join(name);
        if !candidate.exists() { return candidate; }
    }
    parent.join(format!("{stem}-{}", std::process::id()))
}

fn sniff(path: &Path) -> Result<(String, String), String> {
    let mut file = File::open(path).map_err(|e| format!("Cannot read file: {e}"))?;
    let mut head = [0u8; 16];
    let n = file.read(&mut head).map_err(|e| format!("Cannot inspect file: {e}"))?;
    let b = &head[..n];
    if b.starts_with(b"PK\x03\x04") || b.starts_with(b"PK\x05\x06") || b.starts_with(b"PK\x07\x08") { return Ok(("archive".into(), "application/zip".into())); }
    if b.starts_with(b"%PDF-") { return Ok(("pdf".into(), "application/pdf".into())); }
    if b.starts_with(b"\x89PNG\r\n\x1a\n") { return Ok(("image".into(), "image/png".into())); }
    if b.starts_with(b"\xff\xd8\xff") { return Ok(("image".into(), "image/jpeg".into())); }
    if b.len() >= 12 && &b[0..4] == b"RIFF" && &b[8..12] == b"WEBP" { return Ok(("image".into(), "image/webp".into())); }
    if b.starts_with(b"GIF87a") || b.starts_with(b"GIF89a") { return Ok(("image".into(), "image/gif".into())); }
    if b.starts_with(b"BM") { return Ok(("image".into(), "image/bmp".into())); }
    if b.len() >= 4 && (b.starts_with(b"II*\0") || b.starts_with(b"MM\0*")) { return Ok(("image".into(), "image/tiff".into())); }
    let ext = extension(path);
    if ext == "svg" { return Ok(("image".into(), "image/svg+xml".into())); }
    if ["doc", "docx", "odt", "rtf", "txt", "md", "html"].contains(&ext.as_str()) { return Ok(("document".into(), "application/octet-stream".into())); }
    if ["xls", "xlsx", "ods", "csv"].contains(&ext.as_str()) { return Ok(("spreadsheet".into(), "application/octet-stream".into())); }
    if ["ppt", "pptx", "odp"].contains(&ext.as_str()) { return Ok(("presentation".into(), "application/octet-stream".into())); }
    if ["mp3", "wav", "flac", "aac", "m4a", "ogg"].contains(&ext.as_str()) { return Ok(("audio".into(), "application/octet-stream".into())); }
    if ["mp4", "mov", "avi", "mkv", "webm"].contains(&ext.as_str()) { return Ok(("video".into(), "application/octet-stream".into())); }
    Ok(("unknown".into(), "application/octet-stream".into()))
}

#[tauri::command]
fn inspect_file(path: String) -> Result<DetectedFile, String> {
    let p = PathBuf::from(&path);
    if !p.is_file() { return Err("Choose a regular file, not a folder or special file.".into()); }
    let metadata = fs::metadata(&p).map_err(|e| format!("Cannot read file metadata: {e}"))?;
    let (kind, mime) = sniff(&p)?;
    let ext = extension(&p);
    let actions = match kind.as_str() {
        "image" => {
            let mut a = vec![];
            if mime != "image/png" { a.push("to-png".to_string()); }
            if mime != "image/jpeg" { a.push("to-jpeg".to_string()); }
            if mime != "image/webp" { a.push("to-webp".to_string()); }
            a.push("compress-image".to_string());
            a
        },
        "archive" if ext == "zip" || mime == "application/zip" => vec!["extract-zip".into(), "preview-zip".into()],
        _ => vec![],
    };
    Ok(DetectedFile { path, name: file_name(&p), extension: if ext.is_empty() { "unknown".into() } else { ext }, kind, mime, size: metadata.len(), actions })
}

#[tauri::command]
fn convert_image(path: String, format: String, quality: Option<u8>) -> Result<OperationResult, String> {
    let input = PathBuf::from(&path);
    if !input.is_file() { return Err("The source file no longer exists.".into()); }
    let output_format = match format.as_str() { "png" => image::ImageFormat::Png, "jpeg" | "jpg" => image::ImageFormat::Jpeg, "webp" => image::ImageFormat::WebP, _ => return Err("Unsupported image output format.".into()) };
    let decoded = image::ImageReader::open(&input).map_err(|e| format!("Cannot open image: {e}"))?.with_guessed_format().map_err(|e| format!("Cannot identify image format: {e}"))?.decode().map_err(|e| format!("This image cannot be decoded by the bundled image engine: {e}"))?;
    let ext = match format.as_str() { "jpg" => "jpg", "jpeg" => "jpg", other => other };
    let stem = input.file_stem().and_then(|s| s.to_str()).unwrap_or("image");
    let desired = input.with_file_name(format!("{stem}.{ext}"));
    let output = unique_path(&desired);
    let temp = output.with_file_name(format!(".orbit-{}-{}.tmp", std::process::id(), ext));
    let write_result = if output_format == image::ImageFormat::Jpeg {
        let rgb = decoded.to_rgb8();
        let q = quality.unwrap_or(85).clamp(1, 100);
        let mut writer = File::create(&temp).map_err(|e| format!("Cannot create output: {e}"))?;
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut writer, q);
        encoder.write_image(rgb.as_raw(), rgb.width(), rgb.height(), image::ExtendedColorType::Rgb8).map_err(|e| format!("JPEG encoding failed: {e}"))?;
        writer.flush().map_err(|e| format!("Cannot finish output: {e}"))
    } else { decoded.save_with_format(&temp, output_format).map_err(|e| format!("Image conversion failed: {e}")) };
    if let Err(e) = write_result { let _ = fs::remove_file(&temp); return Err(e.to_string()); }
    if output.exists() { let _ = fs::remove_file(&temp); return Err("Output filename was taken while converting. Please try again.".into()); }
    fs::rename(&temp, &output).map_err(|e| { let _ = fs::remove_file(&temp); format!("Could not safely save output: {e}") })?;
    Ok(OperationResult { output_paths: vec![path_string(&output)], message: format!("Saved {} beside your original file.", file_name(&output)) })
}

fn open_zip(path: &Path) -> Result<ZipArchive<File>, String> {
    let file = File::open(path).map_err(|e| format!("Cannot open archive: {e}"))?;
    ZipArchive::new(file).map_err(|e| format!("This ZIP archive is invalid or unsupported: {e}"))
}

#[tauri::command]
fn list_zip_contents(path: String) -> Result<Vec<String>, String> {
    let p = PathBuf::from(path);
    let mut archive = open_zip(&p)?;
    if archive.len() > 10_000 { return Err("This archive has too many entries to preview safely (limit: 10,000).".into()); }
    let mut result = Vec::with_capacity(archive.len());
    for i in 0..archive.len() {
        let entry = archive.by_index(i).map_err(|e| format!("Cannot read archive entry: {e}"))?;
        result.push(entry.name().to_string());
    }
    Ok(result)
}

#[tauri::command]
fn extract_zip(path: String) -> Result<OperationResult, String> {
    const MAX_ENTRIES: usize = 10_000;
    const MAX_TOTAL_SIZE: u64 = 2 * 1024 * 1024 * 1024;
    const MAX_SINGLE_SIZE: u64 = 512 * 1024 * 1024;
    let input = PathBuf::from(&path);
    let stem = input.file_stem().and_then(|s| s.to_str()).unwrap_or("Archive");
    let desired = input.with_file_name(stem);
    let output_dir = unique_path(&desired);
    fs::create_dir(&output_dir).map_err(|e| format!("Cannot create extraction folder: {e}"))?;
    let result = (|| -> Result<(), String> {
        let mut archive = open_zip(&input)?;
        if archive.len() > MAX_ENTRIES { return Err(format!("Archive contains more than {MAX_ENTRIES} entries.")); }
        let mut total = 0u64;
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).map_err(|e| format!("Archive entry {i} is damaged: {e}"))?;
            if entry.is_symlink() { return Err("Extraction stopped because the archive contains a symbolic link.".into()); }
            let safe_name = entry.enclosed_name().ok_or_else(|| format!("Unsafe path in archive: {}", entry.name()))?.to_path_buf();
            if safe_name.as_os_str().is_empty() { continue; }
            let size = entry.size();
            if size > MAX_SINGLE_SIZE { return Err(format!("An archive item exceeds the {} MB per-file limit.", MAX_SINGLE_SIZE / (1024 * 1024))); }
            total = total.checked_add(size).ok_or("Archive size overflow.")?;
            if total > MAX_TOTAL_SIZE { return Err("Archive exceeds the 2 GB total extraction limit.".into()); }
            let out_path = output_dir.join(safe_name);
            if entry.is_dir() { fs::create_dir_all(&out_path).map_err(|e| format!("Cannot create folder: {e}"))?; continue; }
            if let Some(parent) = out_path.parent() { fs::create_dir_all(parent).map_err(|e| format!("Cannot create output folder: {e}"))?; }
            let mut target = OpenOptions::new().write(true).create_new(true).open(&out_path).map_err(|e| format!("Cannot safely create extracted file: {e}"))?;
            io::copy(&mut entry, &mut target).map_err(|e| format!("Failed while extracting {}: {e}", out_path.display()))?;
            target.flush().map_err(|e| format!("Could not finish extracted file: {e}"))?;
        }
        Ok(())
    })();
    if let Err(e) = result { let _ = fs::remove_dir_all(&output_dir); return Err(e); }
    Ok(OperationResult { output_paths: vec![path_string(&output_dir)], message: format!("Extracted to {}.", file_name(&output_dir)) })
}

#[tauri::command]
fn reveal_in_finder(path: String) -> Result<(), String> {
    let p = PathBuf::from(path);
    if !p.exists() { return Err("The output no longer exists.".into()); }
    #[cfg(target_os = "macos")]
    let status = Command::new("open").arg("-R").arg(&p).status();
    #[cfg(target_os = "windows")]
    let status = Command::new("explorer").arg(format!("/select,{}", p.display())).status();
    #[cfg(all(unix, not(target_os = "macos")))]
    let status = Command::new("xdg-open").arg(p.parent().unwrap_or(&p)).status();
    status.map_err(|e| format!("Could not open the file location: {e}"))?.success().then_some(()).ok_or_else(|| "Could not open the file location.".to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![inspect_file, convert_image, list_zip_contents, extract_zip, reveal_in_finder])
        .run(tauri::generate_context!())
        .expect("error while running ORBIT");
}
