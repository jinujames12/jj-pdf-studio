# SPEC addition: Page Editor (merge into SPEC.md)

Merge these into SPEC.md, keeping everything already there, then delete this file.

## A. Replace the sidebar bullets in section 4 with:

- **Sidebar (220 px, collapsible to 64 px icons):** Page Editor at the top as the main workspace (see 5.15), then the whole-file tools. Rotate, Split, Extract and Page numbers have no sidebar entries of their own; they live inside Page Editor with all their options.
- **Sidebar grouping (10 entries):** *Organise* (Page Editor, Merge), *Convert* (Images to PDF, PDF to Images), *Optimise* (Resize, Compress), *Security* (Unlock, Protect), *Edit* (Watermark, Properties). The home screen still shows a card for every task, including Split, Extract, Rotate and Page numbers; those cards open Page Editor with that panel already open, so people who look for "Split" still find it.

In the section 4 layout diagram, the sidebar becomes: Page Editor, Merge | Convert: Images to PDF, PDF to Images | Optimise: Resize, Compress | Security: Unlock, Protect | Edit: Watermark, Properties.

## B. Add as a new section in 5, before About (renumber About and later sections):

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
  - *Edits to this document* (reorder, rotate, delete, insert blank, page numbers) go into undo/redo history and are written when the user clicks **Save**.
  - *Exports to new files* (Split, Extract) run immediately from their panel with their own button ("Split into 4 files", "Extract 3 pages") and use the current edited state of the document, including unsaved edits. They don't change the open document.
- **Keyboard:** everything in 4 (Ctrl+Z/Y, Delete, R/Shift+R, Ctrl+A), plus Ctrl+D duplicate, Ctrl+Shift+S split panel, Ctrl+E extract panel, Ctrl+Shift+N page numbers panel, Esc closes the panel.

**Feature-parity checklist** (every line must still work after the move; turn each into a test or a manual check)

*Pages (5.2):*
- [ ] Drag to reorder single and multiple selected pages
- [ ] Rotate per page and per selection, 90° steps, lossless via `/Rotate`
- [ ] Delete selection; refuses to delete the last page
- [ ] Insert blank via "+" between thumbnails and via toolbar; sizes Same as previous, A4, A3, A5, Letter, Legal
- [ ] Undo/redo for every action; nothing written before Save
- [ ] Status footer: page count, selection count, file size

*Rotate (5.5):*
- [ ] Rotate all, odd pages, even pages, custom range
- [ ] 90° right, 90° left, 180°
- [ ] "Make all pages portrait" and "Make all pages landscape"

*Split (5.3):*
- [ ] Every N pages (default 1)
- [ ] By ranges with `end` keyword, live validation and grid highlighting
- [ ] At selected pages via split markers between thumbnails
- [ ] Output folder choice; `name_part01.pdf` and `name_p1-3.pdf` naming
- [ ] Warning above 100 output files

*Extract (5.4):*
- [ ] Select on grid or type ranges (`2, 5-7`), kept in sync both ways
- [ ] One PDF with all selected pages (default)
- [ ] One PDF per selected page

*Page numbers (5.12):*
- [ ] 6 positions, margin, font size, colour
- [ ] Formats `1`, `Page 1`, `Page 1 of N`, `1 / N`, custom with `{n}` and `{total}`
- [ ] Start number and page range (skip cover)
- [ ] Correct placement on rotated and mixed-size pages

*Shared:*
- [ ] Never overwrites the original; Save As prefilled
- [ ] Password prompt for protected files (5.9) works when opening in Page Editor
- [ ] Home screen cards for Split, Extract, Rotate and Page numbers open Page Editor with the right panel
- [ ] Old routes (`/pages`, `/split`, `/extract`, `/rotate`, `/page-numbers`) redirect to Page Editor with the matching panel, so nothing breaks

**Implementation notes**
- One document state store (the operation list from 3) drives the grid, the panels and Save. Split and Extract call the existing `pdf-core` functions with the current operation list applied first, so no PDF logic is rewritten; this milestone is a frontend refactor plus a thin "apply ops then split/extract" command.
- Keep each tool's settings form as its own component (`SplitOptions.svelte`, `ExtractOptions.svelte`, `RotateOptions.svelte`, `PageNumberOptions.svelte`) and mount it in the options panel, so the forms are moved, not rewritten.

## C. Milestones: insert the Page Editor milestone after M7 and renumber Polish:

| **M8: Page Editor** | Combine Pages, Rotate, Split, Extract and Page numbers into one Page Editor screen (5.15): toolbar, right-click menu, options panel, live grid preview, route redirects, renamed sidebar with 10 entries | Every item in the 5.15 parity checklist passes; existing pdf-core tests still green |
| **M9: Polish and v1.0** | Sidebar grouping, progress and cancel everywhere, keyboard shortcuts, About and third-party notices, real logo, README screenshots, accessibility pass, code signing (if approved) | v1.0.0 release published |
