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

<!-- TODO: screenshot or GIF of the main window -->

> **Status:** early development (milestone M0, the app shell). The PDF tools below are planned
> and not implemented yet.

## Features

- **Merge** several PDFs into one
- **Pages**: reorder, rotate, delete and insert blank pages
- **Split** a PDF by page count, ranges or markers
- **Extract** selected pages
- **Rotate** all or some pages, losslessly
- **Resize** pages to A4, Letter and other sizes
- **Compress** to shrink file size
- **Images to PDF**

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

TODO: a quick start with three or four steps and screenshots.

## Limitations

TODO: fill in as the tools land. Known from the plan: compression only recompresses common image
types and leaves fonts alone; password-protected PDFs are not supported yet; form fields may not
stay interactive after merging or resizing.

- **Resize** moves links and comments with the page, but links that point to a position inside the
  document keep their old coordinates. In "Change page size only" mode, content larger than the new
  page is cropped by the page edge.
- **Images to PDF** reads JPG, PNG, BMP, GIF (first frame), TIFF (first page) and WebP. CMYK and
  other unusual encodings are not tested and may be refused with a "could not be read" message.

## Building from source

See [CONTRIBUTING.md](CONTRIBUTING.md).

## Roadmap

TODO: link to the milestones / GitHub Projects board.

## License

Code: [Apache-2.0](LICENSE). The JJ name and logo are not covered by that license; see
[TRADEMARKS.md](TRADEMARKS.md) and [assets/brand/LICENSE.md](assets/brand/LICENSE.md).

Third-party licenses: TODO (a `THIRD_PARTY_NOTICES.md` will be generated in CI).

Made by JJ Global.
