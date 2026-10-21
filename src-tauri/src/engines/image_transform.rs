use crate::output::AtomicOutput;
use image::{imageops, GenericImageView};
use std::io::Write;
use std::path::{Path, PathBuf};

pub enum TransformKind {
    Rotate90,
    Rotate180,
    Rotate270,
    FlipHorizontal,
    FlipVertical,
    Grayscale,
    ResizeScale(u32),
    ResizeDimensions(u32, u32),
    CropSquare,
    StripMetadata,
}

pub fn transform_image(input_path: &Path, transform: TransformKind) -> Result<PathBuf, String> {
    if !input_path.is_file() {
        return Err("Source file does not exist.".into());
    }

    let reader = image::ImageReader::open(input_path)
        .map_err(|e| format!("Cannot open image: {e}"))?
        .with_guessed_format()
        .map_err(|e| format!("Cannot identify image format: {e}"))?;

    let format = reader
        .format()
        .ok_or_else(|| "Could not identify image format.".to_string())?;

    let decoded = reader
        .decode()
        .map_err(|e| format!("Cannot decode image: {e}"))?;

    let (suffix, transformed) = match transform {
        TransformKind::Rotate90 => ("rotated-90", decoded.rotate90()),
        TransformKind::Rotate180 => ("rotated-180", decoded.rotate180()),
        TransformKind::Rotate270 => ("rotated-270", decoded.rotate270()),
        TransformKind::FlipHorizontal => ("flipped-h", decoded.fliph()),
        TransformKind::FlipVertical => ("flipped-v", decoded.flipv()),
        TransformKind::Grayscale => ("grayscale", decoded.grayscale()),
        TransformKind::ResizeScale(percent) => {
            let p = percent.clamp(10, 800) as f32 / 100.0;
            let (w, h) = decoded.dimensions();
            let new_w = ((w as f32 * p).round() as u32).max(1);
            let new_h = ((h as f32 * p).round() as u32).max(1);
            (
                "resized",
                decoded.resize_exact(new_w, new_h, imageops::FilterType::Lanczos3),
            )
        }
        TransformKind::ResizeDimensions(target_w, target_h) => {
            let new_w = target_w.max(1);
            let new_h = target_h.max(1);
            (
                "resized",
                decoded.resize(new_w, new_h, imageops::FilterType::Lanczos3),
            )
        }
        TransformKind::CropSquare => {
            let (w, h) = decoded.dimensions();
            let min_dim = w.min(h);
            let x = (w - min_dim) / 2;
            let y = (h - min_dim) / 2;
            ("cropped", decoded.crop_imm(x, y, min_dim, min_dim))
        }
        TransformKind::StripMetadata => ("clean", decoded),
    };

    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("image");
    let ext = input_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("png");

    let desired_name = format!("{stem}-{suffix}.{ext}");
    let desired = input_path.with_file_name(desired_name);
    let atomic_writer = AtomicOutput::new(desired)?;

    let mut file = atomic_writer.create_file()?;
    transformed
        .write_to(&mut file, format)
        .map_err(|e| format!("Failed writing transformed image: {e}"))?;
    file.flush().map_err(|e| format!("Flush failed: {e}"))?;

    atomic_writer.finish()
}
