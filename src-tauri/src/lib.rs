pub mod engines;
pub mod metadata;
pub mod models;
pub mod output;
pub mod registry;

use engines::{
    archive::{self, list_zip_contents},
    image_convert,
    image_transform::{self, TransformKind},
    pdf,
};
use metadata::{file_name, path_string};
use models::{ActionExecutionRequest, FileMetadata, OperationResult};
use output::AtomicOutput;
use registry::CapabilityRegistry;
use std::{path::{Path, PathBuf}, process::Command};

#[tauri::command]
fn inspect_file(path: String) -> Result<FileMetadata, String> {
    let mut meta = metadata::inspect_file_properties(&path)?;
    meta.actions = CapabilityRegistry::get_available_actions(&meta);
    Ok(meta)
}

#[tauri::command]
fn execute_action(req: ActionExecutionRequest) -> Result<OperationResult, String> {
    let p = PathBuf::from(&req.path);
    if !p.exists() {
        return Err("The source file no longer exists.".into());
    }

    match req.action_id.as_str() {
        "to-png" => {
            let out = image_convert::convert_image_format(&p, "png", None, None)?;
            Ok(OperationResult {
                output_paths: vec![path_string(&out)],
                message: format!("Saved {} as lossless PNG.", file_name(&out)),
                details: None,
            })
        }
        "to-jpeg" => {
            let out = image_convert::convert_image_format(
                &p,
                "jpg",
                req.quality,
                req.bg_color.as_deref(),
            )?;
            Ok(OperationResult {
                output_paths: vec![path_string(&out)],
                message: format!("Saved {} as JPEG.", file_name(&out)),
                details: None,
            })
        }
        "to-webp" => {
            let out = image_convert::convert_image_format(&p, "webp", req.quality, None)?;
            Ok(OperationResult {
                output_paths: vec![path_string(&out)],
                message: format!("Saved {} as WebP.", file_name(&out)),
                details: None,
            })
        }
        "to-bmp" => {
            let out = image_convert::convert_image_format(&p, "bmp", None, None)?;
            Ok(OperationResult {
                output_paths: vec![path_string(&out)],
                message: format!("Saved {} as BMP.", file_name(&out)),
                details: None,
            })
        }
        "to-tiff" => {
            let out = image_convert::convert_image_format(&p, "tiff", None, None)?;
            Ok(OperationResult {
                output_paths: vec![path_string(&out)],
                message: format!("Saved {} as TIFF.", file_name(&out)),
                details: None,
            })
        }
        "to-pdf" => {
            let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("document");
            let desired = p.with_file_name(format!("{stem}.pdf"));
            let atomic_writer = AtomicOutput::new(desired)?;
            pdf::image_file_to_pdf(&p, &atomic_writer.temp_path)?;
            let final_path = atomic_writer.finish()?;
            Ok(OperationResult {
                output_paths: vec![path_string(&final_path)],
                message: format!("Created PDF document {}.", file_name(&final_path)),
                details: None,
            })
        }
        "compress-image" => {
            let q = req.quality.unwrap_or(75);
            let ext = metadata::extension(&p);
            let target_fmt = if ext == "png" {
                "png"
            } else if ext == "webp" {
                "webp"
            } else {
                "jpg"
            };
            let out = image_convert::convert_image_format(&p, target_fmt, Some(q), None)?;
            Ok(OperationResult {
                output_paths: vec![path_string(&out)],
                message: format!("Saved optimized image {} (quality: {q}%).", file_name(&out)),
                details: None,
            })
        }
        "strip-metadata" => {
            let out = image_transform::transform_image(&p, TransformKind::StripMetadata)?;
            Ok(OperationResult {
                output_paths: vec![path_string(&out)],
                message: format!("Saved clean image {} with all EXIF stripped.", file_name(&out)),
                details: None,
            })
        }
        "rotate-90" => {
            let out = image_transform::transform_image(&p, TransformKind::Rotate90)?;
            Ok(OperationResult {
                output_paths: vec![path_string(&out)],
                message: format!("Rotated 90°: {}.", file_name(&out)),
                details: None,
            })
        }
        "rotate-180" => {
            let out = image_transform::transform_image(&p, TransformKind::Rotate180)?;
            Ok(OperationResult {
                output_paths: vec![path_string(&out)],
                message: format!("Rotated 180°: {}.", file_name(&out)),
                details: None,
            })
        }
        "rotate-270" => {
            let out = image_transform::transform_image(&p, TransformKind::Rotate270)?;
            Ok(OperationResult {
                output_paths: vec![path_string(&out)],
                message: format!("Rotated 270°: {}.", file_name(&out)),
                details: None,
            })
        }
        "flip-h" => {
            let out = image_transform::transform_image(&p, TransformKind::FlipHorizontal)?;
            Ok(OperationResult {
                output_paths: vec![path_string(&out)],
                message: format!("Flipped horizontally: {}.", file_name(&out)),
                details: None,
            })
        }
        "flip-v" => {
            let out = image_transform::transform_image(&p, TransformKind::FlipVertical)?;
            Ok(OperationResult {
                output_paths: vec![path_string(&out)],
                message: format!("Flipped vertically: {}.", file_name(&out)),
                details: None,
            })
        }
        "to-grayscale" => {
            let out = image_transform::transform_image(&p, TransformKind::Grayscale)?;
            Ok(OperationResult {
                output_paths: vec![path_string(&out)],
                message: format!("Converted to grayscale: {}.", file_name(&out)),
                details: None,
            })
        }
        "resize-scale" => {
            let scale = req.scale_percent.unwrap_or(50);
            let out = image_transform::transform_image(&p, TransformKind::ResizeScale(scale))?;
            Ok(OperationResult {
                output_paths: vec![path_string(&out)],
                message: format!("Resized ({scale}%): {}.", file_name(&out)),
                details: None,
            })
        }
        "crop-square" => {
            let out = image_transform::transform_image(&p, TransformKind::CropSquare)?;
            Ok(OperationResult {
                output_paths: vec![path_string(&out)],
                message: format!("Cropped 1:1 square: {}.", file_name(&out)),
                details: None,
            })
        }
        "extract-zip" => {
            let out_dir = archive::extract_zip_safe(&p)?;
            Ok(OperationResult {
                output_paths: vec![path_string(&out_dir)],
                message: format!("Extracted archive safely to {}.", file_name(&out_dir)),
                details: None,
            })
        }
        "reveal-finder" => {
            reveal_in_finder(req.path)?;
            Ok(OperationResult {
                output_paths: vec![],
                message: "Revealed in Finder.".into(),
                details: None,
            })
        }
        "copy-path" => Ok(OperationResult {
            output_paths: vec![req.path.clone()],
            message: format!("Copied path to clipboard: {}", file_name(&p)),
            details: None,
        }),
        unknown => Err(format!("Action '{unknown}' is not supported.")),
    }
}

