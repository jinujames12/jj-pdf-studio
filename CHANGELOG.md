# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and this project follows
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- M0 scaffold: Tauri 2 + Svelte 5 + TypeScript app, `pdf-core` library crate, design tokens with
  light and dark themes, custom title bar, collapsible sidebar, home screen and placeholder tool
  views, About screen, CI and release workflows.
- M3 tools: Merge (file view and page view, optional bookmarks that keep each file's own
  bookmarks), Split (every N pages, by ranges, at markers), Extract (one PDF or one per page) and
  Rotate (all, odd, even or chosen pages, make portrait or landscape). All write new files and
  never overwrite existing ones.
- M4 tools: Images to PDF (JPG, PNG, BMP, GIF, TIFF, WebP; JPEGs embedded without re-encoding,
  PNG transparency kept, EXIF orientation honoured; page size, orientation, margin, fit and quality
  options; one PDF or one per image) and Resize (A4, A3, A5, Letter, Legal or custom; fit, fill or
  size-only; links and comments move with the page).
