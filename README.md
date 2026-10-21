# ORBIT

**Drop it. Transform it. Done.**

ORBIT is a local-first macOS file utility built with Tauri 2, React, TypeScript, and Rust. Its design centres on a single interactive circle rather than a dashboard.

## Privacy principles

- File processing is intended to happen on the user's computer.
- No accounts, analytics, telemetry, cloud conversion, or AI are part of the product scope.
- Runtime UI assets must be bundled locally; do not add remote fonts, scripts, stylesheets, or APIs.
- The app should be tested with networking disabled before release.

## Current implementation scope

The starter implementation includes:

- A circular drag-and-drop interface with light/dark appearance.
- Local file inspection for common file signatures and file extensions.
- Local image conversion to PNG, JPEG, and WebP using the Rust `image` crate.
- ZIP archive listing and extraction with path validation, symlink rejection, output naming, and extraction limits.
- Reveal output in Finder.

Not yet implemented: image-to-PDF, PDF page rendering, DOCX-to-PDF, selected ZIP extraction, password-protected archives, batch processing, audio/video conversion, progress events, and Finder Quick Actions. These must not be advertised as supported until implemented and tested.

## Prerequisites

- Node.js 20 or newer and npm
- Rust toolchain (`rustup` / Cargo)
- macOS development prerequisites required by Tauri 2

## Development

```sh
npm install
npm run tauri dev
```

## Build a distributable macOS app

```sh
npm install
npm run tauri build
```

Tauri's output is created under `src-tauri/target/release/bundle/` when the required toolchain and platform dependencies are available. Signing and notarization are separate release steps.

## Before public release

1. Build and test on macOS.
2. Test conversions and extraction against malformed, large, and unusual files.
3. Verify no network requests are made by the packaged app while processing files.
4. Audit bundled dependencies and their licenses.
5. Test on Apple Silicon and, if supported, Intel Macs.
6. Sign and notarize the app for a smoother download-and-open experience.
