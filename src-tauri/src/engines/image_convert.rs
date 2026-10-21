use crate::output::AtomicOutput;
use image::codecs::jpeg::JpegEncoder;
use image::{ExtendedColorType, ImageEncoder, ImageFormat, Rgb, RgbaImage};
use std::io::Write;
use std::path::{Path, PathBuf};

fn parse_hex_color(hex: Option<&str>) -> Rgb<u8> {
    if let Some(h) = hex {
        let clean = h.trim_start_matches('#');
        if clean.len() == 6 {
            if let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&clean[0..2], 16),
                u8::from_str_radix(&clean[2..4], 16),
                u8::from_str_radix(&clean[4..6], 16),
            ) {
                return Rgb([r, g, b]);
            }
        }
    }
    Rgb([255, 255, 255]) // Default white background
}

pub fn flatten_alpha_to_rgb(rgba: &RgbaImage, bg: Rgb<u8>) -> image::RgbImage {
    let mut rgb = image::RgbImage::new(rgba.width(), rgba.height());
    let bg_r = bg[0] as f32;
    let bg_g = bg[1] as f32;
    let bg_b = bg[2] as f32;

    for (x, y, pixel) in rgba.enumerate_pixels() {
        let alpha = pixel[3] as f32 / 255.0;
        let r = (pixel[0] as f32 * alpha + bg_r * (1.0 - alpha)).round() as u8;
        let g = (pixel[1] as f32 * alpha + bg_g * (1.0 - alpha)).round() as u8;
        let b = (pixel[2] as f32 * alpha + bg_b * (1.0 - alpha)).round() as u8;
        rgb.put_pixel(x, y, Rgb([r, g, b]));
    }
    rgb
}

pub fn convert_image_format(
    input_path: &Path,
    target_format_str: &str,
    quality: Option<u8>,
    bg_color_hex: Option<&str>,
) -> Result<PathBuf, String> {
    if !input_path.is_file() {
        return Err("Source file does not exist.".into());
    }

    let decoded = image::ImageReader::open(input_path)
        .map_err(|e| format!("Cannot open image: {e}"))?
        .with_guessed_format()
        .map_err(|e| format!("Cannot identify image format: {e}"))?
        .decode()
        .map_err(|e| format!("Cannot decode image: {e}"))?;

    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("image");

    let (ext, format) = match target_format_str.to_ascii_lowercase().as_str() {
        "png" => ("png", ImageFormat::Png),
        "jpg" | "jpeg" => ("jpg", ImageFormat::Jpeg),
        "webp" => ("webp", ImageFormat::WebP),
        "bmp" => ("bmp", ImageFormat::Bmp),
        "tiff" | "tif" => ("tiff", ImageFormat::Tiff),
        "gif" => ("gif", ImageFormat::Gif),
        _ => return Err(format!("Unsupported output format: {target_format_str}")),
    };

    let desired = input_path.with_file_name(format!("{stem}.{ext}"));
    let atomic_writer = AtomicOutput::new(desired)?;

    if format == ImageFormat::Jpeg {
        let rgb_image = if decoded.color().has_alpha() {
            let bg = parse_hex_color(bg_color_hex);
            flatten_alpha_to_rgb(&decoded.to_rgba8(), bg)
        } else {
            decoded.to_rgb8()
        };

        let q = quality.unwrap_or(85).clamp(1, 100);
        let mut file = atomic_writer.create_file()?;
        let encoder = JpegEncoder::new_with_quality(&mut file, q);
        encoder
            .write_image(
                rgb_image.as_raw(),
                rgb_image.width(),
                rgb_image.height(),
                ExtendedColorType::Rgb8,
            )
            .map_err(|e| format!("JPEG encoding failed: {e}"))?;
        file.flush().map_err(|e| format!("Flush failed: {e}"))?;
    } else {
        let mut file = atomic_writer.create_file()?;
        decoded
            .write_to(&mut file, format)
            .map_err(|e| format!("Image encoding to {target_format_str} failed: {e}"))?;
        file.flush().map_err(|e| format!("Flush failed: {e}"))?;
    }

    atomic_writer.finish()
}
