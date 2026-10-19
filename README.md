<p align="center">
  <img src="assets/brand/logo.svg" alt="JJ PDF Studio logo" width="96" height="96" />
</p>

<h1 align="center">JJ PDF Studio</h1>

<p align="center">Free, offline PDF tools for Windows.</p>

<p align="center">
  <!-- TODO: replace OWNER/REPO once the GitHub repository exists -->
  <a href="https://github.com/OWNER/REPO/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/OWNER/REPO/actions/workflows/ci.yml/badge.svg" /></a>
  <a href="https://github.com/OWNER/REPO/releases/latest"><img alt="Release" src="https://img.shields.io/github/v/release/OWNER/REPO" /></a>
  <a href="LICENSE"><img alt="License: Apache-2.0" src="https://img.shields.io/badge/license-Apache--2.0-blue" /></a>
</p>

<!-- The images are placeholders; see docs/screenshots/README.md for how to replace them. -->
<p align="center">
  <img src="docs/screenshots/main-window.svg" alt="The JJ PDF Studio home screen with the tool grid" width="720" />
</p>

## Features

**Organise**

- **Merge** several PDFs into one, with optional bookmarks
- **Page Editor**: the main workspace for one PDF. A toolbar, a right-click menu and a thumbnail
  grid cover reordering (drag), rotating, deleting, duplicating, moving and inserting blank
  pages (A4, A3, A5, Letter, Legal or the size of the neighbouring page), with undo and redo and
  a Select menu (all, none, invert, odd, even, range, landscape, portrait). An options panel on
  the right holds the full settings of four more tools:
  - **Split**: every N pages, by ranges such as `1-3, 4-8, 9-end`, or at markers you place
    between thumbnails. The grid shows the parts as tinted groups.
  - **Extract**: pick pages on the grid or type `2, 5-7`; one PDF or one PDF per page.
  - **Rotate**: all, odd, even or chosen pages by 90° or 180°, or make every page portrait or
    landscape.
  - **Page numbers**: six positions, formats like "Page 1 of N", start number and page range,
    previewed on each thumbnail.

  Edits (reorder, rotate, delete, blank pages, page numbers) are written when you save. Split and
  Extract write new files straight away from the pages as they are shown, unsaved edits included.
  The original is never changed.

- **Merge** several PDFs into one, with optional bookmarks

**Convert**

- **Images to PDF** (JPG, PNG, BMP, GIF, TIFF, WebP)
- **PDF to Images** (PNG or JPG, to a folder or a ZIP file)

**Optimise**

- **Resize** pages to A4, Letter and other sizes
- **Compress** to shrink file size (lossless, balanced or strong)

**Security**

- **Unlock**: remove an open password or restrictions
- **Protect**: lock a PDF with a password (AES-256)

**Edit**

- **Watermark** (text or picture) and **Properties** (metadata)

Every long job, Split and Extract included, shows a progress bar with a **Cancel** button (or
press Esc). The home screen has a card for every task, including Split, Extract, Rotate and Page
numbers; those open the Page Editor with the matching panel.

**Page Editor shortcuts:** Ctrl+Z / Ctrl+Y undo and redo, Ctrl+A select all, Delete remove, R /
Shift+R rotate right / left, Ctrl+D duplicate, Ctrl+Shift+S Split panel, Ctrl+E Extract panel,
Ctrl+Shift+N Page numbers panel, Esc close the panel, arrow keys move the selection, Enter
previews a page, Shift+F10 opens the page menu. The toolbar, menus and panel work with the
keyboard alone (arrow keys move along the toolbar). Press **F1** in the app for the full list.

## Download

TODO: link to the latest release once v1.0.0 is published.

- **Portable exe** (`JJPDFStudio-<version>-portable.exe`): double-click to run, no installation.
- **Installer** (`JJPDFStudio-<version>-setup.exe`): optional, for people who prefer one.

