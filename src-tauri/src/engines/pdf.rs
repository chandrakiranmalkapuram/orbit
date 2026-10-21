use flate2::write::ZlibEncoder;
use flate2::Compression;
use image::{DynamicImage, GenericImageView};
use lopdf::{Document, Object, ObjectId};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

struct PdfObject {
    number: usize,
    data: Vec<u8>,
}

struct PdfPageImage {
    width: u32,
    height: u32,
    image_bytes: Vec<u8>,
    is_dct: bool,
}

pub fn image_file_to_pdf(image_path: &Path, output_path: &Path) -> Result<(), String> {
    images_files_to_pdf(&[image_path], output_path)
}

pub fn dynamic_image_to_pdf(img: &DynamicImage, output_path: &Path) -> Result<(), String> {
    let (width, height) = img.dimensions();
    let rgb = img.to_rgb8();
    let raw_pixels = rgb.as_raw();

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(raw_pixels)
        .map_err(|e| format!("Deflate compression failed: {e}"))?;
    let compressed_bytes = encoder
        .finish()
        .map_err(|e| format!("Deflate compression finish failed: {e}"))?;

    let page = PdfPageImage {
        width,
        height,
        image_bytes: compressed_bytes,
        is_dct: false,
    };

    write_multipage_pdf(output_path, &[page])
}

fn process_image_file(image_path: &Path) -> Result<PdfPageImage, String> {
    if !image_path.is_file() {
        return Err(format!("File does not exist: {}", image_path.display()));
    }

    let mut file = File::open(image_path)
        .map_err(|e| format!("Cannot open '{}': {e}", image_path.display()))?;
    let mut head = [0u8; 3];
    let is_jpeg = if file.read_exact(&mut head).is_ok() {
        head == [0xff, 0xd8, 0xff]
    } else {
        false
    };

    if is_jpeg {
        // Direct DCTDecode JPEG passthrough (fast and lossless)
        let dimensions = image::image_dimensions(image_path)
            .map_err(|e| format!("Cannot read JPEG dimensions from '{}': {e}", image_path.display()))?;
        let jpeg_bytes = std::fs::read(image_path)
            .map_err(|e| format!("Cannot read JPEG bytes from '{}': {e}", image_path.display()))?;
        Ok(PdfPageImage {
            width: dimensions.0,
            height: dimensions.1,
            image_bytes: jpeg_bytes,
            is_dct: true,
        })
    } else {
        let decoded = image::ImageReader::open(image_path)
            .map_err(|e| format!("Cannot open image '{}': {e}", image_path.display()))?
            .with_guessed_format()
            .map_err(|e| format!("Cannot identify format of '{}': {e}", image_path.display()))?
            .decode()
            .map_err(|e| format!("Cannot decode image '{}': {e}", image_path.display()))?;

        let (width, height) = decoded.dimensions();
        let rgb = if decoded.color().has_alpha() {
            // Flatten alpha onto a crisp white background
            let rgba = decoded.to_rgba8();
            let mut flat = image::RgbImage::new(width, height);
            for (x, y, pixel) in rgba.enumerate_pixels() {
                let alpha = pixel[3] as f32 / 255.0;
                let r = (pixel[0] as f32 * alpha + 255.0 * (1.0 - alpha)).round() as u8;
                let g = (pixel[1] as f32 * alpha + 255.0 * (1.0 - alpha)).round() as u8;
                let b = (pixel[2] as f32 * alpha + 255.0 * (1.0 - alpha)).round() as u8;
                flat.put_pixel(x, y, image::Rgb([r, g, b]));
            }
            flat
        } else {
            decoded.to_rgb8()
        };

        let raw_pixels = rgb.as_raw();
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(raw_pixels)
            .map_err(|e| format!("Deflate compression failed for '{}': {e}", image_path.display()))?;
        let compressed_bytes = encoder
            .finish()
            .map_err(|e| format!("Deflate compression finish failed for '{}': {e}", image_path.display()))?;

        Ok(PdfPageImage {
            width,
            height,
            image_bytes: compressed_bytes,
            is_dct: false,
        })
    }
}

