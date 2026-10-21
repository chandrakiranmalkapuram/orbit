use crate::models::{
    ActionCategory, ActionDefinition, ActionParameterType, FileKind, FileMetadata,
};

pub struct CapabilityRegistry;

impl CapabilityRegistry {
    pub fn get_available_actions(file: &FileMetadata) -> Vec<ActionDefinition> {
        let mut actions = Vec::new();
        let mime = file.mime.as_str();
        let ext = file.extension.to_ascii_lowercase();

        match file.kind {
            FileKind::Image => {
                // 1. Format Conversions
                if mime != "image/png" {
                    actions.push(ActionDefinition {
                        id: "to-png".into(),
                        label: "PNG".into(),
                        detail: "Convert to lossless PNG image".into(),
                        symbol: "◈".into(),
                        category: ActionCategory::Format,
                        engine: "image-rs".into(),
                        input_formats: vec!["jpg".into(), "jpeg".into(), "webp".into(), "bmp".into(), "tiff".into(), "gif".into()],
                        output_format: Some("png".into()),
                        available: true,
                        limitations: None,
                        supports_batch: true,
                        supports_progress: false,
                        output_naming_rule: "stem.png".into(),
                        parameter: None,
                        is_primary: true,
                    });
                }

                if mime != "image/jpeg" {
                    actions.push(ActionDefinition {
                        id: "to-jpeg".into(),
                        label: "JPEG".into(),
                        detail: "Convert to standard JPEG image".into(),
                        symbol: "◉".into(),
                        category: ActionCategory::Format,
                        engine: "image-rs".into(),
                        input_formats: vec!["png".into(), "webp".into(), "bmp".into(), "tiff".into(), "gif".into()],
                        output_format: Some("jpg".into()),
                        available: true,
                        limitations: Some("Alpha channel flattened to background color".into()),
                        supports_batch: true,
                        supports_progress: false,
                        output_naming_rule: "stem.jpg".into(),
                        parameter: Some(ActionParameterType::Quality { min: 10, max: 100, default: 85 }),
                        is_primary: true,
                    });
                }

                if mime != "image/webp" {
                    actions.push(ActionDefinition {
                        id: "to-webp".into(),
                        label: "WebP".into(),
                        detail: "Convert to modern WebP image".into(),
                        symbol: "◇".into(),
                        category: ActionCategory::Format,
                        engine: "image-rs".into(),
                        input_formats: vec!["jpg".into(), "jpeg".into(), "png".into(), "bmp".into(), "tiff".into(), "gif".into()],
                        output_format: Some("webp".into()),
                        available: true,
                        limitations: None,
                        supports_batch: true,
                        supports_progress: false,
                        output_naming_rule: "stem.webp".into(),
                        parameter: None,
                        is_primary: true,
                    });
                }

                // Image to PDF (always available)
                actions.push(ActionDefinition {
                    id: "to-pdf".into(),
                    label: "To PDF".into(),
                    detail: "Convert image into a PDF document".into(),
                    symbol: "▤".into(),
                    category: ActionCategory::Format,
                    engine: "pdf-writer".into(),
                    input_formats: vec!["jpg".into(), "jpeg".into(), "png".into(), "webp".into(), "bmp".into(), "tiff".into()],
                    output_format: Some("pdf".into()),
                    available: true,
                    limitations: None,
                    supports_batch: true,
                    supports_progress: false,
                    output_naming_rule: "stem.pdf".into(),
                    parameter: None,
                    is_primary: true,
                });

                if mime != "image/bmp" {
                    actions.push(ActionDefinition {
                        id: "to-bmp".into(),
                        label: "BMP".into(),
                        detail: "Convert to uncompressed Windows BMP".into(),
                        symbol: "▨".into(),
                        category: ActionCategory::Format,
                        engine: "image-rs".into(),
                        input_formats: vec!["jpg".into(), "jpeg".into(), "png".into(), "webp".into(), "tiff".into(), "gif".into()],
                        output_format: Some("bmp".into()),
                        available: true,
                        limitations: None,
                        supports_batch: true,
                        supports_progress: false,
                        output_naming_rule: "stem.bmp".into(),
                        parameter: None,
                        is_primary: false,
                    });
                }

                if mime != "image/tiff" {
                    actions.push(ActionDefinition {
                        id: "to-tiff".into(),
                        label: "TIFF".into(),
                        detail: "Convert to TIFF archival image".into(),
                        symbol: "▩".into(),
                        category: ActionCategory::Format,
                        engine: "image-rs".into(),
                        input_formats: vec!["jpg".into(), "jpeg".into(), "png".into(), "webp".into(), "bmp".into(), "gif".into()],
                        output_format: Some("tiff".into()),
                        available: true,
                        limitations: None,
                        supports_batch: true,
                        supports_progress: false,
                        output_naming_rule: "stem.tiff".into(),
                        parameter: None,
                        is_primary: false,
                    });
                }

                // 2. Optimizations
                actions.push(ActionDefinition {
                    id: "compress-image".into(),
                    label: "Compress".into(),
                    detail: "Optimize quality and file size".into(),
                    symbol: "⇣".into(),
                    category: ActionCategory::Optimize,
                    engine: "image-rs".into(),
                    input_formats: vec!["jpg".into(), "jpeg".into(), "png".into(), "webp".into()],
                    output_format: None,
                    available: true,
                    limitations: None,
                    supports_batch: true,
                    supports_progress: false,
                    output_naming_rule: "stem-compressed.ext".into(),
                    parameter: Some(ActionParameterType::Quality { min: 20, max: 95, default: 75 }),
                    is_primary: true,
                });

                actions.push(ActionDefinition {
                    id: "strip-metadata".into(),
                    label: "Strip EXIF".into(),
                    detail: "Remove camera, GPS, and sensitive metadata".into(),
                    symbol: "⌧".into(),
                    category: ActionCategory::Optimize,
                    engine: "image-rs".into(),
                    input_formats: vec!["jpg".into(), "jpeg".into(), "png".into(), "webp".into(), "tiff".into()],
                    output_format: None,
                    available: true,
                    limitations: None,
                    supports_batch: true,
                    supports_progress: false,
                    output_naming_rule: "stem-clean.ext".into(),
                    parameter: None,
                    is_primary: false,
                });

                // 3. Transformations
                actions.push(ActionDefinition {
                    id: "resize-scale".into(),
                    label: "Resize".into(),
                    detail: "Scale image dimensions (50%, 75%, 150%, 200%)".into(),
                    symbol: "⤢".into(),
                    category: ActionCategory::Transform,
                    engine: "image-rs".into(),
                    input_formats: vec!["jpg".into(), "jpeg".into(), "png".into(), "webp".into(), "bmp".into(), "tiff".into()],
                    output_format: None,
                    available: true,
                    limitations: None,
                    supports_batch: true,
                    supports_progress: false,
                    output_naming_rule: "stem-resized.ext".into(),
                    parameter: Some(ActionParameterType::Scale { presets: vec![25, 50, 75, 150, 200], default: 50 }),
                    is_primary: true,
                });

                actions.push(ActionDefinition {
                    id: "rotate-90".into(),
                    label: "Rotate 90°".into(),
                    detail: "Rotate clockwise 90 degrees".into(),
                    symbol: "↻".into(),
                    category: ActionCategory::Transform,
                    engine: "image-rs".into(),
                    input_formats: vec!["jpg".into(), "jpeg".into(), "png".into(), "webp".into(), "bmp".into(), "tiff".into()],
                    output_format: None,
                    available: true,
                    limitations: None,
                    supports_batch: true,
                    supports_progress: false,
                    output_naming_rule: "stem-rotated-90.ext".into(),
                    parameter: None,
                    is_primary: false,
                });

                actions.push(ActionDefinition {
                    id: "rotate-180".into(),
                    label: "Rotate 180°".into(),
                    detail: "Rotate 180 degrees upside down".into(),
                    symbol: "↷".into(),
                    category: ActionCategory::Transform,
                    engine: "image-rs".into(),
                    input_formats: vec!["jpg".into(), "jpeg".into(), "png".into(), "webp".into(), "bmp".into(), "tiff".into()],
                    output_format: None,
                    available: true,
                    limitations: None,
                    supports_batch: true,
                    supports_progress: false,
                    output_naming_rule: "stem-rotated-180.ext".into(),
                    parameter: None,
                    is_primary: false,
                });

                actions.push(ActionDefinition {
                    id: "rotate-270".into(),
                    label: "Rotate 270°".into(),
                    detail: "Rotate 270 degrees (90° counter-clockwise)".into(),
                    symbol: "↺".into(),
                    category: ActionCategory::Transform,
                    engine: "image-rs".into(),
                    input_formats: vec!["jpg".into(), "jpeg".into(), "png".into(), "webp".into(), "bmp".into(), "tiff".into()],
                    output_format: None,
                    available: true,
                    limitations: None,
                    supports_batch: true,
                    supports_progress: false,
                    output_naming_rule: "stem-rotated-270.ext".into(),
                    parameter: None,
                    is_primary: false,
                });

                actions.push(ActionDefinition {
                    id: "flip-h".into(),
                    label: "Flip H".into(),
                    detail: "Flip image horizontally (mirror)".into(),
                    symbol: "⇄".into(),
                    category: ActionCategory::Transform,
                    engine: "image-rs".into(),
                    input_formats: vec!["jpg".into(), "jpeg".into(), "png".into(), "webp".into(), "bmp".into(), "tiff".into()],
                    output_format: None,
                    available: true,
                    limitations: None,
                    supports_batch: true,
                    supports_progress: false,
                    output_naming_rule: "stem-flipped-h.ext".into(),
                    parameter: None,
                    is_primary: false,
                });

                actions.push(ActionDefinition {
                    id: "flip-v".into(),
                    label: "Flip V".into(),
                    detail: "Flip image vertically".into(),
                    symbol: "⇅".into(),
                    category: ActionCategory::Transform,
                    engine: "image-rs".into(),
                    input_formats: vec!["jpg".into(), "jpeg".into(), "png".into(), "webp".into(), "bmp".into(), "tiff".into()],
                    output_format: None,
                    available: true,
                    limitations: None,
                    supports_batch: true,
                    supports_progress: false,
                    output_naming_rule: "stem-flipped-v.ext".into(),
                    parameter: None,
                    is_primary: false,
                });

                actions.push(ActionDefinition {
                    id: "to-grayscale".into(),
                    label: "Grayscale".into(),
                    detail: "Convert colors to black and white".into(),
                    symbol: "◐".into(),
                    category: ActionCategory::Transform,
                    engine: "image-rs".into(),
                    input_formats: vec!["jpg".into(), "jpeg".into(), "png".into(), "webp".into(), "bmp".into(), "tiff".into()],
                    output_format: None,
                    available: true,
                    limitations: None,
                    supports_batch: true,
                    supports_progress: false,
                    output_naming_rule: "stem-grayscale.ext".into(),
                    parameter: None,
                    is_primary: true,
                });

                actions.push(ActionDefinition {
                    id: "crop-square".into(),
                    label: "Square Crop".into(),
                    detail: "Center crop into a 1:1 square".into(),
                    symbol: "⎕".into(),
                    category: ActionCategory::Transform,
                    engine: "image-rs".into(),
                    input_formats: vec!["jpg".into(), "jpeg".into(), "png".into(), "webp".into(), "bmp".into(), "tiff".into()],
                    output_format: None,
                    available: true,
                    limitations: None,
                    supports_batch: true,
                    supports_progress: false,
                    output_naming_rule: "stem-cropped.ext".into(),
                    parameter: None,
                    is_primary: false,
                });
            }

            FileKind::Archive if ext == "zip" || mime == "application/zip" => {
                actions.push(ActionDefinition {
                    id: "extract-zip".into(),
                    label: "Extract".into(),
                    detail: "Safely extract archive contents to folder".into(),
                    symbol: "↗".into(),
                    category: ActionCategory::Archive,
                    engine: "zip-rs".into(),
                    input_formats: vec!["zip".into()],
                    output_format: None,
                    available: true,
                    limitations: Some("Encrypted/password-protected archives not yet supported".into()),
                    supports_batch: false,
                    supports_progress: false,
                    output_naming_rule: "stem".into(),
                    parameter: None,
                    is_primary: true,
                });

                actions.push(ActionDefinition {
                    id: "preview-zip".into(),
                    label: "Preview".into(),
                    detail: "Inspect filenames and folder hierarchy".into(),
                    symbol: "☷".into(),
                    category: ActionCategory::Archive,
                    engine: "zip-rs".into(),
                    input_formats: vec!["zip".into()],
                    output_format: None,
                    available: true,
                    limitations: Some("Max 10,000 entries".into()),
                    supports_batch: false,
                    supports_progress: false,
                    output_naming_rule: "".into(),
                    parameter: None,
                    is_primary: true,
                });
            }

            _ => {}
        }

        // Common Utilities for every supported file
        if !actions.is_empty() {
            actions.push(ActionDefinition {
                id: "reveal-finder".into(),
                label: "Finder".into(),
                detail: "Highlight original file in Finder".into(),
                symbol: "⌕".into(),
                category: ActionCategory::Utility,
                engine: "system".into(),
                input_formats: vec!["*".into()],
                output_format: None,
                available: true,
                limitations: None,
                supports_batch: false,
                supports_progress: false,
                output_naming_rule: "".into(),
                parameter: None,
                is_primary: false,
            });

            actions.push(ActionDefinition {
                id: "copy-path".into(),
                label: "Copy Path".into(),
                detail: "Copy file path to clipboard".into(),
                symbol: "⎘".into(),
                category: ActionCategory::Utility,
                engine: "system".into(),
                input_formats: vec!["*".into()],
                output_format: None,
                available: true,
                limitations: None,
                supports_batch: false,
                supports_progress: false,
                output_naming_rule: "".into(),
                parameter: None,
                is_primary: false,
            });
        }

        // Only return actions whose implementation is genuinely available
        actions.into_iter().filter(|a| a.available).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_metadata(kind: FileKind, mime: &str, ext: &str) -> FileMetadata {
        FileMetadata {
            path: format!("/path/to/test.{}", ext),
            name: format!("test.{}", ext),
            extension: ext.to_string(),
            kind,
            mime: mime.to_string(),
            size: 1024,
            width: Some(800),
            height: Some(600),
            format_name: Some(ext.to_uppercase()),
            has_alpha: Some(false),
            actions: vec![],
        }
    }

    #[test]
    fn test_jpeg_actions_registry() {
        let meta = create_test_metadata(FileKind::Image, "image/jpeg", "jpg");
        let actions = CapabilityRegistry::get_available_actions(&meta);

        let ids: Vec<&str> = actions.iter().map(|a| a.id.as_str()).collect();

        // Must contain genuine implemented conversions
        assert!(ids.contains(&"to-png"));
        assert!(ids.contains(&"to-webp"));
        assert!(ids.contains(&"to-pdf"));
        assert!(ids.contains(&"to-bmp"));
        assert!(ids.contains(&"to-tiff"));
        assert!(ids.contains(&"compress-image"));
        assert!(ids.contains(&"resize-scale"));
        assert!(ids.contains(&"rotate-90"));
        assert!(ids.contains(&"rotate-180"));
        assert!(ids.contains(&"rotate-270"));
        assert!(ids.contains(&"flip-h"));
        assert!(ids.contains(&"flip-v"));
        assert!(ids.contains(&"to-grayscale"));
        assert!(ids.contains(&"strip-metadata"));
        assert!(ids.contains(&"crop-square"));
        assert!(ids.contains(&"reveal-finder"));
        assert!(ids.contains(&"copy-path"));

        // Must NOT offer converting JPEG to JPEG format conversion
        assert!(!ids.contains(&"to-jpeg"));

        // Must NOT offer unsupported/unimplemented operations
        assert!(!ids.contains(&"to-heic"));
        assert!(!ids.contains(&"to-svg"));
    }

    #[test]
    fn test_png_actions_registry() {
        let meta = create_test_metadata(FileKind::Image, "image/png", "png");
        let actions = CapabilityRegistry::get_available_actions(&meta);
        let ids: Vec<&str> = actions.iter().map(|a| a.id.as_str()).collect();

        assert!(ids.contains(&"to-jpeg"));
        assert!(ids.contains(&"to-webp"));
        assert!(ids.contains(&"to-pdf"));
        assert!(!ids.contains(&"to-png")); // PNG to PNG redundant
    }

    #[test]
    fn test_zip_actions_registry() {
        let meta = create_test_metadata(FileKind::Archive, "application/zip", "zip");
        let actions = CapabilityRegistry::get_available_actions(&meta);
        let ids: Vec<&str> = actions.iter().map(|a| a.id.as_str()).collect();

        assert!(ids.contains(&"extract-zip"));
        assert!(ids.contains(&"preview-zip"));
        assert!(!ids.contains(&"to-png"));
    }

    #[test]
    fn test_unsupported_file_has_no_actions() {
        let meta = create_test_metadata(FileKind::Unknown, "application/octet-stream", "bin");
        let actions = CapabilityRegistry::get_available_actions(&meta);
        assert!(actions.is_empty());
    }
}
