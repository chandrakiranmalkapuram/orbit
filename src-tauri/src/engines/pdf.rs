use flate2::write::ZlibEncoder;
use flate2::Compression;
use image::{DynamicImage, GenericImageView};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

struct PdfObject {
    number: usize,
    data: Vec<u8>,
}

pub fn image_file_to_pdf(image_path: &Path, output_path: &Path) -> Result<(), String> {
    let mut file = File::open(image_path).map_err(|e| format!("Cannot open input image: {e}"))?;
    let mut head = [0u8; 3];
    let is_jpeg = if file.read_exact(&mut head).is_ok() {
        head == [0xff, 0xd8, 0xff]
    } else {
        false
    };

    if is_jpeg {
        // Direct DCTDecode JPEG passthrough for lossless, high-speed PDF creation
        let jpeg_bytes = std::fs::read(image_path).map_err(|e| format!("Cannot read JPEG: {e}"))?;
        let dimensions = image::image_dimensions(image_path)
            .map_err(|e| format!("Cannot read JPEG dimensions: {e}"))?;
        write_single_image_pdf(
            output_path,
            dimensions.0,
            dimensions.1,
            &jpeg_bytes,
            true, // is_dct
        )
    } else {
        let decoded = image::ImageReader::open(image_path)
            .map_err(|e| format!("Cannot open image: {e}"))?
            .with_guessed_format()
            .map_err(|e| format!("Cannot guess format: {e}"))?
            .decode()
            .map_err(|e| format!("Cannot decode image: {e}"))?;

        dynamic_image_to_pdf(&decoded, output_path)
    }
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

    write_single_image_pdf(
        output_path,
        width,
        height,
        &compressed_bytes,
        false, // is_dct (FlateDecode)
    )
}

fn write_single_image_pdf(
    output_path: &Path,
    width: u32,
    height: u32,
    image_bytes: &[u8],
    is_dct: bool,
) -> Result<(), String> {
    let mut file = File::create(output_path).map_err(|e| format!("Cannot create PDF: {e}"))?;
    let filter = if is_dct { "/DCTDecode" } else { "/FlateDecode" };

    // Standard PDF 1.4 coordinates (points)
    let w_pt = width as f64;
    let h_pt = height as f64;

    let content_stream = format!("q {w_pt:.2} 0 0 {h_pt:.2} 0 0 cm /Im1 Do Q\n");
    let content_bytes = content_stream.as_bytes();

    let mut objects: Vec<PdfObject> = Vec::new();

    // 1: Catalog
    objects.push(PdfObject {
        number: 1,
        data: b"<< /Type /Catalog /Pages 2 0 R >>\n".to_vec(),
    });

    // 2: Pages
    objects.push(PdfObject {
        number: 2,
        data: b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>\n".to_vec(),
    });

    // 3: Page
    let page_dict = format!(
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {w_pt:.2} {h_pt:.2}] /Contents 4 0 R /Resources << /XObject << /Im1 5 0 R >> /ProcSet [/PDF /ImageC] >> >>\n"
    );
    objects.push(PdfObject {
        number: 3,
        data: page_dict.into_bytes(),
    });

    // 4: Content Stream
    let mut obj4 = Vec::new();
    let _ = write!(obj4, "<< /Length {} >>\nstream\n", content_bytes.len());
    obj4.extend_from_slice(content_bytes);
    obj4.extend_from_slice(b"endstream\n");
    objects.push(PdfObject {
        number: 4,
        data: obj4,
    });

    // 5: Image XObject
    let mut obj5 = Vec::new();
    let _ = write!(
        obj5,
        "<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter {} /Length {} >>\nstream\n",
        width,
        height,
        filter,
        image_bytes.len()
    );
    obj5.extend_from_slice(image_bytes);
    obj5.extend_from_slice(b"\nendstream\n");
    objects.push(PdfObject {
        number: 5,
        data: obj5,
    });

    // Write PDF file
    file.write_all(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n")
        .map_err(|e| format!("Failed to write PDF header: {e}"))?;

    let mut offsets = Vec::new();
    offsets.push(0); // 0th object

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
    let xref_header = format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1);
    file.write_all(xref_header.as_bytes())
        .map_err(|e| format!("Write failed: {e}"))?;

    for &off in &offsets[1..] {
        let entry = format!("{:010} 00000 n \n", off);
        file.write_all(entry.as_bytes())
            .map_err(|e| format!("Write failed: {e}"))?;
    }

    let trailer = format!(
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
        objects.len() + 1,
        xref_offset
    );
    file.write_all(trailer.as_bytes())
        .map_err(|e| format!("Write failed: {e}"))?;

    file.flush().map_err(|e| format!("Flush failed: {e}"))?;
    Ok(())
}