pub fn images_files_to_pdf(image_paths: &[&Path], output_path: &Path) -> Result<(), String> {
    if image_paths.is_empty() {
        return Err("No input images provided to create PDF.".into());
    }

    let mut pages = Vec::with_capacity(image_paths.len());
    for &path in image_paths {
        let page = process_image_file(path)?;
        pages.push(page);
    }

    write_multipage_pdf(output_path, &pages)
}

fn write_multipage_pdf(output_path: &Path, pages: &[PdfPageImage]) -> Result<(), String> {
    if pages.is_empty() {
        return Err("Cannot create empty PDF.".into());
    }

    let page_count = pages.len();
    let mut objects: Vec<PdfObject> = Vec::new();

    // 1: Catalog
    objects.push(PdfObject {
        number: 1,
        data: b"<< /Type /Catalog /Pages 2 0 R >>\n".to_vec(),
    });

    // 2: Pages dictionary listing all child page object numbers
    let mut kids = String::new();
    for i in 0..page_count {
        let page_obj_num = 3 + i * 3;
        if i > 0 {
            kids.push(' ');
        }
        kids.push_str(&format!("{page_obj_num} 0 R"));
    }
    let pages_dict = format!("<< /Type /Pages /Kids [{kids}] /Count {page_count} >>\n");
    objects.push(PdfObject {
        number: 2,
        data: pages_dict.into_bytes(),
    });

    // For each page:
    // Page Obj: 3 + i * 3
    // Content Stream Obj: 3 + i * 3 + 1
    // Image XObject Obj: 3 + i * 3 + 2
    for (i, page) in pages.iter().enumerate() {
        let page_obj_num = 3 + i * 3;
        let content_obj_num = page_obj_num + 1;
        let image_obj_num = page_obj_num + 2;

        let w_pt = page.width as f64;
        let h_pt = page.height as f64;

        // Page Dict
        let page_dict = format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {w_pt:.2} {h_pt:.2}] /Contents {content_obj_num} 0 R /Resources << /XObject << /Im1 {image_obj_num} 0 R >> /ProcSet [/PDF /ImageC] >> >>\n"
        );
        objects.push(PdfObject {
            number: page_obj_num,
            data: page_dict.into_bytes(),
        });

        // Content Stream
        let content_stream = format!("q {w_pt:.2} 0 0 {h_pt:.2} 0 0 cm /Im1 Do Q\n");
        let content_bytes = content_stream.as_bytes();
        let mut obj_content = Vec::new();
        let _ = write!(obj_content, "<< /Length {} >>\nstream\n", content_bytes.len());
        obj_content.extend_from_slice(content_bytes);
        obj_content.extend_from_slice(b"endstream\n");
        objects.push(PdfObject {
            number: content_obj_num,
            data: obj_content,
        });

        // Image XObject
        let filter = if page.is_dct { "/DCTDecode" } else { "/FlateDecode" };
        let mut obj_image = Vec::new();
        let _ = write!(
            obj_image,
            "<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter {} /Length {} >>\nstream\n",
            page.width,
            page.height,
            filter,
            page.image_bytes.len()
        );
        obj_image.extend_from_slice(&page.image_bytes);
        obj_image.extend_from_slice(b"\nendstream\n");
        objects.push(PdfObject {
            number: image_obj_num,
            data: obj_image,
        });
    }

    // Write file to output
    let mut file = File::create(output_path).map_err(|e| format!("Cannot create PDF: {e}"))?;
    file.write_all(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n")
        .map_err(|e| format!("Failed writing PDF header: {e}"))?;

    let mut offsets = Vec::new();
    offsets.push(0); // 0th entry

    let mut current_offset = 15; // length of header b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n"

    for obj in &objects {
        offsets.push(current_offset);
        let header = format!("{} 0 obj\n", obj.number);
        file.write_all(header.as_bytes())
            .map_err(|e| format!("Write failed: {e}"))?;
        current_offset += header.len();

        file.write_all(&obj.data)
            .map_err(|e| format!("Write failed: {e}"))?;
        current_offset += obj.data.len();

        file.write_all(b"endobj\n")
            .map_err(|e| format!("Write failed: {e}"))?;
        current_offset += 7;
    }

    let xref_offset = current_offset;
    let total_entries = objects.len() + 1;
    let xref_header = format!("xref\n0 {total_entries}\n0000000000 65535 f \n");
    file.write_all(xref_header.as_bytes())
        .map_err(|e| format!("Write failed: {e}"))?;

    for &off in &offsets[1..] {
        let entry = format!("{:010} 00000 n \n", off);
        file.write_all(entry.as_bytes())
            .map_err(|e| format!("Write failed: {e}"))?;
    }

    let trailer = format!(
        "trailer\n<< /Size {total_entries} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n"
    );
    file.write_all(trailer.as_bytes())
        .map_err(|e| format!("Write failed: {e}"))?;

    file.flush().map_err(|e| format!("Flush failed: {e}"))?;
    Ok(())
}