**Requirements:** Windows 10 (1809 or later) or Windows 11, x64. The Microsoft WebView2 runtime
is required; it is already present on Windows 11 and on up-to-date Windows 10.

**SmartScreen:** until releases are code-signed, Windows may show "Windows protected your PC" on
first run. Choose **More info**, then **Run anyway**. You can verify the download against the
SHA-256 checksums published with each release.

## Privacy

- Fully offline: no network calls, no telemetry, no update pings.
- Your files never leave your computer.
- Originals are never overwritten; every operation saves a new file.

## Usage

1. Pick a tool in the sidebar, or drop a PDF anywhere on the window.
2. Make your changes. Nothing is written until you save.
3. Press **Save** (Ctrl+S) and choose a file name. The original is never overwritten.
4. Use **Open file** or **Show in folder** in the message that appears.

<p align="center">
  <img src="docs/screenshots/pages.svg" alt="The Page Editor with a few pages selected" width="480" />
  <img src="docs/screenshots/dark-theme.svg" alt="A tool in the dark theme" width="480" />
</p>

## Limitations

Form fields may not stay interactive after merging or resizing. PDFs that need a password to open
ask for it first; passwords are never guessed.

- **Resize** moves links and comments with the page, but links that point to a position inside the
  document keep their old coordinates. In "Change page size only" mode, content larger than the new
  page is cropped by the page edge.
- **Images to PDF** reads JPG, PNG, BMP, GIF (first frame), TIFF (first page) and WebP. CMYK and
  other unusual encodings are not tested and may be refused with a "could not be read" message.

- **Compress** only recompresses images it fully understands: JPEG and Flate images in RGB or
  gray (8 bits per channel), with or without a soft mask. JBIG2, JPEG 2000, CCITT, CMYK, indexed
  colour, images with a Decode array or colour-key mask, and inline images are left as they are.
  Fonts are not subset or removed, so text-only and vector PDFs shrink little. If the result is
  not smaller, no file is written and the original should be kept. Recompressing images is lossy;
  use **Lossless** if nothing may change.

## Compression results

Run `cargo run --release -p pdf-core --example bench_compress` to reproduce. The example builds
three kinds of PDF in memory, so the numbers show how each preset behaves, not what a particular
real-world file will do. Photos and scans in the wild vary a lot; the generated pictures are
smoother and larger than most, which exaggerates the savings of Balanced and Strong.

| Document                                   | Original | Lossless       | Balanced       | Strong          |
| ------------------------------------------ | -------- | -------------- | -------------- | --------------- |
| Scan, 8 pages, JPEG 300 DPI                | 17.11 MB | 17.11 MB (-0%) | 2.14 MB (-87%) | 0.91 MB (-95%)  |
| Office report, 10 pages, 2 photos per page | 73.36 MB | 73.36 MB (-0%) | 0.63 MB (-99%) | 0.20 MB (-100%) |
| Text only, 100 pages, uncompressed         | 0.50 MB  | 0.04 MB (-93%) | 0.04 MB (-93%) | 0.04 MB (-93%)  |
| Text only, 100 pages, already Flate        | 0.05 MB  | 0.04 MB (-24%) | 0.04 MB (-24%) | 0.04 MB (-24%)  |

Lossless saves nothing on files whose size is mostly already-compressed images, and a lot on files
written without stream compression. Balanced and Strong only change images drawn above 200 and
150 DPI.

## Building from source

See [CONTRIBUTING.md](CONTRIBUTING.md).

## Roadmap

TODO: link to the milestones / GitHub Projects board.

## License

Code: [Apache-2.0](LICENSE). The JJ name and logo are not covered by that license; see
[TRADEMARKS.md](TRADEMARKS.md) and [assets/brand/LICENSE.md](assets/brand/LICENSE.md).

Third-party licenses: see [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md), also shown in the app
under **About**. Regenerate it with `npm run notices` after changing dependencies.

Made by James & James LLP.
