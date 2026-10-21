# ORBIT architecture

## Product rule

The interface may only show actions registered as supported for the detected input type. No action should be displayed merely because a format name sounds compatible.

## Layers

1. **Presentation (`src/`)** — React components, circular interaction, animation, theme, accessible status and errors.
2. **Capability registry (`src/lib/actions.ts`)** — maps supported input kinds to real operations. Keep unsupported future actions out of the registry until implemented.
3. **Native command boundary (`src-tauri/src/lib.rs`)** — validates paths and inputs, performs local processing, returns structured results, and avoids shelling out unless a specific platform integration requires it.
4. **Conversion engines** — libraries bundled with the app. Every engine must be audited for licensing, supported formats, memory use, and offline behaviour.
5. **Output manager** — preserves originals, chooses non-conflicting output paths, writes temporary output before finalizing, and reports errors clearly.

## Offline guarantee

Do not add remote API calls, web fonts, external scripts, remote stylesheets, analytics, telemetry, or model downloads. Keep CSP restrictive. Before release, run the packaged app with network access disabled and inspect its network activity. Development mode uses a local Vite server, but production should load the bundled frontend only.

## Security priorities

- Treat file contents and archive paths as untrusted.
- Use file signatures in addition to extensions where practical.
- Reject archive traversal paths and symbolic links.
- Enforce entry-count and expanded-size limits.
- Avoid overwrites and use exclusive file creation where possible.
- Never execute content from user-supplied files.
- Keep CPU- and memory-intensive work off the UI thread.

## Phased delivery

1. Foundation and offline UI.
2. Verified file inspection and supported image conversions.
3. Secure ZIP listing and extraction.
4. Image-to-PDF and PDF rendering.
5. Document conversion with a bundled local engine.
6. Batch processing and cancellation.
7. Optional media engines, only if bundle size and licensing are acceptable.
8. macOS packaging, signing, notarization, offline and security testing.
