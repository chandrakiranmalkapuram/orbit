# ORBIT

**Drop it. Transform it. Done.**

ORBIT is a local-first macOS desktop utility built with Tauri 2, React, TypeScript, and Rust. Its design centres on a floating, transparent interactive orb that stays on your desktop, ready for Finder drag-and-drop.

---

## Key Features

- **Floating Desktop Orb**: Frameless, transparent floating window with `alwaysOnTop` toggle and native macOS window dragging, featuring an adaptive non-overlapping radial layout with dedicated palette drawer for many options.
- **Dynamic Capability Registry**: Rust-backed registry that determines genuine, valid actions based on detected magic bytes, selection counts, and installed local engines.
- **Multipage PDF Combination**:
  - Drag 2, 5, 10, or more images (JPEG, PNG, WebP, BMP, TIFF) directly from Finder onto the orb.
  - Automatically groups files and provides a 1-click **"Combine into one PDF"** action.
  - Preserves exact drop order, creates 1 page per image with individual MediaBox dimensions matching aspect ratio (no distortion or stretching).
  - Handles mixed formats, portrait and landscape orientations, and flattens alpha cleanly.
  - Generates safe atomic output saved beside the first image (`Combined.pdf` or `Combined (2).pdf` on collision without overwriting).
- **Comprehensive Image Conversions**:
  - Convert single images or batch convert between **PNG, JPEG, WebP, BMP, TIFF**, and **GIF**.
  - **Image-to-PDF**: Pure-Rust PDF 1.4 document creation with JPEG passthrough (`/DCTDecode`) and Deflate compression (`/FlateDecode`).
  - **Alpha Flattening**: PNG/WebP with transparency are cleanly flattened onto customizable background colors for JPEG output (no black box artifacts).
- **Image Transformations**:
  - Rotate 90°, 180°, and 270°.
  - Horizontal & Vertical flip.
  - Grayscale conversion.
  - Lanczos3 high-quality scaling (percentage presets or custom dimensions).
  - Center square cropping (1:1).
  - EXIF & metadata stripping.
- **Hardened Archive Processing**:
  - ZIP listing & safe extraction.
  - Path traversal defense (`..` rejection).
  - Symbolic link rejection.
  - Zip-bomb limits (10,000 max entries, 2 GB total extracted limit, 512 MB per-file limit).
- **PDF Document Processing**:
  - **PDF to Images**: Convert PDF page(s) to lossless PNG or standard JPEG images using macOS native `sips` rendering.
  - **Extract Text**: Extract all text layers from PDF pages into a readable `.txt` document via pure-Rust `lopdf`.
  - **Split Pages**: Split multipage PDFs into individual 1-page PDF files (`doc_page_1.pdf`, `doc_page_2.pdf`...).
  - **Merge PDFs**: Drop 2 or more PDFs to merge them into a single `Combined.pdf`.
  - **Mixed Combine**: Drop images and PDFs together to compile them into one continuous document in original drop order.
  - **PDF Info**: Inspect PDF version, page count, and metadata.
- **Atomic Output Safety**: Original files are never modified. Outputs are written atomically to temporary files and renamed without overwriting existing files (`file (2).png`, `Combined (2).pdf`).
- **Contextual Parameters**: Adjust compression quality or resize scaling directly on the orb without complex modal dialogs or overlapping controls.
- **Privacy & Offline First**: Zero cloud dependencies, zero external APIs, zero telemetry, zero analytics, zero AI models.

---

## Genuine Capabilities vs Known Limitations

### Supported & Tested
- Multipage PDF: Multi-image drop (JPEG, PNG, WebP, etc.) or multi-PDF drop or mixed drop ➔ Single combined PDF with N pages in original order.
- PDF Operations: PDF ➔ PNG, PDF ➔ JPEG, PDF Text Extraction (.txt), PDF Page Splitting, PDF Info.
- Format conversions: JPEG ➔ PNG, JPEG ➔ WebP, JPEG ➔ BMP, JPEG ➔ TIFF, JPEG ➔ PDF, PNG ➔ JPEG, PNG ➔ WebP, WebP ➔ PNG, WebP ➔ JPEG.
- Transformations: Rotate, Flip, Grayscale, Resize, Square Crop, Strip EXIF.
- Archives: ZIP preview and safe extraction.
- Utilities: Reveal in Finder, Copy Path.

### Explicitly Excluded (No Placeholders)
- **HEIC Encoding**: Not offered unless a native macOS hardware encoder or libheif is installed locally.
- **Raster-to-SVG**: Not offered because raster pixels cannot be converted into true vector artwork without vector tracing.
- **Encrypted Archives**: Password-protected ZIPs are not yet supported.

---

## Prerequisites

- macOS 12.0 or newer (Apple Silicon or Intel)
- Node.js 20 or newer and npm
- Rust toolchain (`rustc` and `cargo` via `rustup`)
- Xcode Command Line Tools (`xcode-select --install`)

---

## Development & Testing

```sh
# Install dependencies
npm install

# Run automated tests (Rust unit & integration tests)
cargo test --manifest-path src-tauri/Cargo.toml

# Run TypeScript type-check and frontend build
npm run build

# Run in development mode
npm run tauri dev
```

---

## Building the macOS App Bundle

```sh
npm run tauri build
```

Built bundles will be located at:
- **macOS App**: `src-tauri/target/release/bundle/macos/ORBIT.app`
- **Installer DMG**: `src-tauri/target/release/bundle/dmg/ORBIT_0.1.0_aarch64.dmg`

---

## Verifying Offline Operation

1. Turn off Wi-Fi on your Mac (or disconnect Ethernet).
2. Launch `ORBIT.app` from `src-tauri/target/release/bundle/macos/`.
3. Drop a JPEG, PNG, or ZIP file onto the orb.
4. Perform conversions and extraction.
5. All operations run locally and complete without any internet connection.