/// Merge multiple PDF documents into one single output PDF.
pub fn merge_pdf_documents(pdf_paths: &[&Path], output_path: &Path) -> Result<(), String> {
    if pdf_paths.is_empty() {
        return Err("No PDF files provided to merge.".into());
    }
    if pdf_paths.len() == 1 {
        std::fs::copy(pdf_paths[0], output_path)
            .map_err(|e| format!("Failed copying single PDF: {e}"))?;
        return Ok(());
    }

    let mut documents: Vec<Document> = Vec::new();
    for p in pdf_paths {
        let doc = Document::load(p)
            .map_err(|e| format!("Failed to load PDF '{}': {e}", p.display()))?;
        documents.push(doc);
    }

    let mut merged = Document::new();
    let mut documents_pages = std::collections::BTreeMap::new();
    let mut documents_objects = std::collections::BTreeMap::new();
    let mut max_id: u32 = 1;

    for mut doc in documents {
        doc.renumber_objects_with(max_id);
        max_id = doc.max_id + 1;

        let pages = doc.get_pages();
        for object_id in pages.values() {
            if let Ok(obj) = doc.get_object(*object_id) {
                documents_pages.insert(*object_id, obj.to_owned());
            }
        }
        documents_objects.extend(doc.objects);
    }

    let mut catalog_object: Option<(ObjectId, Object)> = None;
    let mut pages_object: Option<(ObjectId, Object)> = None;

    for (object_id, object) in documents_objects {
        match object.type_name().unwrap_or(b"") {
            b"Catalog" => {
                if catalog_object.is_none() {
                    catalog_object = Some((object_id, object));
                }
            }
            b"Pages" => {
                if let Ok(dict) = object.as_dict() {
                    let mut dict = dict.clone();
                    if let Some((_, ref old_obj)) = pages_object {
                        if let Ok(old_dict) = old_obj.as_dict() {
                            dict.extend(old_dict);
                        }
                    }
                    let id = pages_object.as_ref().map(|(id, _)| *id).unwrap_or(object_id);
                    pages_object = Some((id, Object::Dictionary(dict)));
                }
            }
            b"Page" => {}
            b"Outlines" | b"Outline" => {}
            _ => {
                merged.objects.insert(object_id, object);
            }
        }
    }

    let pages_id = match pages_object {
        Some((id, _)) => id,
        None => return Err("PDF merge failed: Pages root not found in documents.".into()),
    };
    let catalog_id = match catalog_object {
        Some((id, _)) => id,
        None => return Err("PDF merge failed: Catalog root not found in documents.".into()),
    };

    let mut pages_dict = lopdf::Dictionary::new();
    pages_dict.set("Type", "Pages");
    pages_dict.set("Count", documents_pages.len() as u32);
    pages_dict.set(
        "Kids",
        documents_pages
            .keys()
            .copied()
            .map(Object::Reference)
            .collect::<Vec<_>>(),
    );
    merged.objects.insert(pages_id, Object::Dictionary(pages_dict));

    for (object_id, object) in documents_pages {
        if let Ok(dict) = object.as_dict() {
            let mut dict = dict.clone();
            dict.set("Parent", pages_id);
            merged.objects.insert(object_id, Object::Dictionary(dict));
        }
    }

    let mut catalog_dict = lopdf::Dictionary::new();
    catalog_dict.set("Type", "Catalog");
    catalog_dict.set("Pages", pages_id);
    merged.objects.insert(catalog_id, Object::Dictionary(catalog_dict));

    merged.trailer.set("Root", catalog_id);
    merged.max_id = merged.objects.len() as u32;
    merged.renumber_objects();

    merged
        .save(output_path)
        .map_err(|e| format!("Failed saving merged PDF: {e}"))?;

    Ok(())
}

