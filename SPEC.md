# JJ PDF Studio: Project Specification

**Version:** 0.2 (adds security, convert and edit tools) · **Date:** 2026-10-19 · **Owner:** James & James LLP

A free, open source, offline Windows desktop tool for everyday PDF work: merge, split, extract, reorder, rotate, delete and insert pages, resize pages, compress, convert between images and PDF, unlock and password-protect, add page numbers and watermarks, and edit document properties. It ships as one portable `.exe` that runs without installation.

---

## 1. Goals and non-goals

### Goals

- **Portable single exe.** Double-click to run. No installer, no admin rights, no runtime downloads.
- **Lightweight.** Target under 15 MB for the exe, under 150 MB RAM for a typical 50-page document, cold start under 2 seconds.
- **Fully offline.** No network calls, no telemetry, no update pings. Files never leave the machine.
- **Safe by default.** Originals are never overwritten. Every operation writes a new file (default name `original-name (edited).pdf`, user can change it).
- **Professional, calm UI** in the James & James LLP brand colour `#2C388E`, light and dark themes.
- **Open source** on GitHub under a permissive license, with reproducible builds from GitHub Actions.
- **Windows 10 (1809+) and Windows 11, x64.** ARM64 is a later nice-to-have.

### Non-goals (v1)

- Editing text or images inside a page, annotations, form filling, digital signatures.
- OCR, PDF/A conversion, conversion to or from Word/Excel.
- Recovering or guessing unknown passwords. Unlock works only when the PDF opens without a password or the user types the password.
- OCR (making scanned PDFs searchable). Planned for v1.x using the OCR engine built into Windows 10/11.
- True redaction, digital signatures and PDF to Word. These are hard to do safely and are left out on purpose.
- macOS and Linux builds (Tauri makes them possible later, but they are not tested or supported in v1).
- Cloud sync, accounts, auto-update.

---

## 2. License

### Recommendation: Apache-2.0 for the code, logo kept separately

|                                            | MIT | Apache-2.0          |
| ------------------------------------------ | --- | ------------------- |
| Permissive, allows commercial use          | Yes | Yes                 |
| Explicit patent grant from contributors    | No  | **Yes**             |
| Says the license gives no trademark rights | No  | **Yes (section 6)** |
| Compatible with all dependencies below     | Yes | Yes                 |

**Why Apache-2.0:** it is just as permissive as MIT, but it adds a patent grant and explicitly excludes trademark rights. That second point matters for a branded tool: anyone may fork the code, but they may not ship it as "JJ PDF Studio" with the James & James LLP logo. Tauri itself is dual MIT/Apache-2.0, so this sits naturally in the ecosystem.

**Logo and brand assets:** stored in `assets/brand/` with their own `assets/brand/LICENSE.md` stating "All rights reserved, James & James LLP. Not covered by the project license." Add a short `TRADEMARKS.md` explaining forks must rebrand.

### Dependency licenses (all permissive, no AGPL/GPL)

| Purpose                        | Library                               | License          |
| ------------------------------ | ------------------------------------- | ---------------- |
| App shell                      | Tauri 2 (`tauri`, `tauri-build`)      | MIT / Apache-2.0 |
| File dialogs                   | `tauri-plugin-dialog`                 | MIT / Apache-2.0 |
| PDF read/write/manipulate      | `lopdf`                               | MIT              |
| Image decode/encode/resize     | `image` (with `jpeg`, `png` features) | MIT / Apache-2.0 |
| Errors, serialisation          | `thiserror`, `anyhow`, `serde`        | MIT / Apache-2.0 |
| Parallel work                  | `rayon`                               | MIT / Apache-2.0 |
| AES-256 encryption (only if lopdf lacks it) | RustCrypto `aes`, `cbc`, `sha2`, `rand` | MIT / Apache-2.0 |
| Thumbnail rendering (frontend) | `pdfjs-dist` (Mozilla pdf.js)         | Apache-2.0       |
| UI framework                   | Svelte 5 + Vite + TypeScript          | MIT              |
| Drag-and-drop sorting          | `sortablejs`                          | MIT              |
| Icons                          | Lucide                                | ISC              |
| Font                           | Inter (bundled, no Google Fonts call) | OFL-1.1          |

**Deliberately avoided:** MuPDF / PyMuPDF (AGPL), iText (AGPL), Ghostscript (AGPL), qpdf binaries (Apache-2.0 but a native exe we would have to bundle and spawn, breaking the single-exe goal).

