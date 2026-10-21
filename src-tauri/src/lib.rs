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
use models::{ActionExecutionRequest, FileKind, FileMetadata, OperationResult, SelectionMetadata};
use output::AtomicOutput;
use registry::CapabilityRegistry;
use std::{
    path::{Path, PathBuf},
    process::Command,
};

#[tauri::command]
fn inspect_file(path: String) -> Result<FileMetadata, String> {
    let mut meta = metadata::inspect_file_properties(&path)?;
    meta.actions = CapabilityRegistry::get_available_actions(&meta);
    Ok(meta)
}

#[tauri::command]
fn inspect_selection(paths: Vec<String>) -> Result<SelectionMetadata, String> {
    if paths.is_empty() {
        return Err("No files dropped or selected.".into());
    }

    let mut files = Vec::with_capacity(paths.len());
    let mut total_size = 0u64;

    for p in &paths {
        let meta = metadata::inspect_file_properties(p)?;
        total_size = total_size.saturating_add(meta.size);
        files.push(meta);
    }

    let is_multi = files.len() > 1;
    let (summary_label, actions) = if is_multi {
        let all_images = files.iter().all(|f| f.kind == FileKind::Image);
        let label = if all_images {
            format!("{} images selected", files.len())
        } else {
            format!("{} files selected", files.len())
        };
        let acts = CapabilityRegistry::get_multi_selection_actions(&files);
        (label, acts)
    } else {
        let single_file = &mut files[0];
        let acts = CapabilityRegistry::get_available_actions(single_file);
        let name = single_file.name.clone();
        single_file.actions = acts.clone();
        (name, acts)
    };

    Ok(SelectionMetadata {
        files,
        is_multi,
        total_size,
        summary_label,
        actions,
    })
}