// Retain compatibility with existing commands
#[tauri::command]
fn convert_image(
    path: String,
    format: String,
    quality: Option<u8>,
) -> Result<OperationResult, String> {
    let action_id = match format.to_ascii_lowercase().as_str() {
        "png" => "to-png",
        "jpg" | "jpeg" => "to-jpeg",
        "webp" => "to-webp",
        "bmp" => "to-bmp",
        "tiff" => "to-tiff",
        "pdf" => "to-pdf",
        _ => return Err("Unsupported format".into()),
    };
    execute_action(ActionExecutionRequest {
        path,
        action_id: action_id.into(),
        quality,
        scale_percent: None,
        target_width: None,
        target_height: None,
        bg_color: None,
    })
}

#[tauri::command]
fn extract_zip(path: String) -> Result<OperationResult, String> {
    execute_action(ActionExecutionRequest {
        path,
        action_id: "extract-zip".into(),
        quality: None,
        scale_percent: None,
        target_width: None,
        target_height: None,
        bg_color: None,
    })
}

#[tauri::command]
fn list_zip_entries(path: String) -> Result<Vec<String>, String> {
    list_zip_contents(Path::new(&path))
}

#[tauri::command]
fn reveal_in_finder(path: String) -> Result<(), String> {
    let p = PathBuf::from(path);
    if !p.exists() {
        return Err("The file no longer exists.".into());
    }
    #[cfg(target_os = "macos")]
    let status = Command::new("open").arg("-R").arg(&p).status();
    #[cfg(target_os = "windows")]
    let status = Command::new("explorer")
        .arg(format!("/select,{}", p.display()))
        .status();
    #[cfg(all(unix, not(target_os = "macos")))]
    let status = Command::new("xdg-open").arg(p.parent().unwrap_or(&p)).status();

    status
        .map_err(|e| format!("Could not reveal file: {e}"))?
        .success()
        .then_some(())
        .ok_or_else(|| "Could not open file location in Finder.".to_string())
}