/// Combine images, PDFs, or a mixture of both into one single PDF.
pub fn combine_files_to_pdf(paths: &[&Path], output_path: &Path) -> Result<(), String> {
    if paths.is_empty() {
        return Err("No input files provided to combine.".into());
    }

    // Check if ALL files are purely images
    let all_images = paths.iter().all(|p| {
        if let Ok((kind, _, _)) = crate::metadata::sniff_format(p) {
            kind == crate::models::FileKind::Image
        } else {
            false
        }
    });

    if all_images {
        return images_files_to_pdf(paths, output_path);
    }

    // Check if ALL files are purely PDFs
    let all_pdfs = paths.iter().all(|p| {
        if let Ok((kind, _, _)) = crate::metadata::sniff_format(p) {
            kind == crate::models::FileKind::Pdf
        } else {
            false
        }
    });

    if all_pdfs {
        return merge_pdf_documents(paths, output_path);
    }

    // Mixed collection of images and PDFs:
    use std::sync::atomic::{AtomicUsize, Ordering};
    static MIXED_COUNTER: AtomicUsize = AtomicUsize::new(1);
    let run_id = MIXED_COUNTER.fetch_add(1, Ordering::Relaxed);
    let temp_dir = std::env::temp_dir().join(format!("orbit_mix_{}_{}", std::process::id(), run_id));
    std::fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Failed creating temporary directory: {e}"))?;

    let mut pdf_list: Vec<PathBuf> = Vec::new();
    for (i, &p) in paths.iter().enumerate() {
        let (kind, _, _) = crate::metadata::sniff_format(p)?;
        match kind {
            crate::models::FileKind::Pdf => {
                pdf_list.push(p.to_path_buf());
            }
            crate::models::FileKind::Image => {
                let temp_pdf = temp_dir.join(format!("img_{i}.pdf"));
                image_file_to_pdf(p, &temp_pdf)?;
                pdf_list.push(temp_pdf);
            }
            _ => {
                let _ = std::fs::remove_dir_all(&temp_dir);
                return Err(format!("File '{}' cannot be converted into a PDF page.", p.display()));
            }
        }
    }

    let pdf_refs: Vec<&Path> = pdf_list.iter().map(|p| p.as_path()).collect();
    let res = merge_pdf_documents(&pdf_refs, output_path);
    let _ = std::fs::remove_dir_all(&temp_dir);
    res
}