#[tauri::command]
fn execute_action(req: ActionExecutionRequest) -> Result<OperationResult, String> {
    let input_paths = req.get_paths();
    if input_paths.is_empty() {
        return Err("No input files specified.".into());
    }

    // Verify all input files exist before running any operation
    for path_str in &input_paths {
        let p = Path::new(path_str);
        if !p.exists() {
            return Err(format!("The input file '{}' does not exist.", file_name(p)));
        }
    }

    let first_path = PathBuf::from(&input_paths[0]);

    match req.action_id.as_str() {
        "combine-pdf" => {
            let parent_dir = first_path.parent().unwrap_or_else(|| Path::new("."));
            let desired = output::unique_path(&parent_dir.join("Combined.pdf"));
            let atomic_writer = AtomicOutput::new(desired)?;

            let path_refs: Vec<&Path> = input_paths.iter().map(Path::new).collect();
            pdf::combine_files_to_pdf(&path_refs, &atomic_writer.temp_path)?;
            let final_path = atomic_writer.finish()?;

            Ok(OperationResult {
                output_paths: vec![path_string(&final_path)],
                message: format!(
                    "Combined {} file(s) into {}.",
                    input_paths.len(),
                    file_name(&final_path)
                ),
                details: None,
            })
        }

        "pdf-to-png" | "batch-pdf-to-png" => {
            let mut all_outputs = Vec::new();
            for p_str in &input_paths {
                let p = Path::new(p_str);
                let parent_dir = p.parent().unwrap_or_else(|| Path::new("."));
                let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("document");
                let page_outputs = pdf::pdf_to_images(p, parent_dir, stem, "png")?;
                for out_p in page_outputs {
                    all_outputs.push(path_string(&out_p));
                }
            }
            let msg = if all_outputs.len() == 1 {
                format!("Converted PDF to {}.", file_name(Path::new(&all_outputs[0])))
            } else {
                format!("Generated {} PNG image(s) from PDF.", all_outputs.len())
            };
            Ok(OperationResult {
                output_paths: all_outputs,
                message: msg,
                details: None,
            })
        }

        "pdf-to-jpeg" | "batch-pdf-to-jpeg" => {
            let mut all_outputs = Vec::new();
            for p_str in &input_paths {
                let p = Path::new(p_str);
                let parent_dir = p.parent().unwrap_or_else(|| Path::new("."));
                let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("document");
                let page_outputs = pdf::pdf_to_images(p, parent_dir, stem, "jpeg")?;
                for out_p in page_outputs {
                    all_outputs.push(path_string(&out_p));
                }
            }
            let msg = if all_outputs.len() == 1 {
                format!("Converted PDF to {}.", file_name(Path::new(&all_outputs[0])))
            } else {
                format!("Generated {} JPEG image(s) from PDF.", all_outputs.len())
            };
            Ok(OperationResult {
                output_paths: all_outputs,
                message: msg,
                details: None,
            })
        }

        "pdf-extract-text" | "batch-pdf-extract-text" => {
            let mut all_outputs = Vec::new();
            for p_str in &input_paths {
                let p = Path::new(p_str);
                let parent_dir = p.parent().unwrap_or_else(|| Path::new("."));
                let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("document");
                let desired = parent_dir.join(format!("{stem}.txt"));
                let final_path = output::unique_path(&desired);
                pdf::pdf_extract_text(p, &final_path)?;
                all_outputs.push(path_string(&final_path));
            }
            let msg = if all_outputs.len() == 1 {
                format!("Extracted text to {}.", file_name(Path::new(&all_outputs[0])))
            } else {
                format!("Extracted text from {} PDF file(s).", all_outputs.len())
            };
            Ok(OperationResult {
                output_paths: all_outputs,
                message: msg,
                details: None,
            })
        }

        "pdf-split-pages" => {
            let mut all_outputs = Vec::new();
            for p_str in &input_paths {
                let p = Path::new(p_str);
                let parent_dir = p.parent().unwrap_or_else(|| Path::new("."));
                let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("document");
                let split_pages = pdf::pdf_split_pages(p, parent_dir, stem)?;
                for sp in split_pages {
                    all_outputs.push(path_string(&sp));
                }
            }
            let msg = format!("Split PDF into {} page file(s).", all_outputs.len());
            Ok(OperationResult {
                output_paths: all_outputs,
                message: msg,
                details: None,
            })
        }

        "pdf-info" => {
            let info = pdf::pdf_info(&first_path)?;
            Ok(OperationResult {
                output_paths: vec![path_string(&first_path)],
                message: format!("{}: {}", file_name(&first_path), info),
                details: Some(info),
            })
        }

        "to-pdf" => {
            if input_paths.len() > 1 {
                // If multiple images are provided to to-pdf, combine them
                let parent_dir = first_path.parent().unwrap_or_else(|| Path::new("."));
                let desired = output::unique_path(&parent_dir.join("Combined.pdf"));
                let atomic_writer = AtomicOutput::new(desired)?;

                let path_refs: Vec<&Path> = input_paths.iter().map(Path::new).collect();
                pdf::images_files_to_pdf(&path_refs, &atomic_writer.temp_path)?;
                let final_path = atomic_writer.finish()?;

                return Ok(OperationResult {
                    output_paths: vec![path_string(&final_path)],
                    message: format!(
                        "Combined {} images into {}.",
                        input_paths.len(),
                        file_name(&final_path)
                    ),
                    details: None,
                });
            }

            let stem = first_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("document");
            let desired = first_path.with_file_name(format!("{stem}.pdf"));
            let atomic_writer = AtomicOutput::new(desired)?;
            pdf::image_file_to_pdf(&first_path, &atomic_writer.temp_path)?;
            let final_path = atomic_writer.finish()?;
            Ok(OperationResult {
                output_paths: vec![path_string(&final_path)],
                message: format!("Created PDF document {}.", file_name(&final_path)),
                details: None,
            })
        }

        "to-png" | "batch-to-png" => {
            let mut outputs = Vec::new();
            for p_str in &input_paths {
                let p = Path::new(p_str);
                let is_pdf = p
                    .extension()
                    .and_then(|s| s.to_str())
                    .map(|ext| ext.eq_ignore_ascii_case("pdf"))
                    .unwrap_or(false)
                    || metadata::sniff_format(p).map(|(k, _, _)| k == FileKind::Pdf).unwrap_or(false);

                if is_pdf {
                    let parent_dir = p.parent().unwrap_or_else(|| Path::new("."));
                    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("document");
                    let page_outputs = pdf::pdf_to_images(p, parent_dir, stem, "png")?;
                    for out_p in page_outputs {
                        outputs.push(path_string(&out_p));
                    }
                } else {
                    let out = image_convert::convert_image_format(p, "png", None, None)?;
                    outputs.push(path_string(&out));
                }
            }
            let msg = if outputs.len() == 1 {
                format!("Saved {} as lossless PNG.", file_name(Path::new(&outputs[0])))
            } else {
                format!("Converted {} file(s) to PNG.", outputs.len())
            };
            Ok(OperationResult {
                output_paths: outputs,
                message: msg,
                details: None,
            })
        }

        "to-jpeg" | "batch-to-jpeg" => {
            let mut outputs = Vec::new();
            for p_str in &input_paths {
                let p = Path::new(p_str);
                let is_pdf = p
                    .extension()
                    .and_then(|s| s.to_str())
                    .map(|ext| ext.eq_ignore_ascii_case("pdf"))
                    .unwrap_or(false)
                    || metadata::sniff_format(p).map(|(k, _, _)| k == FileKind::Pdf).unwrap_or(false);

                if is_pdf {
                    let parent_dir = p.parent().unwrap_or_else(|| Path::new("."));
                    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("document");
                    let page_outputs = pdf::pdf_to_images(p, parent_dir, stem, "jpeg")?;
                    for out_p in page_outputs {
                        outputs.push(path_string(&out_p));
                    }
                } else {
                    let out = image_convert::convert_image_format(
                        p,
                        "jpg",
                        req.quality,
                        req.bg_color.as_deref(),
                    )?;
                    outputs.push(path_string(&out));
                }
            }
            let msg = if outputs.len() == 1 {
                format!("Saved {} as JPEG.", file_name(Path::new(&outputs[0])))
            } else {
                format!("Converted {} file(s) to JPEG.", outputs.len())
            };
            Ok(OperationResult {
                output_paths: outputs,
                message: msg,
                details: None,
            })
        }

        "to-webp" | "batch-to-webp" => {
            let mut outputs = Vec::new();
            for p_str in &input_paths {
                let p = Path::new(p_str);
                let out = image_convert::convert_image_format(p, "webp", req.quality, None)?;
                outputs.push(path_string(&out));
            }
            let msg = if outputs.len() == 1 {
                format!("Saved {} as WebP.", file_name(Path::new(&outputs[0])))
            } else {
                format!("Converted {} images to WebP.", outputs.len())
            };
            Ok(OperationResult {
                output_paths: outputs,
                message: msg,
                details: None,
            })
        }

        "to-bmp" => {
            let out = image_convert::convert_image_format(&first_path, "bmp", None, None)?;
            Ok(OperationResult {
                output_paths: vec![path_string(&out)],
                message: format!("Saved {} as BMP.", file_name(&out)),
                details: None,
            })
        }

        "to-tiff" => {
            let out = image_convert::convert_image_format(&first_path, "tiff", None, None)?;
            Ok(OperationResult {
                output_paths: vec![path_string(&out)],
                message: format!("Saved {} as TIFF.", file_name(&out)),
                details: None,
            })
        }

        "compress-image" => {
            let q = req.quality.unwrap_or(75);
            let ext = metadata::extension(&first_path);
            let target_fmt = if ext == "png" {
                "png"
            } else if ext == "webp" {
                "webp"
            } else {
                "jpg"
            };
            let out = image_convert::convert_image_format(&first_path, target_fmt, Some(q), None)?;
            Ok(OperationResult {
                output_paths: vec![path_string(&out)],
                message: format!("Saved optimized image {} (quality: {q}%).", file_name(&out)),
                details: None,
            })
        }

        "strip-metadata" => {
            let out = image_transform::transform_image(&first_path, TransformKind::StripMetadata)?;
            Ok(OperationResult {
                output_paths: vec![path_string(&out)],
                message: format!("Saved clean image {} with all EXIF stripped.", file_name(&out)),
                details: None,
            })
        }

        "rotate-90" => {
            let out = image_transform::transform_image(&first_path, TransformKind::Rotate90)?;
            Ok(OperationResult {
                output_paths: vec![path_string(&out)],
                message: format!("Rotated 90°: {}.", file_name(&out)),
                details: None,
            })
        }

        "rotate-180" => {
            let out = image_transform::transform_image(&first_path, TransformKind::Rotate180)?;
            Ok(OperationResult {
                output_paths: vec![path_string(&out)],
                message: format!("Rotated 180°: {}.", file_name(&out)),
                details: None,
            })
        }

        "rotate-270" => {
            let out = image_transform::transform_image(&first_path, TransformKind::Rotate270)?;
            Ok(OperationResult {
                output_paths: vec![path_string(&out)],
                message: format!("Rotated 270°: {}.", file_name(&out)),
                details: None,
            })
        }

        "flip-h" => {
            let out = image_transform::transform_image(&first_path, TransformKind::FlipHorizontal)?;
            Ok(OperationResult {
                output_paths: vec![path_string(&out)],
                message: format!("Flipped horizontally: {}.", file_name(&out)),
                details: None,
            })
        }

        "flip-v" => {
            let out = image_transform::transform_image(&first_path, TransformKind::FlipVertical)?;
            Ok(OperationResult {
                output_paths: vec![path_string(&out)],
                message: format!("Flipped vertically: {}.", file_name(&out)),
                details: None,
            })
        }

        "to-grayscale" => {
            let out = image_transform::transform_image(&first_path, TransformKind::Grayscale)?;
            Ok(OperationResult {
                output_paths: vec![path_string(&out)],
                message: format!("Converted to grayscale: {}.", file_name(&out)),
                details: None,
            })
        }

        "resize-scale" => {
            let scale = req.scale_percent.unwrap_or(50);
            let out = image_transform::transform_image(&first_path, TransformKind::ResizeScale(scale))?;
            Ok(OperationResult {
                output_paths: vec![path_string(&out)],
                message: format!("Resized ({scale}%): {}.", file_name(&out)),
                details: None,
            })
        }

        "crop-square" => {
            let out = image_transform::transform_image(&first_path, TransformKind::CropSquare)?;
            Ok(OperationResult {
                output_paths: vec![path_string(&out)],
                message: format!("Cropped 1:1 square: {}.", file_name(&out)),
                details: None,
            })
        }

        "extract-zip" => {
            let out_dir = archive::extract_zip_safe(&first_path)?;
            Ok(OperationResult {
                output_paths: vec![path_string(&out_dir)],
                message: format!("Extracted archive safely to {}.", file_name(&out_dir)),
                details: None,
            })
        }

        "reveal-finder" => {
            reveal_in_finder(path_string(&first_path))?;
            Ok(OperationResult {
                output_paths: vec![],
                message: "Revealed in Finder.".into(),
                details: None,
            })
        }

        "copy-path" => {
            let joined = input_paths.join("\n");
            Ok(OperationResult {
                output_paths: input_paths.clone(),
                message: if input_paths.len() == 1 {
                    format!("Copied path to clipboard: {}", file_name(&first_path))
                } else {
                    format!("Copied {} file paths to clipboard.", input_paths.len())
                },
                details: Some(joined),
            })
        }

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
        path: Some(path),
        paths: None,
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
        path: Some(path),
        paths: None,
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
            inspect_selection,
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
    use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(100);

    fn create_temp_test_image_with_dims(ext: &str, width: u32, height: u32) -> PathBuf {
        let mut img = RgbImage::new(width, height);
        for x in 0..width {
            for y in 0..height {
                img.put_pixel(
                    x,
                    y,
                    Rgb([(x % 255) as u8, (y % 255) as u8, 128]),
                );
            }
        }
        let count = TEST_COUNTER.fetch_add(1, Relaxed);
        let temp_dir = std::env::temp_dir();
        let filename = format!("orbit_test_{}_{}_{}.{}", std::process::id(), count, ext, ext);
        let path = temp_dir.join(filename);
        img.save(&path).expect("Failed to create test image");
        path
    }

    fn create_temp_test_image(ext: &str) -> PathBuf {
        create_temp_test_image_with_dims(ext, 100, 100)
    }

    // Helper to inspect the generated PDF structure
    fn inspect_pdf_metadata(pdf_path: &Path) -> (usize, Vec<(f64, f64)>) {
        let content = fs::read(pdf_path).expect("Read PDF content");
        let text = String::from_utf8_lossy(&content);

        // Find /Type /Pages ... /Count N
        let count = if let Some(idx) = text.find("/Type /Pages") {
            let substr = &text[idx..std::cmp::min(idx + 120, text.len())];
            if let Some(c_idx) = substr.find("/Count ") {
                let num_str: String = substr[c_idx + 7..]
                    .chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect();
                num_str.parse::<usize>().unwrap_or(0)
            } else {
                0
            }
        } else {
            0
        };

        // Extract MediaBoxes: /MediaBox [0 0 w.00 h.00]
        let mut boxes = Vec::new();
        let mut pos = 0;
        while let Some(mb_idx) = text[pos..].find("/MediaBox [0 0 ") {
            let start = pos + mb_idx + 15;
            let rest = &text[start..];
            if let Some(end_bracket) = rest.find(']') {
                let dims_str = &rest[..end_bracket];
                let parts: Vec<&str> = dims_str.split_whitespace().collect();
                if parts.len() == 2 {
                    if let (Ok(w), Ok(h)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>()) {
                        boxes.push((w, h));
                    }
                }
            }
            pos = start;
        }

        (count, boxes)
    }

    #[test]
    fn test_combine_two_images_into_one_pdf() {
        let img1 = create_temp_test_image_with_dims("jpg", 150, 100);
        let img2 = create_temp_test_image_with_dims("png", 80, 120);

        let req = ActionExecutionRequest {
            path: None,
            paths: Some(vec![
                img1.to_string_lossy().to_string(),
                img2.to_string_lossy().to_string(),
            ]),
            action_id: "combine-pdf".into(),
            quality: None,
            scale_percent: None,
            target_width: None,
            target_height: None,
            bg_color: None,
        };

        let res = execute_action(req).expect("Failed to combine two images into PDF");
        let out_path = PathBuf::from(&res.output_paths[0]);
        assert!(out_path.exists());
        assert_eq!(out_path.extension().unwrap(), "pdf");

        // Inspect PDF structure
        let (count, boxes) = inspect_pdf_metadata(&out_path);
        assert_eq!(count, 2, "PDF must contain exactly 2 pages");
        assert_eq!(boxes.len(), 2, "PDF must define 2 MediaBoxes");
        // Verify page order matches input order
        assert_eq!(boxes[0], (150.0, 100.0), "Page 1 dimensions must match img1");
        assert_eq!(boxes[1], (80.0, 120.0), "Page 2 dimensions must match img2");

        let _ = fs::remove_file(img1);
        let _ = fs::remove_file(img2);
        let _ = fs::remove_file(out_path);
    }

    #[test]
    fn test_combine_three_images_different_formats_and_dimensions() {
        let img_jpg = create_temp_test_image_with_dims("jpg", 200, 120);
        let img_png = create_temp_test_image_with_dims("png", 100, 300);
        let img_webp = create_temp_test_image_with_dims("webp", 150, 150);

        let req = ActionExecutionRequest {
            path: None,
            paths: Some(vec![
                img_jpg.to_string_lossy().to_string(),
                img_png.to_string_lossy().to_string(),
                img_webp.to_string_lossy().to_string(),
            ]),
            action_id: "combine-pdf".into(),
            quality: None,
            scale_percent: None,
            target_width: None,
            target_height: None,
            bg_color: None,
        };

        let res = execute_action(req).expect("Failed to combine 3 images");
        let out_path = PathBuf::from(&res.output_paths[0]);
        assert!(out_path.exists());

        let (count, boxes) = inspect_pdf_metadata(&out_path);
        assert_eq!(count, 3, "PDF must contain 3 pages");
        assert_eq!(boxes.len(), 3);
        assert_eq!(boxes[0], (200.0, 120.0), "Page 1 must be landscape JPG");
        assert_eq!(boxes[1], (100.0, 300.0), "Page 2 must be portrait PNG");
        assert_eq!(boxes[2], (150.0, 150.0), "Page 3 must be square WebP");

        let _ = fs::remove_file(img_jpg);
        let _ = fs::remove_file(img_png);
        let _ = fs::remove_file(img_webp);
        let _ = fs::remove_file(out_path);
    }

    #[test]
    fn test_single_image_to_pdf_generation() {
        let jpg_path = create_temp_test_image("jpg");
        let req = ActionExecutionRequest {
            path: Some(jpg_path.to_string_lossy().to_string()),
            paths: None,
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

        let (count, _) = inspect_pdf_metadata(&out_path);
        assert_eq!(count, 1, "Single image must produce a 1-page PDF");

        let _ = fs::remove_file(jpg_path);
        let _ = fs::remove_file(out_path);
    }

    #[test]
    fn test_combine_invalid_image_produces_error_and_no_partial_file() {
        let valid_img = create_temp_test_image("jpg");
        let invalid_path = valid_img.with_file_name("orbit_non_existent_image_12345.png");

        let req = ActionExecutionRequest {
            path: None,
            paths: Some(vec![
                valid_img.to_string_lossy().to_string(),
                invalid_path.to_string_lossy().to_string(),
            ]),
            action_id: "combine-pdf".into(),
            quality: None,
            scale_percent: None,
            target_width: None,
            target_height: None,
            bg_color: None,
        };

        let result = execute_action(req);
        assert!(result.is_err(), "Must return error for missing image");
        let err_msg = result.unwrap_err();
        assert!(
            err_msg.contains("non_existent") || err_msg.contains("does not exist"),
            "Error must specify the failing image"
        );

        // Verify no Combined.pdf was created
        let parent = valid_img.parent().unwrap();
        let should_not_exist = parent.join("Combined.pdf");
        if should_not_exist.exists() {
            // make sure it wasn't created by this failed test
            let _ = fs::remove_file(&should_not_exist);
        }

        let _ = fs::remove_file(valid_img);
    }

    #[test]
    fn test_combine_pdf_does_not_overwrite_existing() {
        let test_dir = std::env::temp_dir().join(format!("orbit_test_no_overwrite_{}_{}", std::process::id(), TEST_COUNTER.fetch_add(1, Relaxed)));
        let _ = fs::create_dir_all(&test_dir);

        let img1 = test_dir.join("img1.jpg");
        let img2 = test_dir.join("img2.png");
        let mut raw_img = RgbImage::new(40, 40);
        for x in 0..40 {
            for y in 0..40 {
                raw_img.put_pixel(x, y, Rgb([200, 100, 50]));
            }
        }
        raw_img.save(&img1).unwrap();
        raw_img.save(&img2).unwrap();

        let req = ActionExecutionRequest {
            path: None,
            paths: Some(vec![
                img1.to_string_lossy().to_string(),
                img2.to_string_lossy().to_string(),
            ]),
            action_id: "combine-pdf".into(),
            quality: None,
            scale_percent: None,
            target_width: None,
            target_height: None,
            bg_color: None,
        };

        let res1 = execute_action(req.clone()).expect("First combine");
        let res2 = execute_action(req).expect("Second combine should not overwrite");

        assert_ne!(res1.output_paths[0], res2.output_paths[0]);
        assert!(Path::new(&res1.output_paths[0]).exists());
        assert!(Path::new(&res2.output_paths[0]).exists());
        assert!(res2.output_paths[0].contains("(2)"));

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_jpeg_to_png_conversion() {
        let jpg_path = create_temp_test_image("jpg");
        let req = ActionExecutionRequest {
            path: Some(jpg_path.to_string_lossy().to_string()),
            paths: None,
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
    fn test_image_transform_rotate_and_grayscale() {
        let png_path = create_temp_test_image("png");
        let req = ActionExecutionRequest {
            path: Some(png_path.to_string_lossy().to_string()),
            paths: None,
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
            path: Some(jpg_path.to_string_lossy().to_string()),
            paths: None,
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
                img.put_pixel(x, y, Rgba([200, 100, 50, 128]));
            }
        }
        let temp_dir = std::env::temp_dir();
        let png_path = temp_dir.join(format!("orbit_test_trans_{}.png", std::process::id()));
        img.save(&png_path).expect("Save test transparent PNG");

        let req_jpg = ActionExecutionRequest {
            path: Some(png_path.to_string_lossy().to_string()),
            paths: None,
            action_id: "to-jpeg".into(),
            quality: Some(90),
            scale_percent: None,
            target_width: None,
            target_height: None,
            bg_color: Some("#ffffff".into()),
        };
        let res_jpg = execute_action(req_jpg).expect("Convert PNG with alpha to JPEG");
        assert!(Path::new(&res_jpg.output_paths[0]).exists());

        let req_webp = ActionExecutionRequest {
            path: Some(png_path.to_string_lossy().to_string()),
            paths: None,
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
            path: Some("/path/that/does/not/exist.jpg".into()),
            paths: None,
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
            path: Some(jpg_path.to_string_lossy().to_string()),
            paths: None,
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

    #[test]
    fn test_pdf_to_png_conversion() {
        let png_img = create_temp_test_image("png");
        let parent = png_img.parent().unwrap();
        let pdf_path = parent.join(format!("test_conv_{}.pdf", TEST_COUNTER.fetch_add(1, Relaxed)));

        // Create PDF from image
        pdf::image_file_to_pdf(&png_img, &pdf_path).expect("generate source PDF");

        // Execute pdf-to-png
        let req = ActionExecutionRequest {
            path: Some(pdf_path.to_string_lossy().to_string()),
            paths: None,
            action_id: "pdf-to-png".into(),
            quality: None,
            scale_percent: None,
            target_width: None,
            target_height: None,
            bg_color: None,
        };
        let res = execute_action(req).expect("pdf-to-png execution");
        assert!(!res.output_paths.is_empty());
        assert!(Path::new(&res.output_paths[0]).exists());

        let _ = fs::remove_file(png_img);
        let _ = fs::remove_file(pdf_path);
        for p in &res.output_paths {
            let _ = fs::remove_file(p);
        }
    }

    #[test]
    fn test_pdf_to_jpeg_conversion() {
        let png_img = create_temp_test_image("png");
        let parent = png_img.parent().unwrap();
        let pdf_path = parent.join(format!("test_conv_jpg_{}.pdf", TEST_COUNTER.fetch_add(1, Relaxed)));

        // Create PDF from image
        pdf::image_file_to_pdf(&png_img, &pdf_path).expect("generate source PDF");

        // Execute pdf-to-jpeg
        let req = ActionExecutionRequest {
            path: Some(pdf_path.to_string_lossy().to_string()),
            paths: None,
            action_id: "pdf-to-jpeg".into(),
            quality: None,
            scale_percent: None,
            target_width: None,
            target_height: None,
            bg_color: None,
        };
        let res = execute_action(req).expect("pdf-to-jpeg execution");
        assert!(!res.output_paths.is_empty());
        let out = Path::new(&res.output_paths[0]);
        assert!(out.exists());
        assert_eq!(out.extension().and_then(|s| s.to_str()), Some("jpg"));

        // Also test executing "to-jpeg" on this PDF
        let req2 = ActionExecutionRequest {
            path: Some(pdf_path.to_string_lossy().to_string()),
            paths: None,
            action_id: "to-jpeg".into(),
            quality: Some(90),
            scale_percent: None,
            target_width: None,
            target_height: None,
            bg_color: None,
        };
        let res2 = execute_action(req2).expect("to-jpeg with pdf input execution");
        assert!(!res2.output_paths.is_empty());
        let out2 = Path::new(&res2.output_paths[0]);
        assert!(out2.exists());

        let _ = fs::remove_file(png_img);
        let _ = fs::remove_file(pdf_path);
        for p in &res.output_paths {
            let _ = fs::remove_file(p);
        }
        for p in &res2.output_paths {
            let _ = fs::remove_file(p);
        }
    }

    #[test]
    fn test_combine_mixed_images_and_pdf() {
        let img1 = create_temp_test_image("jpg");
        let img2 = create_temp_test_image("png");
        let parent = img1.parent().unwrap();
        let pdf1 = parent.join(format!("mix_doc_{}.pdf", TEST_COUNTER.fetch_add(1, Relaxed)));

        // Create PDF from img2
        pdf::image_file_to_pdf(&img2, &pdf1).expect("generate doc PDF");

        // Combine img1 and pdf1 into one combined PDF
        let req = ActionExecutionRequest {
            path: None,
            paths: Some(vec![
                img1.to_string_lossy().to_string(),
                pdf1.to_string_lossy().to_string(),
            ]),
            action_id: "combine-pdf".into(),
            quality: None,
            scale_percent: None,
            target_width: None,
            target_height: None,
            bg_color: None,
        };
        let res = execute_action(req).expect("combine mixed images and PDF");
        assert_eq!(res.output_paths.len(), 1);
        let combined_path = Path::new(&res.output_paths[0]);
        assert!(combined_path.exists());

        // Verify with lopdf that the resulting PDF has 2 pages
        let doc = lopdf::Document::load(combined_path).expect("load combined PDF");
        assert_eq!(doc.get_pages().len(), 2);

        let _ = fs::remove_file(img1);
        let _ = fs::remove_file(img2);
        let _ = fs::remove_file(pdf1);
        let _ = fs::remove_file(combined_path);
    }
}

