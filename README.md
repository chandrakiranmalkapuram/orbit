# ORBIT 🪐

> **Drop it. Transform it. Done.**  
> A minimal, private, floating companion for your Mac.

ORBIT floats quietly on your screen. Whenever you need to convert an image, merge photos into a clean PDF, or unpack an archive, just drag and drop your files onto the orb. Everything happens instantly on your computer.

---

## ✨ How It Works

It’s as easy as **1, 2, 3**:

```
 1. Drag & Drop          2. Pick an Action          3. Done!
┌──────────────┐         ┌──────────────┐         ┌──────────────┐
│  Photo 1 🖼️  │  ───►   │  [ Combine ] │  ───►   │  Saved right │
│  Photo 2 🖼️  │  drop   │  [ to JPEG ] │  click  │  beside your │
│  Photo 3 🖼️  │  files  │  [ to PNG  ] │         │  originals   │
└──────────────┘         └──────────────┘         └──────────────┘
```

1. **Drop your files** onto the floating orb (or click **Choose Files**).
2. **Choose what you want to do** from the buttons around the orb.
3. **Done!** Your new file is saved right next to the original without ever touching or deleting your original files.

---

## 🎨 What You Can Do

### 🖼️ Photos & Images
- **Change formats**: Switch between PNG, JPEG, and WebP in one click.
- **Quick edits**: Rotate, flip, convert to clean black & white, or crop to a square.
- **Resize & Compress**: Make images smaller for sharing or emailing.
- **Privacy clean**: Remove hidden camera and location details from photos.

### 📄 PDFs
- **Combine photos into one PDF**: Select multiple pictures (in any order) and combine them into a single, beautiful multi-page PDF document.
- **Turn PDFs into images**: Convert your PDF pages into sharp JPG or PNG images.
- **Extract text**: Pull out readable text from your documents into a clean text file.
- **Split pages**: Separate a multi-page PDF into individual single-page documents.

### 📦 Zip Archives
- **Safe Unzip**: Extract zip files with one click.
- **Peek Inside**: Preview what’s inside an archive before unpacking it.

---

## 🔒 Private & Offline by Design

- **No Internet Required**: ORBIT works completely offline. You can disconnect your Wi-Fi and it works just the same.
- **100% Private**: None of your photos, documents, or data ever leave your Mac. No accounts, no uploads, no tracking.
- **Safe Outputs**: Your original files are never altered or overwritten. New files get a clean name (like `Photo (2).jpg`) so you never lose anything.

---

## 🧭 Controls & Shortcuts

| Button | What it does |
|:---:|:---|
| 📌 **Pin** | Keeps ORBIT floating on top of your work windows while you drag files. |
| ☼ / ☾ **Theme** | Switch between sleek Dark mode and clean Light mode. |
| **Top Bar** | Click and drag anywhere on the top pill to move ORBIT anywhere on your screen. |
| ✕ **Close** | Closes the floating orb. |

---

## 🚀 Running ORBIT on Your Mac

### Quick Launch
If you have the app on your Mac, simply open it:
```bash
open "src-tauri/target/release/bundle/macos/ORBIT.app"
```
*(Or double-click `ORBIT.app` in Finder)*.

### For Developers
To run and test ORBIT in live development mode:
```bash
# 1. Install dependencies
npm install

# 2. Start ORBIT
npm run tauri dev
```

---

<p align="center">
  <b>ORBIT</b> — Designed for effortless macOS file workflows.
</p>