#[tauri::command]
fn close_window(window: tauri::WebviewWindow) -> Result<(), String> {
    window.close().map_err(|e| e.to_string())
}

#[tauri::command]
fn toggle_always_on_top(window: tauri::WebviewWindow, enabled: bool) -> Result<bool, String> {
    window.set_always_on_top(enabled).map_err(|e| e.to_string())?;
    Ok(enabled)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            inspect_file,
            execute_action,
            convert_image,
            extract_zip,
            list_zip_entries,
            reveal_in_finder,
            close_window,
            toggle_always_on_top
        ])
        .run(tauri::generate_context!())
        .expect("error while running ORBIT");
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use image::{Rgb, RgbImage};
    use std::fs;

    fn create_temp_test_image(ext: &str) -> PathBuf {
        let mut img = RgbImage::new(100, 100);
        for x in 0..100 {
            for y in 0..100 {
                img.put_pixel(x, y, Rgb([(x * 2) as u8, (y * 2) as u8, 128]));
            }
        }
        use std::sync::atomic::{AtomicU64, Ordering};
        static TEST_COUNTER: AtomicU64 = AtomicU64::new(100);
        let count = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let temp_dir = std::env::temp_dir();
        let filename = format!("orbit_test_{}_{}_{}.{}", std::process::id(), count, ext, ext);
        let path = temp_dir.join(filename);
        img.save(&path).expect("Failed to create test image");
        path
    }

    #[test]
    fn test_jpeg_to_png_conversion() {
        let jpg_path = create_temp_test_image("jpg");
        let req = ActionExecutionRequest {
            path: jpg_path.to_string_lossy().to_string(),
            action_id: "to-png".into(),
            quality: None,
            scale_percent: None,
            target_width: None,
            target_height: None,
            bg_color: None,
        };
        let res = execute_action(req).expect("Failed to convert JPEG to PNG");
        let out_path = PathBuf::from(&res.output_paths[0]);
        assert!(out_path.exists());
        assert_eq!(out_path.extension().unwrap(), "png");
        let _ = fs::remove_file(jpg_path);
        let _ = fs::remove_file(out_path);
    }

    #[test]
    fn test_image_to_pdf_generation() {
        let jpg_path = create_temp_test_image("jpg");
        let req = ActionExecutionRequest {
            path: jpg_path.to_string_lossy().to_string(),
            action_id: "to-pdf".into(),
            quality: None,
            scale_percent: None,
            target_width: None,
            target_height: None,
            bg_color: None,
        };
        let res = execute_action(req).expect("Failed to convert image to PDF");
        let out_path = PathBuf::from(&res.output_paths[0]);
        assert!(out_path.exists());
        assert_eq!(out_path.extension().unwrap(), "pdf");

        // Verify PDF magic header
        let bytes = fs::read(&out_path).expect("Read PDF bytes");
        assert!(bytes.starts_with(b"%PDF-1.4"));

        let _ = fs::remove_file(jpg_path);
        let _ = fs::remove_file(out_path);
    }

    #[test]
    fn test_image_transform_rotate_and_grayscale() {
        let png_path = create_temp_test_image("png");
        let req = ActionExecutionRequest {
            path: png_path.to_string_lossy().to_string(),
            action_id: "to-grayscale".into(),
            quality: None,
            scale_percent: None,
            target_width: None,
            target_height: None,
            bg_color: None,
        };
        let res = execute_action(req).expect("Failed to grayscale");
        let out_path = PathBuf::from(&res.output_paths[0]);
        assert!(out_path.exists());

        let _ = fs::remove_file(png_path);
        let _ = fs::remove_file(out_path);
    }

    #[test]
    fn test_output_safety_no_overwrite() {
        let jpg_path = create_temp_test_image("jpg");
        let req = ActionExecutionRequest {
            path: jpg_path.to_string_lossy().to_string(),
            action_id: "to-png".into(),
            quality: None,
            scale_percent: None,
            target_width: None,
            target_height: None,
            bg_color: None,
        };
        let res1 = execute_action(req.clone()).expect("First conversion");
        let res2 = execute_action(req).expect("Second conversion should not overwrite");

        assert_ne!(res1.output_paths[0], res2.output_paths[0]);
        assert!(Path::new(&res1.output_paths[0]).exists());
        assert!(Path::new(&res2.output_paths[0]).exists());

        let _ = fs::remove_file(jpg_path);
        let _ = fs::remove_file(&res1.output_paths[0]);
        let _ = fs::remove_file(&res2.output_paths[0]);
    }

    #[test]
    fn test_webp_conversion_and_transparency() {
        use image::{Rgba, RgbaImage};
        let mut img = RgbaImage::new(50, 50);
        for x in 0..50 {
            for y in 0..50 {
                // Semi-transparent colored pixel
                img.put_pixel(x, y, Rgba([200, 100, 50, 128]));
            }
        }
        let temp_dir = std::env::temp_dir();
        let png_path = temp_dir.join(format!("orbit_test_trans_{}.png", std::process::id()));
        img.save(&png_path).expect("Save test transparent PNG");

        // Convert PNG with alpha to JPEG (tests alpha flattening to white background)
        let req_jpg = ActionExecutionRequest {
            path: png_path.to_string_lossy().to_string(),
            action_id: "to-jpeg".into(),
            quality: Some(90),
            scale_percent: None,
            target_width: None,
            target_height: None,
            bg_color: Some("#ffffff".into()),
        };
        let res_jpg = execute_action(req_jpg).expect("Convert PNG with alpha to JPEG");
        assert!(Path::new(&res_jpg.output_paths[0]).exists());

        // Convert PNG with alpha to WebP
        let req_webp = ActionExecutionRequest {
            path: png_path.to_string_lossy().to_string(),
            action_id: "to-webp".into(),
            quality: None,
            scale_percent: None,
            target_width: None,
            target_height: None,
            bg_color: None,
        };
        let res_webp = execute_action(req_webp).expect("Convert PNG to WebP");
        assert!(Path::new(&res_webp.output_paths[0]).exists());

        let _ = fs::remove_file(png_path);
        let _ = fs::remove_file(&res_jpg.output_paths[0]);
        let _ = fs::remove_file(&res_webp.output_paths[0]);
    }

    #[test]
    fn test_invalid_input_file_handling() {
        let req = ActionExecutionRequest {
            path: "/path/that/does/not/exist.jpg".into(),
            action_id: "to-png".into(),
            quality: None,
            scale_percent: None,
            target_width: None,
            target_height: None,
            bg_color: None,
        };
        let result = execute_action(req);
        assert!(result.is_err());
    }

    #[test]
    fn test_unsupported_action_handling() {
        let jpg_path = create_temp_test_image("jpg");
        let req = ActionExecutionRequest {
            path: jpg_path.to_string_lossy().to_string(),
            action_id: "unsupported-magic-action".into(),
            quality: None,
            scale_percent: None,
            target_width: None,
            target_height: None,
            bg_color: None,
        };
        let result = execute_action(req);
        assert!(result.is_err());
        let _ = fs::remove_file(jpg_path);
    }
}