/// Split a multipage PDF into individual 1-page PDF documents.
pub fn pdf_split_pages(
    pdf_path: &Path,
    output_dir: &Path,
    stem: &str,
) -> Result<Vec<PathBuf>, String> {
    let doc = Document::load(pdf_path)
        .map_err(|e| format!("Failed to read PDF '{}': {e}", pdf_path.display()))?;

    let pages = doc.get_pages();
    let total_pages = pages.len();
    if total_pages == 0 {
        return Err("PDF document contains no pages to split.".into());
    }

    if total_pages == 1 {
        let desired = output_dir.join(format!("{stem}_page_1.pdf"));
        let final_path = crate::output::unique_path(&desired);
        std::fs::copy(pdf_path, &final_path)
            .map_err(|e| format!("Failed copying single page PDF: {e}"))?;
        return Ok(vec![final_path]);
    }

    let mut generated_paths = Vec::new();
    for (page_num, object_id) in pages {
        let mut single_doc = doc.clone();

        let mut pages_id = None;
        for (id, obj) in &single_doc.objects {
            if obj.type_name().unwrap_or(b"") == b"Pages" {
                pages_id = Some(*id);
                break;
            }
        }

        if let Some(pid) = pages_id {
            if let Ok(dict) = single_doc.get_object_mut(pid).and_then(|o| o.as_dict_mut()) {
                dict.set("Count", 1);
                dict.set("Kids", vec![Object::Reference(object_id)]);
            }
        }

        single_doc.prune_objects();
        single_doc.renumber_objects();

        let desired = output_dir.join(format!("{stem}_page_{page_num}.pdf"));
        let final_path = crate::output::unique_path(&desired);

        single_doc
            .save(&final_path)
            .map_err(|e| format!("Failed saving page {page_num}: {e}"))?;
        generated_paths.push(final_path);
    }

    Ok(generated_paths)
}

