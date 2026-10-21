# ORBIT Architecture

## Product Rule

The interface may **only** show actions registered as genuinely supported for the detected input type and installed local engines. No action is ever displayed merely because a format name sounds compatible or as a placeholder.

## Architectural Layers

1. **Presentation Layer (`src/`)**
   - Floating transparent desktop orb with frameless window and macOS always-on-top toggle.
   - Circular radial action layout with dynamic category filtering (`Formats`, `Transforms`, `Optimize`, `Archive`, `Utilities`).
   - Contextual micro-parameter controls for operations requiring user input (e.g., quality slider, resize scale presets) directly on the orb without opening full settings screens.
   - Respects macOS `prefers-reduced-motion` settings.

2. **Capability Registry (`src-tauri/src/registry.rs`)**
   - Central authority in Rust determining available actions based on file metadata, MIME signatures, and installed local processing engines.
   - Each action definition provides unique ID, user label, detail, symbol, category, engine ID, input/output formats, naming rules, and parameter schemas.
   - Never exposes unsupported operations (e.g., raster-to-SVG vectorization or HEIC encoding without local hardware encoder).

3. **Processing Engines (`src-tauri/src/engines/`)**
   - **`image_convert`**: Multi-format image conversion (PNG, JPEG, WebP, BMP, TIFF, GIF) with configurable quality and alpha channel flattening to clean background color (preventing black artifacts).
   - **`image_transform`**: Local pixel transformations including 90°/180°/270° rotation, horizontal/vertical flipping, Lanczos3 dimension and percentage scaling, square center-cropping, grayscale conversion, and EXIF/metadata stripping.
   - **`pdf`**: Pure-Rust PDF 1.4 document generator supporting direct DCTDecode JPEG stream passthrough (zero-loss, instantaneous) and Zlib Deflate compression for raster images.
   - **`archive`**: Hardened ZIP preview and extraction with path traversal protection (`enclosed_name`), symbolic link rejection, entry count limits (10,000 entries), and size bomb limits (2 GB total, 512 MB single file).
   - **`metadata`**: Magic byte detection (PNG, JPEG, WebP, GIF, BMP, TIFF, PDF, ZIP) and safe header inspection without loading full pixel buffers into RAM.

4. **Output Safety Manager (`src-tauri/src/output.rs`)**
   - Strict preservation of original files.
   - Conflict-safe unique filename resolution (`stem (2).ext`).
   - Atomic temporary file writing (`.orbit-<pid>-<nanos>.tmp`) with automatic RAII cleanup on failure or cancellation.

## Offline Guarantee

- Zero cloud dependencies, zero external APIs, zero telemetry, zero analytics, zero AI models.
- All fonts and assets bundled locally.
- Strict Content Security Policy (CSP).
- The packaged app operates identically with network interfaces disabled.