**Why not PDFium for rendering?** PDFium (BSD/Apache) renders more accurately than pdf.js, but it is a ~5 MB native DLL that must sit beside the exe or be extracted at runtime, which works against the single portable file. pdf.js runs inside WebView2, which is already on the machine, so thumbnails cost almost nothing in size. If rendering accuracy ever becomes a problem, `pdfium-render` (MIT/Apache) is the fallback, with the DLL embedded and extracted to `%LOCALAPPDATA%\JJPDFStudio\` on first run.

A `THIRD_PARTY_NOTICES.md` is generated in CI with `cargo about` (Rust) and `license-checker` (npm) and shipped in the release zip and the About screen.

---

## 3. Architecture

```
┌──────────────────────────── JJPDFStudio.exe ────────────────────────────┐
│                                                                          │
│  Frontend (WebView2)                     Backend (Rust)                  │
│  Svelte + TS                              Tauri commands                  │
│  ┌──────────────────────┐   invoke()    ┌──────────────────────────────┐ │
│  │ Tool views, page grid │ ───────────▶ │ commands/*.rs (thin layer)   │ │
│  │ pdf.js thumbnails     │ ◀─────────── │   validates input, spawns    │ │
│  │ job progress UI       │   events      │   work, emits progress       │ │
│  └──────────────────────┘               │ core/  (pure Rust, no Tauri) │ │
│                                          │   merge, split, pages,       │ │
│                                          │   resize, compress, images   │ │
│                                          │   built on lopdf + image     │ │
│                                          └──────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────────────┘
```

**Principles**

- **All PDF writing happens in Rust.** The frontend only renders previews and describes what the user wants (a list of operations). This keeps file handling fast and testable.
- **`core` is a plain Rust library crate** with no Tauri dependency, so it can be unit tested with `cargo test` on any OS and reused later (for example a CLI).
- **Operation list model.** The page workspace builds a list like `[{src: 0, page: 3, rotate: 90}, {blank: "A4"}, ...]`. Saving sends that list to one Rust command that assembles the output. Undo/redo is just an edit history of that list on the frontend, so nothing is written until Save.
- **Long jobs run on a background thread** (`tauri::async_runtime::spawn_blocking`) and emit `progress` events (`{jobId, done, total, message}`). A Cancel button sets an `AtomicBool` the job checks between pages.
- **Previews:** Rust reads the file once and returns bytes to the frontend (`tauri::ipc::Response` for zero-copy binary). pdf.js renders thumbnails lazily as they scroll into view (IntersectionObserver), at 160 px width, cached in memory as bitmaps.
- **Security:** strict CSP (`default-src 'self'; img-src 'self' blob: data:; worker-src 'self' blob:`), no remote URLs, Tauri capabilities limited to `dialog` and the app's own commands. Rust only touches paths the user picked via a dialog or drag-and-drop.

### Folder structure

```
jj-pdf-studio/
├─ .github/
│  ├─ workflows/
│  │  ├─ ci.yml               # lint + test + build on every PR
│  │  └─ release.yml          # build and publish exe on tag v*
│  ├─ ISSUE_TEMPLATE/         # bug_report.yml, feature_request.yml
│  └─ pull_request_template.md
├─ assets/
│  └─ brand/                  # logo.svg, logo.png (placeholder until supplied), LICENSE.md
├─ crates/
│  └─ pdf-core/               # pure Rust library, no Tauri
│     ├─ src/
│     │  ├─ lib.rs
│     │  ├─ document.rs       # load, save, page tree helpers
│     │  ├─ assemble.rs       # build output from an operation list (merge/reorder/delete/insert/rotate)
│     │  ├─ split.rs
│     │  ├─ resize.rs
│     │  ├─ compress.rs
│     │  ├─ images.rs         # images to PDF
│     │  └─ error.rs
│     └─ tests/               # integration tests with fixture PDFs
├─ src-tauri/
│  ├─ src/
│  │  ├─ main.rs
│  │  ├─ commands/            # one file per tool, thin wrappers over pdf-core
│  │  └─ jobs.rs              # background jobs, progress events, cancel
│  ├─ icons/                  # generated from assets/brand via `tauri icon`
│  ├─ capabilities/default.json
│  ├─ tauri.conf.json
│  ├─ Cargo.toml
│  └─ .cargo/config.toml      # static CRT
├─ src/                       # frontend
│  ├─ lib/
│  │  ├─ components/          # Sidebar, PageGrid, PageThumb, Dropzone, Toast, Dialog...
│  │  ├─ tools/               # Merge.svelte, Pages.svelte, Split.svelte, ...
│  │  ├─ stores/              # documents, history (undo/redo), theme, jobs
│  │  ├─ pdf/                 # pdf.js wrapper, thumbnail cache
│  │  └─ api.ts               # typed wrappers around invoke()
│  ├─ styles/tokens.css       # design tokens
│  ├─ App.svelte
│  └─ main.ts
├─ tests/fixtures/            # small sample PDFs and images (public domain)
├─ Cargo.toml                 # workspace: crates/pdf-core, src-tauri
├─ package.json
├─ LICENSE                    # Apache-2.0
├─ TRADEMARKS.md
├─ THIRD_PARTY_NOTICES.md     # generated
├─ README.md
├─ CONTRIBUTING.md
├─ CODE_OF_CONDUCT.md         # Contributor Covenant
├─ SECURITY.md
└─ CHANGELOG.md
```

---

## 4. UI layout and design system

### Layout

```
┌───────────────────────────────────────────────────────────────────┐
│ [logo] JJ PDF Studio                      ☀/☾   ?   ⋯            │  title bar (custom, 44 px)
├──────────────┬────────────────────────────────────────────────────┤
│  ORGANISE    │  Merge PDFs                                [Save ▸] │  tool header
│  ▣ Page Editor│ ────────────────────────────────────────────────── │
│  ▣ Merge     │                                                     │
│  CONVERT     │   ┌────┐ ┌────┐ ┌────┐ ┌────┐ ┌────┐                │
│  ▣ Images→PDF│   │ 1  │ │ 2  │ │ 3  │ │ 4  │ │ +  │                │  thumbnail workspace
│  ▣ PDF→Images│   └────┘ └────┘ └────┘ └────┘ └────┘                │  (drag to reorder)
│  OPTIMISE    │                                                     │
│  ▣ Resize    │        Drop PDFs here or click to browse            │
│  ▣ Compress  │                                                     │
│  SECURITY    │                                                     │
│  ▣ Unlock    │                                                     │
│  ▣ Protect   │                                                     │
│  EDIT        │                                                     │
│  ▣ Watermark │                                                     │
│  ▣ Properties│                                                     │
│              ├─────────────────────────────────────────────────────┤
│  ⓘ About     │  Options panel (tool-specific)   12 pages · 3.4 MB  │  footer / options
└──────────────┴────────────────────────────────────────────────────┘
```

- **Sidebar (220 px, collapsible to 64 px icons):** Page Editor at the top as the main workspace (see 5.15), then the whole-file tools. Rotate, Split, Extract and Page numbers have no sidebar entries of their own; they live inside Page Editor with all their options.
- **Workspace:** large thumbnail grid with page numbers. Click selects, Shift/Ctrl multi-select, drag to reorder, hover shows rotate and delete buttons. Double-click opens a larger preview.
- **Options panel:** tool-specific settings at the bottom (or right on wide screens ≥ 1400 px).
- **Primary action** always top right in brand colour: "Save merged PDF", "Split", "Compress".
- **Home screen** (no file open): a grid of tool cards with icon, name and one-line description, plus a big drop zone. Drop a file anywhere and it opens in the Pages tool by default.
- **Feedback:** progress bar in the tool header during jobs with Cancel; success toast with "Open file" and "Show in folder" buttons; errors in plain language ("This PDF needs a password to open. Enter it to continue.").
- **Sidebar grouping (10 entries):** _Organise_ (Page Editor, Merge), _Convert_ (Images to PDF, PDF to Images), _Optimise_ (Resize, Compress), _Security_ (Unlock, Protect), _Edit_ (Watermark, Properties). The home screen still shows a card for every task, including Split, Extract, Rotate and Page numbers; those cards open Page Editor with that panel already open, so people who look for "Split" still find it.
- **Keyboard:** Ctrl+O open, Ctrl+S save, Ctrl+Z / Ctrl+Y undo/redo, Delete removes selected pages, Ctrl+A select all, R / Shift+R rotate right/left, arrows move selection.
- **Window:** min 960×640, remembers size and position and last theme in `%APPDATA%\JJPDFStudio\settings.json` (portable mode: if a file named `portable` sits next to the exe, settings live beside the exe instead).

### Design tokens (`src/styles/tokens.css`)

Brand primary is **`#2C388E`** (deep indigo). The scale below is derived from it.

| Token          | Light         | Dark      | Use                                              |
| -------------- | ------------- | --------- | ------------------------------------------------ |
| `--brand-50`   | `#EEF0FA`     |           | selected thumbnail fill, hover                   |
| `--brand-100`  | `#D5D9F1`     |           | subtle badges                                    |
| `--brand-300`  | `#8C95D6`     |           | dark-mode primary text links                     |
| `--brand-500`  | `#3F4DB8`     |           | focus rings                                      |
| `--brand-600`  | **`#2C388E`** |           | primary buttons, active sidebar item             |
| `--brand-700`  | `#232D72`     |           | primary hover                                    |
| `--brand-800`  | `#1A2256`     |           | primary pressed                                  |
| `--primary`    | `#2C388E`     | `#7D88E0` | primary action (dark mode lightens for contrast) |
| `--on-primary` | `#FFFFFF`     | `#0E1330` | text on primary                                  |
| `--bg`         | `#F6F7FB`     | `#0F1220` | window background                                |
| `--surface`    | `#FFFFFF`     | `#181C2E` | panels, cards                                    |
| `--surface-2`  | `#EEF0F6`     | `#22273D` | sidebar, inputs                                  |
| `--border`     | `#DDE1EC`     | `#2E3450` | dividers                                         |
| `--text`       | `#161A2E`     | `#E8EAF4` | body text                                        |
| `--text-muted` | `#5B6178`     | `#9AA0B8` | secondary text                                   |
| `--success`    | `#1F8A5B`     | `#3CC48A` |                                                  |
| `--warning`    | `#B7791F`     | `#E3A948` |                                                  |
| `--danger`     | `#C2353A`     | `#F0686C` | delete, errors                                   |

`#2C388E` on white has a contrast ratio of about 10:1, well above WCAG AA. In dark mode the primary is lightened so it stays readable against the dark surface.

- **Type:** Inter (bundled). 13 px body, 12 px captions, 15 px tool headers, 20 px page titles; weights 400/500/600.
- **Spacing:** 4 px grid (4, 8, 12, 16, 24, 32).
- **Radius:** 6 px controls, 10 px cards, 4 px thumbnails.
- **Elevation:** one soft shadow for cards (`0 1px 2px rgba(22,26,46,.06), 0 2px 8px rgba(22,26,46,.06)`), one for dialogs.
- **Motion:** 120 to 180 ms ease-out for hover and reorder; respect `prefers-reduced-motion`.
- **Icons:** Lucide, 18 px, 1.75 stroke.
- **Theme:** follows Windows setting by default, with a manual toggle.

### Logo placeholder

- `assets/brand/logo.svg` and `logo.png` (1024×1024) start as a simple placeholder: a rounded square in `#2C388E` with "JJ" in white.
- When the real logo arrives: replace both files, run `npm run tauri icon assets/brand/logo.png` to regenerate all app icon sizes, and rebuild. The title bar and About screen read from `assets/brand/logo.svg`, so nothing else changes.

---

## 5. Feature behaviour

All tools: input via drag-and-drop or file dialog; output via a Save dialog prefilled with a suggested name in the source folder; never overwrite the source (if the user picks the source path, show "Choose a different name, the original is kept safe"). Encrypted PDFs that need a password to open prompt for it (see 5.9); PDFs that open freely but carry restrictions load normally in every tool. Damaged PDFs: lopdf tries to recover the xref; if it fails, show "This file looks damaged and could not be opened."

### 5.1 Merge

- Add multiple PDFs (and optionally images, converted on the fly with Images-to-PDF defaults).
- **File view** (default): one card per file showing first-page thumbnail, name, page count; drag to reorder files; remove button.
- **Page view** toggle: expands all pages into the grid so individual pages can be reordered or removed across files before merging.
- Options: "Add bookmarks for each file" (on by default, bookmark title = file name without extension).
- Implementation: load each doc with lopdf, renumber object IDs to avoid collisions (`renumber_objects_with(max_id)`), graft page objects into one page tree, carry over each file's outline under a per-file bookmark when possible. Form fields from different files may clash; v1 keeps them but does not guarantee forms stay interactive (documented limitation).

### 5.2 Pages (organise: reorder, rotate, delete, insert blank)

- Opens one PDF into the thumbnail grid.
- **Reorder:** drag single or multiple selected pages.
- **Rotate:** per page or selection, 90° steps. Stored by updating the page's `/Rotate` (lossless, content untouched).
- **Delete:** selected pages; at least one page must remain.
- **Insert blank page:** "+" between thumbnails or "Insert blank" button; size defaults to the neighbouring page's size, with a dropdown (Same as previous, A4, A3, A5, Letter, Legal). Creates an empty page with `/MediaBox` and empty content.
- Undo/redo for every action. Nothing is written until Save.
- Status footer: page count, selection count, source file size.

### 5.3 Split

Modes:

1. **Every N pages** (default N = 1, one file per page).
2. **By ranges:** text input like `1-3, 4-8, 9-end`; each range becomes one file. Validates live and highlights ranges on the grid in alternating tints.
3. **At selected pages:** click the gaps between thumbnails to place split markers.

- Output: choose a folder; files named `name_part01.pdf`, `name_part02.pdf` (or `name_p1-3.pdf` for ranges). Warn before writing more than 100 files.
- Implementation: for each output, clone the document, delete the pages not in the range, prune unused objects, save.

### 5.4 Extract pages

- Select pages on the grid or type ranges (`2, 5-7`).
- Two outputs: **one PDF with all selected pages** (default) or **one PDF per selected page**.
- Shares the split engine.

### 5.5 Rotate

- Quick tool for whole-document or range rotation: "Rotate all", "Rotate odd pages", "Rotate even pages", "Rotate pages…" with 90° right, 90° left, 180°.
- Also auto-detect option: "Make all pages portrait" or "Make all pages landscape" (rotates pages whose effective width/height don't match).
- Lossless, via `/Rotate`.

### 5.6 Resize pages

- Target: A4, A3, A5, Letter, Legal, or custom (mm or inches). Orientation: Auto (match each page), Portrait, Landscape.
- Mode:
  - **Fit (default):** scale content uniformly to fit inside the target with margins, centred. Never distorts.
  - **Fill:** scale to cover the target, cropping overflow.
  - **Change page size only:** set the new size without scaling content (content stays at its original size, centred or anchored top-left).
- Margin option (0 to 20 mm, default 0).
- Apply to all pages or a selection.
- Implementation: compute scale `s` and offset `(tx, ty)` from the page's effective box (CropBox, else MediaBox, accounting for `/Rotate`). Wrap the existing content streams with `q s 0 0 s tx ty cm … Q` (by prepending and appending small content streams to the page's `/Contents` array, so original streams are untouched). Set new `/MediaBox` and remove `/CropBox`, `/BleedBox`, `/TrimBox`. Annotations (links, comments) get their `/Rect` transformed by the same matrix.
- Limitation documented: some interactive form fields may misalign after scaling.

### 5.7 Compress

Three presets plus a details panel.

| Preset                 | What it does                                                                                                                                                                               | Typical result                                         |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------ |
| **Lossless**           | Removes unused objects, merges duplicate objects, compresses uncompressed streams with Flate, packs objects into object streams, strips metadata junk (thumbnails `/Thumb`, `/PieceInfo`). | 0 to 30% smaller; nothing visibly changes              |
| **Balanced** (default) | Lossless + images above 200 DPI are downsampled to 150 DPI and re-encoded as JPEG quality 75.                                                                                              | Often 40 to 80% smaller on scanned or photo-heavy PDFs |
| **Strong**             | Lossless + downsample to 100 DPI, JPEG quality 60, optional convert colour images to grayscale.                                                                                            | Smallest; visible softening of images                  |

**How it works honestly:**

- Image effective DPI is computed from the image pixel size and the size it is drawn at on the page (parsing the content stream's `cm` and `Do` operators). If an image is drawn in several places, the largest use wins.
- Supported images to recompress: `DCTDecode` (JPEG) and `FlateDecode` RGB/Gray 8-bit images, with or without `/SMask`. These cover the large majority of real-world PDFs (scans, photos, office exports).
- **Skipped, left untouched:** JBIG2, JPX (JPEG 2000), CCITT fax images, CMYK and ICC-based colour spaces beyond simple cases, indexed palettes, images with `/Decode` arrays or masks we don't handle, inline images. They pass through unchanged rather than risk breaking the file.
- **Fonts are not subset or removed** in v1 (that needs a font parser; it is a candidate for v2).
- **Text-only or vector PDFs will barely shrink**, because there is little to compress. The UI says so when the result saves under 5%.
- **Never makes a file bigger:** each image keeps its original stream if the new one is not smaller, and if the final file is not smaller than the input, the tool says "This PDF is already well optimised" and offers to keep the original.
- Result dialog shows before/after size and percentage, with "Open" and "Show in folder".
- Ghostscript-class results (font subsetting, full colour management) are explicitly out of scope because Ghostscript is AGPL.

### 5.8 Images to PDF

- Add multiple JPG, PNG (also BMP, GIF first frame, TIFF first page, WebP via the `image` crate). Drag to reorder; sort by name or date buttons; rotate an image 90°.
- Options:
  - **Page size:** Fit to image (page = image size at its DPI, default 96 if none), A4, Letter, A3, custom.
  - **Orientation:** Auto (per image), Portrait, Landscape.
  - **Margin:** None, Small (10 mm), Large (20 mm), custom.
  - **Fit:** Fit inside (default), Fill page (crop), Original size (centred).
  - **Quality:** Keep original (default) or Reduce (re-encode JPEG 80, max 2000 px long edge).
- Output: one PDF (default) or one PDF per image.
- Implementation: JPEGs are embedded directly as `DCTDecode` streams with no re-encoding (fast, no quality loss). PNG and others are decoded and stored as Flate-compressed RGB, with alpha split into an `/SMask`. EXIF orientation is honoured.

### 5.9 Unlock (remove password and restrictions)

PDFs can carry two kinds of protection, and the tool explains which one a file has:

- **Open password (user password):** the file can't be viewed without it. The user types the password; the tool decrypts and saves an unprotected copy. There is no guessing or cracking.
- **Restrictions only (owner password):** the file opens freely but blocks copying, printing or editing. This is the usual reason text can't be copied. Because the file is readable without a password, the tool can decrypt it and save a copy with no restrictions, without asking for the owner password.
- Before saving, show one line: "Only remove protection from files you own or have permission to change."
- **Copying still fails on some files after unlocking**, and the tool should say why when it detects it:
  - **Scanned PDFs** are pictures of text, so there is no text to copy. Detect pages with no text operators and one full-page image, and suggest OCR (v1.x).
  - **Fonts without a Unicode map** (missing `/ToUnicode`) copy as garbled characters. Only OCR fixes that.
- Implementation: `lopdf` decrypts standard security handler files (RC4 40/128-bit, AES-128, AES-256). Check AES-256 (revision 6) support in the pinned lopdf version at M6 start; if it's missing, add it with the RustCrypto crates. Save without `/Encrypt`.

### 5.10 Protect (add password)

- Set an open password, and optionally restrictions (disallow printing, copying, editing) with a separate owner password.
- Always AES-256 (PDF 2.0 / revision 6); no RC4 or 40-bit options, because those are easy to break.
- Password entered twice; a strength hint; a clear warning that a forgotten password cannot be recovered by this tool.
- Implementation: use lopdf's encryption if the pinned version supports AES-256 writing; otherwise implement the revision 6 security handler with RustCrypto `aes`, `cbc`, `sha2` and `rand` (all MIT/Apache-2.0). Test that output opens in Edge, Chrome and Acrobat Reader.

### 5.11 PDF to Images

- Export all pages or a selection as PNG (default) or JPG (quality slider, default 85).
- Resolution: 72, 150 (default), 300 DPI, or custom.
- Output to a folder as `name_p001.png`, or one ZIP file.
- Implementation: render with pdf.js in the frontend (already bundled, no extra size) to an offscreen canvas at the target DPI, encode to PNG/JPG blobs, and send bytes to Rust to write. Render one page at a time to keep memory low.

### 5.12 Page numbers

- Position: 6 spots (top/bottom × left/centre/right), margin, font size, colour.
- Format: `1`, `Page 1`, `Page 1 of N`, `1 / N`, or custom text with `{n}` and `{total}`.
- Start number and page range (for example skip the cover page).
- Implementation: append a small content stream per page drawing text with a standard 14 font (Helvetica), which needs no font embedding. Position computed from the page's effective box and `/Rotate`.

### 5.13 Watermark / stamp

- **Text watermark:** text, font size, colour, opacity, rotation (default 45°), position (centre or tiled), over or under page content.
- **Image stamp:** a PNG or JPG (for example the James & James LLP logo), size, opacity, position (corners or centre).
- Page range.
- Implementation: a Form XObject per watermark, drawn on each page from an extra content stream; opacity via an `/ExtGState` with `/ca`. "Under content" prepends the stream instead of appending it.
- Note in the UI: a watermark added this way can be removed by someone with PDF editing tools. It is a visible label, not security.

### 5.14 Properties (metadata)

- View and edit title, author, subject, keywords, creator; view producer, PDF version, page count, page sizes, encryption status, file size.
- "Remove all metadata" button for privacy before sharing (clears `/Info` and the XMP `/Metadata` stream).
- Implementation: direct `/Info` dictionary edit in lopdf; XMP is removed rather than edited in v1.

### 5.15 Page Editor (combined workspace)

**Name:** "Page Editor" replaces "Pages". It says what the screen does (edit the pages of one PDF), it reads well as the first sidebar item, and it matches what people know from Acrobat ("Organize Pages") and Stirling ("Page editor"). Internal route `/page-editor`, component `PageEditor.svelte`.

**What lives here:** reorder, rotate, delete, insert blank page (from 5.2), plus Rotate (5.5), Split (5.3), Extract (5.4) and Page numbers (5.12). Sections 5.2 to 5.5 and 5.12 remain the source of truth for each tool's behaviour; this section only changes where they appear. **No option, mode, setting or output format is removed or reduced by the move.**

**Layout**

```
┌───────────────────────────────────────────────────────────────────────┐
│ Page Editor · report.pdf · 24 pages · 3 selected        [Undo][Redo] [Save ▸] │
├───────────────────────────────────────────────────────────────────────┤
│ ⟲ ⟳ │ 🗑 │ ＋Blank │ ✂ Split │ ⇪ Extract │ # Page numbers │ Select ▾ │ Zoom ─○─ │  toolbar
├──────────────────────────────────────────────────────┬────────────────┤
│                                                      │ Options panel  │
│   ┌──┐ ┌──┐ ┌──┐ ┌──┐ ┌──┐ ┌──┐                       │ (context:      │
│   │1 │ │2 │ │3 │ │4 │ │5 │ │6 │      thumbnail grid   │  Split, Extract,│
│   └──┘ └──┘ └──┘ └──┘ └──┘ └──┘                       │  Rotate or Page │
│                                                      │  numbers)       │
└──────────────────────────────────────────────────────┴────────────────┘
```

- **Toolbar** (always visible): rotate left, rotate right, delete, insert blank, Split, Extract, Page numbers, a Select menu (All, None, Invert, Odd, Even, Range…, Landscape pages, Portrait pages), thumbnail zoom slider.
- **Right-click on pages:** the same actions, plus Rotate 180°, Duplicate, Move to start, Move to end, Insert blank before or after.
- **Options panel** (right, 320 px, collapsible): opens when the user picks Split, Extract, Rotate (advanced) or Page numbers and shows that tool's full settings. Only one panel is open at a time; closing it keeps the settings for the session.
- **Live preview on the grid:** split boundaries show as tinted groups with "Part 1, Part 2…" labels; extract selection is highlighted; page number placement shows as a small overlay on each thumbnail.
- **Two kinds of action**, made visible in the UI:
  - _Edits to this document_ (reorder, rotate, delete, insert blank, page numbers) go into undo/redo history and are written when the user clicks **Save**.
  - _Exports to new files_ (Split, Extract) run immediately from their panel with their own button ("Split into 4 files", "Extract 3 pages") and use the current edited state of the document, including unsaved edits. They don't change the open document.
- **Keyboard:** everything in 4 (Ctrl+Z/Y, Delete, R/Shift+R, Ctrl+A), plus Ctrl+D duplicate, Ctrl+Shift+S split panel, Ctrl+E extract panel, Ctrl+Shift+N page numbers panel, Esc closes the panel.

**Feature-parity checklist** (every line must still work after the move; turn each into a test or a manual check)

_Pages (5.2):_

- [ ] Drag to reorder single and multiple selected pages
- [ ] Rotate per page and per selection, 90° steps, lossless via `/Rotate`
- [ ] Delete selection; refuses to delete the last page
- [ ] Insert blank via "+" between thumbnails and via toolbar; sizes Same as previous, A4, A3, A5, Letter, Legal
- [ ] Undo/redo for every action; nothing written before Save
- [ ] Status footer: page count, selection count, file size

_Rotate (5.5):_

- [ ] Rotate all, odd pages, even pages, custom range
- [ ] 90° right, 90° left, 180°
- [ ] "Make all pages portrait" and "Make all pages landscape"

_Split (5.3):_

- [ ] Every N pages (default 1)
- [ ] By ranges with `end` keyword, live validation and grid highlighting
- [ ] At selected pages via split markers between thumbnails
- [ ] Output folder choice; `name_part01.pdf` and `name_p1-3.pdf` naming
- [ ] Warning above 100 output files

_Extract (5.4):_

- [ ] Select on grid or type ranges (`2, 5-7`), kept in sync both ways
- [ ] One PDF with all selected pages (default)
- [ ] One PDF per selected page

_Page numbers (5.12):_

- [ ] 6 positions, margin, font size, colour
- [ ] Formats `1`, `Page 1`, `Page 1 of N`, `1 / N`, custom with `{n}` and `{total}`
- [ ] Start number and page range (skip cover)
- [ ] Correct placement on rotated and mixed-size pages

_Shared:_

- [ ] Never overwrites the original; Save As prefilled
- [ ] Password prompt for protected files (5.9) works when opening in Page Editor
- [ ] Home screen cards for Split, Extract, Rotate and Page numbers open Page Editor with the right panel
- [ ] Old routes (`/pages`, `/split`, `/extract`, `/rotate`, `/page-numbers`) redirect to Page Editor with the matching panel, so nothing breaks

**Implementation notes**

- One document state store (the operation list from 3) drives the grid, the panels and Save. Split and Extract call the existing `pdf-core` functions with the current operation list applied first, so no PDF logic is rewritten; this milestone is a frontend refactor plus a thin "apply ops then split/extract" command.
- Keep each tool's settings form as its own component (`SplitOptions.svelte`, `ExtractOptions.svelte`, `RotateOptions.svelte`, `PageNumberOptions.svelte`) and mount it in the options panel, so the forms are moved, not rewritten.

### 5.16 About

- Logo, "JJ PDF Studio", version and build date, "Made by James & James LLP", links to GitHub repo and issue tracker (open in default browser), license name, and a "Third-party licenses" button showing the bundled notices.

---

## 6. Building a portable exe

### Prerequisites (developer machine)

- Windows 10/11 x64
- Rust stable (`rustup`, MSVC toolchain, Visual Studio Build Tools 2022 with "Desktop development with C++")
- Node.js 20 LTS
- WebView2 runtime (already on Windows 10 with Edge updates and on all Windows 11)

### Commands

```powershell
npm install
npm run tauri dev          # run with hot reload
npm run tauri build        # release build
```

The portable exe is at `src-tauri\target\release\jj-pdf-studio.exe`. The frontend is embedded in the exe, so this one file is the whole app. Rename to `JJPDFStudio-<version>-portable.exe` for release.

### Settings that make it portable and small

`src-tauri/.cargo/config.toml` (static C runtime, so no VC++ redistributable is needed):

```toml
[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "target-feature=+crt-static"]
```

`Cargo.toml` (workspace root) release profile:

```toml
[profile.release]
opt-level = "s"
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

`tauri.conf.json` highlights:

- `"bundle": { "active": true, "targets": ["nsis"] }`: also produces an optional installer for people who prefer one. The portable exe is the primary download.
- `"app": { "windows": [{ "decorations": false, "minWidth": 960, "minHeight": 640 }] }` for the custom title bar.
- `"bundle.windows.webviewInstallMode": { "type": "skip" }` for the portable build (WebView2 is present on all supported Windows versions); the NSIS installer can use `downloadBootstrapper`.
- CSP as in section 3.

If WebView2 is somehow missing (rare, for example stripped corporate images), the app shows a native message box with a link to Microsoft's WebView2 download instead of failing silently.

### Code signing and SmartScreen

Unsigned exes show a "Windows protected your PC" warning on first run. Options:

1. **SignPath Foundation** offers free code signing for qualifying open source projects. Recommended once the repo is public and has a first release.
2. A paid OV/EV certificate (or Azure Trusted Signing) if James & James LLP wants its own company name on the signature.
3. Until then, document "More info → Run anyway" in the README, and IT can allow-list the exe hash.

---

## 7. GitHub Actions

### `ci.yml` (every push and PR)

```yaml
name: CI
on: [push, pull_request]
jobs:
  test:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with: { node-version: 20, cache: npm }
      - uses: dtolnay/rust-toolchain@stable
        with: { components: 'clippy, rustfmt' }
      - uses: Swatinem/rust-cache@v2
      - run: npm ci
      - run: npm run lint && npm run check # eslint + svelte-check
      - run: cargo fmt --all -- --check
      - run: cargo clippy --workspace -- -D warnings
      - run: cargo test --workspace
      - run: npm run tauri build -- --no-bundle # proves the exe builds
```

### `release.yml` (on tag `v*`)

```yaml
name: Release
on:
  push:
    tags: ['v*']
permissions:
  contents: write
jobs:
  release:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with: { node-version: 20, cache: npm }
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: npm ci
      - run: cargo test --workspace
      - run: npm run tauri build
      - name: Prepare artifacts
        shell: pwsh
        run: |
          $v = "${{ github.ref_name }}"
          New-Item -ItemType Directory dist-release | Out-Null
          Copy-Item src-tauri/target/release/jj-pdf-studio.exe "dist-release/JJPDFStudio-$v-portable.exe"
          Copy-Item src-tauri/target/release/bundle/nsis/*.exe "dist-release/JJPDFStudio-$v-setup.exe"
          Get-FileHash dist-release/*.exe -Algorithm SHA256 |
            ForEach-Object { "$($_.Hash.ToLower())  $(Split-Path $_.Path -Leaf)" } |
            Out-File dist-release/SHA256SUMS.txt -Encoding ascii
      # (later) code signing step via SignPath goes here
      - uses: softprops/action-gh-release@v2
        with:
          draft: true # review notes, then publish
          generate_release_notes: true
          files: dist-release/*
```

Release process: update `CHANGELOG.md` and the version in `package.json`, `src-tauri/Cargo.toml` and `tauri.conf.json` (a small `npm run version <x.y.z>` script keeps them in sync), commit, `git tag v1.0.0`, `git push --tags`. CI builds a draft release; review and publish it.

---

## 8. README and CONTRIBUTING outlines

### README.md

1. Logo, name, one-line tagline ("Free, offline PDF tools for Windows."), badges (CI, release, license).
2. Screenshot / GIF of the main window.
3. Features list (all 14 tools, grouped as in the sidebar).
4. **Download:** link to latest release; portable exe vs installer; system requirements (Windows 10 1809+/11 x64); SmartScreen note.
5. **Privacy:** fully offline, no telemetry, never overwrites originals.
6. Usage quick start (3 or 4 steps with screenshots).
7. Limitations (compress limits, no password support yet, forms after merge/resize).
8. Building from source (link to CONTRIBUTING).
9. Roadmap (link to milestones / GitHub Projects).
10. License (Apache-2.0), trademarks note, third-party notices, credits ("Made by James & James LLP").

### CONTRIBUTING.md

1. Welcome and code of conduct link.
2. Ways to help: bug reports (template asks for Windows version, app version, a sample PDF if shareable), feature ideas, code, translations (later).
3. Dev setup: prerequisites, `npm install`, `npm run tauri dev`.
4. Project layout (short version of section 3).
5. Coding standards: `cargo fmt`, `cargo clippy -D warnings`, ESLint + Prettier, Svelte check; PDF logic goes in `crates/pdf-core` with tests; no network code; no new dependency without a permissive license (MIT, Apache-2.0, BSD, ISC, Zlib, OFL for fonts).
6. Tests: add a fixture to `tests/fixtures/` (must be your own or public domain) and an integration test for every bug fix.
7. Commits: Conventional Commits (`feat:`, `fix:`, `docs:`...).
8. Pull request checklist.
9. Release process (maintainers).
10. License of contributions: by submitting a PR you agree it is licensed under Apache-2.0 (inbound = outbound, no CLA).

---

## 9. Milestone plan

| Milestone                             | Scope                                                                                                                                                               | Done when                                                                                                 |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| **M0: Scaffold**                      | Repo, license files, Tauri 2 + Svelte + TS app, workspace with `pdf-core`, design tokens, sidebar and home screen shell, placeholder logo, CI and release workflows | `npm run tauri dev` shows the branded shell; CI green; a tag produces a draft release with a portable exe |
| **M1: Open and view**                 | Open PDFs via dialog/drop, pdf.js lazy thumbnails, selection, preview dialog, error handling, settings file, light/dark theme                                       | 300-page PDF opens and scrolls smoothly                                                                   |
| **M2: Pages tool**                    | Reorder, rotate, delete, insert blank, undo/redo, save-as with original protection                                                                                  | All operations round-trip and open correctly in Edge, Acrobat Reader and Chrome                           |
| **M3: Merge, Split, Extract, Rotate** | Merge with file/page view and bookmarks; split modes; extract; rotate tool                                                                                          | Integration tests for each; 20-file merge works                                                           |
| **M4: Images to PDF, Resize**         | Images tool with all options and EXIF; resize with fit/fill/size-only and annotation transform                                                                      | Visual checks on mixed-size documents                                                                     |
| **M5: Compress**                      | Lossless pipeline, image recompression presets, never-larger guarantee, result dialog                                                                               | Benchmarked on a set of scans, office exports and text PDFs; results in README                            |
| **M6: Security**                      | Unlock (open password and restrictions, scanned-page detection), Protect (AES-256), password prompt when opening files in any tool                                  | Round-trip tests with RC4, AES-128 and AES-256 fixtures; output opens in Edge, Chrome, Acrobat Reader    |
| **M7: Convert and edit**              | PDF to Images, Page numbers, Watermark (text and image), Properties                                                                                                 | Visual checks on rotated and mixed-size pages                                                             |
| **M8: Page Editor**                   | Combine Pages, Rotate, Split, Extract and Page numbers into one Page Editor screen (5.15): toolbar, right-click menu, options panel, live grid preview, route redirects, renamed sidebar with 10 entries | Every item in the 5.15 parity checklist passes; existing pdf-core tests still green |
| **M9: Polish and v1.0**               | Sidebar grouping, progress and cancel everywhere, keyboard shortcuts, About and third-party notices, real logo, README screenshots, accessibility pass, code signing (if approved) | v1.0.0 release published                                                                  |
| **v1.x**                              | OCR for scanned PDFs using the Windows built-in OCR engine (`Windows.Media.Ocr` via the `windows` crate, no download, adds an invisible text layer), N-up and booklet printing, flatten forms, reverse or interleave pages (for double-sided scans) |                                                                  |
| **v2 ideas**                          | Font subsetting in compress, ARM64 build, translations, command-line mode reusing `pdf-core`                                                                        |                                                                                                           |

---

## 10. First prompt for Claude in VS Code

Paste this into Claude in VS Code in an empty folder (or the freshly cloned empty repo). It covers M0 only, so the result is small enough to review properly.

```text
You are helping me build "JJ PDF Studio", a free, open source (Apache-2.0), offline Windows 10/11 desktop app for basic PDF work. The full spec is in SPEC.md in this folder; read it first and follow it. This task is Milestone M0 only: the scaffold. Do not implement any PDF features yet.

Stack (fixed): Tauri 2 (Rust backend) + Svelte 5 + Vite + TypeScript frontend. PDF logic will live in a separate pure-Rust library crate. Permissive dependencies only (MIT, Apache-2.0, BSD, ISC); never add AGPL/GPL code.

Please do the following:

1. Create a Cargo workspace at the repo root with two members: `crates/pdf-core` (library, no Tauri dependency, one placeholder function `version() -> &'static str` with a unit test) and `src-tauri` (the Tauri app). Add the release profile from SPEC section 6 (opt-level "s", lto, codegen-units 1, panic abort, strip) and `src-tauri/.cargo/config.toml` with `+crt-static` for x86_64-pc-windows-msvc.

2. Scaffold the Tauri 2 app with Svelte 5 + TypeScript + Vite. Product name "JJ PDF Studio", identifier "in.jjglobal.pdfstudio", binary name "jj-pdf-studio". Window: min 960x640, default 1280x800, decorations false (we draw a custom title bar with minimise/maximise/close and drag region). Add tauri-plugin-dialog. Set a strict CSP as in SPEC section 3 and keep capabilities minimal. Bundle target: nsis only. No network access anywhere.

3. Add `src/styles/tokens.css` with the full light and dark token set from SPEC section 4 (brand primary #2C388E), theme following the OS with a manual toggle stored in localStorage for now. Bundle the Inter font locally (no Google Fonts). Use lucide-svelte for icons.

4. Build the UI shell: custom title bar (logo + "JJ PDF Studio", theme toggle, window controls), collapsible left sidebar with the tools Merge, Pages, Split, Extract, Rotate, Resize, Compress, Images to PDF, and About at the bottom. Home screen with a tool card grid (icon, name, one-line description) and a large drop zone. Each tool route shows a placeholder view with its title and "Coming soon". About view shows logo, name, version (read from Tauri app API), "Made by James & James LLP", and license. Make it look polished and calm: 4px spacing grid, 6px/10px radii, subtle shadows, 120-180ms transitions, respects prefers-reduced-motion.

5. Add a placeholder logo at `assets/brand/logo.svg` (rounded square #2C388E with white "JJ") and a 1024x1024 `logo.png` generated from it, plus `assets/brand/LICENSE.md` saying brand assets are all rights reserved by James & James LLP and not covered by the project license. Generate app icons with `npm run tauri icon assets/brand/logo.png`.

6. Add a Tauri command `ping_core` that returns `pdf_core::version()` and show it in the About view, to prove the frontend -> Rust -> pdf-core path works.

7. Repo files: LICENSE (Apache-2.0, copyright 2026 James & James LLP), TRADEMARKS.md, README.md and CONTRIBUTING.md following the outlines in SPEC section 8 (mark unfinished parts as TODO), CODE_OF_CONDUCT.md (Contributor Covenant 2.1), SECURITY.md, CHANGELOG.md, .gitignore, .editorconfig, ESLint + Prettier config, GitHub issue templates and PR template.

8. Add `.github/workflows/ci.yml` and `.github/workflows/release.yml` exactly as outlined in SPEC section 7, adjusted to the real paths and script names you created. Add npm scripts: dev, build, lint, check, format, tauri, and `version` (a small node script that sets the same version in package.json, src-tauri/Cargo.toml and tauri.conf.json).

When done: run `cargo fmt`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace`, `npm run lint`, `npm run check`, and `npm run tauri build`, fix anything that fails, then tell me the size of the release exe, list what you created, and note anything you could not do. Keep code clean and commented where intent is not obvious. Ask me before adding any dependency not named here or in SPEC.md.
```

Follow-up prompts, one per milestone, can reuse the same pattern: "Read SPEC.md. Implement Milestone M<n> exactly as described in sections 5.x. Put PDF logic in `crates/pdf-core` with integration tests using fixtures in `tests/fixtures/`. Run all checks and the build before finishing."
