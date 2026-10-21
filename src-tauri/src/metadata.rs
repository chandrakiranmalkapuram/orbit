use crate::models::{FileKind, FileMetadata};
use std::{
    fs::{self, File},
    io::Read,
    path::Path,
};

pub fn extension(path: &Path) -> String {
    path.extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
}

pub fn file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("file")
        .to_string()
}

pub fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

pub fn sniff_format(path: &Path) -> Result<(FileKind, String, Option<String>), String> {
    let mut file = File::open(path).map_err(|e| format!("Cannot read file: {e}"))?;
    let mut head = [0u8; 32];
    let n = file.read(&mut head).map_err(|e| format!("Cannot inspect file: {e}"))?;
    let b = &head[..n];

    // Check Magic Signatures
    if b.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Ok((FileKind::Image, "image/png".into(), Some("PNG".into())));
    }
    if b.starts_with(b"\xff\xd8\xff") {
        return Ok((FileKind::Image, "image/jpeg".into(), Some("JPEG".into())));
    }
    if b.len() >= 12 && &b[0..4] == b"RIFF" && &b[8..12] == b"WEBP" {
        return Ok((FileKind::Image, "image/webp".into(), Some("WebP".into())));
    }
    if b.starts_with(b"GIF87a") || b.starts_with(b"GIF89a") {
        return Ok((FileKind::Image, "image/gif".into(), Some("GIF".into())));
    }
    if b.starts_with(b"BM") {
        return Ok((FileKind::Image, "image/bmp".into(), Some("BMP".into())));
    }
    if b.len() >= 4 && (b.starts_with(b"II*\0") || b.starts_with(b"MM\0*")) {
        return Ok((FileKind::Image, "image/tiff".into(), Some("TIFF".into())));
    }
    if b.starts_with(b"%PDF-") {
        return Ok((FileKind::Pdf, "application/pdf".into(), Some("PDF".into())));
    }
    if b.starts_with(b"PK\x03\x04") || b.starts_with(b"PK\x05\x06") || b.starts_with(b"PK\x07\x08") {
        return Ok((FileKind::Archive, "application/zip".into(), Some("ZIP".into())));
    }

    let ext = extension(path);
    if ext == "svg" {
        return Ok((FileKind::Image, "image/svg+xml".into(), Some("SVG".into())));
    }
    if ["doc", "docx", "odt", "rtf", "txt", "md", "html"].contains(&ext.as_str()) {
        return Ok((FileKind::Document, "application/octet-stream".into(), Some(ext.to_uppercase())));
    }
    if ["xls", "xlsx", "ods", "csv"].contains(&ext.as_str()) {
        return Ok((FileKind::Spreadsheet, "application/octet-stream".into(), Some(ext.to_uppercase())));
    }
    if ["ppt", "pptx", "odp"].contains(&ext.as_str()) {
        return Ok((FileKind::Presentation, "application/octet-stream".into(), Some(ext.to_uppercase())));
    }
    if ["mp3", "wav", "flac", "aac", "m4a", "ogg"].contains(&ext.as_str()) {
        return Ok((FileKind::Audio, "audio/octet-stream".into(), Some(ext.to_uppercase())));
    }
    if ["mp4", "mov", "avi", "mkv", "webm"].contains(&ext.as_str()) {
        return Ok((FileKind::Video, "video/octet-stream".into(), Some(ext.to_uppercase())));
    }

    Ok((FileKind::Unknown, "application/octet-stream".into(), None))
}

pub fn inspect_file_properties(path_str: &str) -> Result<FileMetadata, String> {
    let p = Path::new(path_str);
    if !p.is_file() {
        return Err("Choose a regular file, not a folder or special file.".into());
    }

    let metadata = fs::metadata(p).map_err(|e| format!("Cannot read file metadata: {e}"))?;
    let (kind, mime, format_name) = sniff_format(p)?;
    let ext = extension(p);

    let mut width = None;
    let mut height = None;
    let mut has_alpha = None;

    if kind == FileKind::Image {
        if let Ok(reader) = image::ImageReader::open(p) {
            if let Ok(reader_with_format) = reader.with_guessed_format() {
                if let Ok((w, h)) = reader_with_format.into_dimensions() {
                    width = Some(w);
                    height = Some(h);
                }
            }
        }
        if mime == "image/png" || mime == "image/webp" || mime == "image/gif" {
            has_alpha = Some(true);
        } else if mime == "image/jpeg" || mime == "image/bmp" {
            has_alpha = Some(false);
        }
    }

    Ok(FileMetadata {
        path: path_string(p),
        name: file_name(p),
        extension: if ext.is_empty() { "unknown".into() } else { ext },
        kind,
        mime,
        size: metadata.len(),
        width,
        height,
        format_name,
        has_alpha,
        actions: vec![],
    })
}