/// Convert a PDF into PNG or JPEG images using native macOS sips renderer.
pub fn pdf_to_images(
    pdf_path: &Path,
    output_dir: &Path,
    stem: &str,
    format: &str,
) -> Result<Vec<PathBuf>, String> {
    let doc = Document::load(pdf_path)
        .map_err(|e| format!("Failed to read PDF '{}': {e}", pdf_path.display()))?;
    let page_count = doc.get_pages().len();
    if page_count == 0 {
        return Err("PDF document contains no pages.".into());
    }

    let mut outputs = Vec::new();

    if page_count == 1 {
        let desired = output_dir.join(format!("{stem}.{format}"));
        let final_path = crate::output::unique_path(&desired);

        let status = std::process::Command::new("/usr/bin/sips")
            .arg("-s")
            .arg("format")
            .arg(format)
            .arg(pdf_path)
            .arg("--out")
            .arg(&final_path)
            .output()
            .map_err(|e| format!("Failed to execute sips: {e}"))?;

        if !status.status.success() {
            let stderr = String::from_utf8_lossy(&status.stderr);
            return Err(format!("sips conversion failed: {stderr}"));
        }
        outputs.push(final_path);
    } else {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static COUNTER: AtomicUsize = AtomicUsize::new(1);
        let run_id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let temp_dir = std::env::temp_dir().join(format!("orbit_pdf_img_{}_{}", std::process::id(), run_id));
        std::fs::create_dir_all(&temp_dir)
            .map_err(|e| format!("Failed creating temporary directory: {e}"))?;

        let split_pdfs = pdf_split_pages(pdf_path, &temp_dir, "temp")?;
        for (idx, page_pdf) in split_pdfs.iter().enumerate() {
            let page_num = idx + 1;
            let desired = output_dir.join(format!("{stem}_page_{page_num}.{format}"));
            let final_path = crate::output::unique_path(&desired);

            let status = std::process::Command::new("/usr/bin/sips")
                .arg("-s")
                .arg("format")
                .arg(format)
                .arg(page_pdf)
                .arg("--out")
                .arg(&final_path)
                .output()
                .map_err(|e| format!("Failed to execute sips: {e}"))?;

            if !status.status.success() {
                let stderr = String::from_utf8_lossy(&status.stderr);
                let _ = std::fs::remove_dir_all(&temp_dir);
                return Err(format!("sips conversion failed on page {page_num}: {stderr}"));
            }
            outputs.push(final_path);
        }
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    Ok(outputs)
}

/// Extract all text layers from a PDF into a readable UTF-8 text file.
pub fn pdf_extract_text(pdf_path: &Path, output_path: &Path) -> Result<usize, String> {
    let doc = Document::load(pdf_path)
        .map_err(|e| format!("Failed to read PDF '{}': {e}", pdf_path.display()))?;
    let page_numbers: Vec<u32> = (1..=doc.get_pages().len() as u32).collect();
    if page_numbers.is_empty() {
        return Err("PDF contains no pages.".into());
    }

    let mut full_text = String::new();
    for &page in &page_numbers {
        if let Ok(text) = doc.extract_text(&[page]) {
            if !text.trim().is_empty() {
                full_text.push_str(&format!("--- Page {page} ---\n"));
                full_text.push_str(&text);
                full_text.push_str("\n\n");
            }
        }
    }

    if full_text.trim().is_empty() {
        full_text = format!(
            "PDF document '{}' contains {} page(s), but no extractable text layer was found (the file may be scanned or image-based).\n",
            pdf_path.file_name().unwrap_or_default().to_string_lossy(),
            page_numbers.len()
        );
    }

    std::fs::write(output_path, &full_text)
        .map_err(|e| format!("Failed writing text output: {e}"))?;

    Ok(page_numbers.len())
}

/// Inspect PDF version, page count, and title.
pub fn pdf_info(pdf_path: &Path) -> Result<String, String> {
    let doc = Document::load(pdf_path)
        .map_err(|e| format!("Failed to read PDF '{}': {e}", pdf_path.display()))?;
    let page_count = doc.get_pages().len();
    let version = &doc.version;
    Ok(format!("PDF v{version} • {page_count} page(s)"))
}

#[cfg(test)]
mod pdf_engine_tests {
    use super::*;

    #[test]
    fn test_merge_two_pdfs() {
        use image::{Rgb, RgbImage};
        let temp_dir = std::env::temp_dir().join("orbit_test_merge_pdf");
        let _ = std::fs::create_dir_all(&temp_dir);

        let img1_path = temp_dir.join("test_img1.png");
        let img2_path = temp_dir.join("test_img2.png");
        let pdf1_path = temp_dir.join("test_pdf1.pdf");
        let pdf2_path = temp_dir.join("test_pdf2.pdf");
        let merged_path = temp_dir.join("test_merged.pdf");

        let img1 = RgbImage::from_pixel(30, 30, Rgb([255, 0, 0]));
        img1.save(&img1_path).unwrap();
        let img2 = RgbImage::from_pixel(40, 40, Rgb([0, 255, 0]));
        img2.save(&img2_path).unwrap();

        image_file_to_pdf(&img1_path, &pdf1_path).unwrap();
        image_file_to_pdf(&img2_path, &pdf2_path).unwrap();

        merge_pdf_documents(&[&pdf1_path, &pdf2_path], &merged_path).unwrap();

        let doc = Document::load(&merged_path).unwrap();
        assert_eq!(doc.get_pages().len(), 2);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_split_multipage_pdf() {
        use image::{Rgb, RgbImage};
        let temp_dir = std::env::temp_dir().join("orbit_test_split_pdf");
        let _ = std::fs::create_dir_all(&temp_dir);

        let img1_path = temp_dir.join("s_img1.png");
        let img2_path = temp_dir.join("s_img2.png");
        let multipage_path = temp_dir.join("multipage.pdf");

        let img1 = RgbImage::from_pixel(20, 20, Rgb([100, 100, 100]));
        img1.save(&img1_path).unwrap();
        let img2 = RgbImage::from_pixel(30, 30, Rgb([200, 200, 200]));
        img2.save(&img2_path).unwrap();

        images_files_to_pdf(&[&img1_path, &img2_path], &multipage_path).unwrap();

        let split_results = pdf_split_pages(&multipage_path, &temp_dir, "split_out").unwrap();
        assert_eq!(split_results.len(), 2);
        for p in &split_results {
            let doc = Document::load(p).unwrap();
            assert_eq!(doc.get_pages().len(), 1);
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}

