// Typed wrappers around Tauri's invoke(), so components never use string
// command names directly.
import { invoke, isTauri } from '@tauri-apps/api/core';
import { runJob } from './stores/jobs.svelte';

/** Version reported by the pdf-core crate (proves frontend -> Rust -> pdf-core). */
export async function pingCore(): Promise<string> {
  return invoke<string>('ping_core');
}

/** True when running inside the Tauri window (false in a plain browser tab). */
export const inTauri = (): boolean => isTauri();

/** Geometry of one page in PDF points, before rotation. */
export interface PageInfo {
  width: number;
  height: number;
  rotate: number;
}

export interface DocInfo {
  pages: PageInfo[];
}

/** One output page, as understood by pdf-core's `assemble`. */
export type PageOp =
  | { kind: 'page'; page: number; rotate: number }
  | { kind: 'blank'; width: number; height: number; rotate: number };

export interface SavedFile {
  path: string;
  size: number;
}

/** Errors from Rust arrive as plain-language strings. */
export function errorMessage(e: unknown): string {
  return typeof e === 'string' ? e : e instanceof Error ? e.message : 'Something went wrong.';
}

export const openPdf = (path: string) => invoke<DocInfo>('open_pdf', { path });

export async function readPdf(path: string): Promise<Uint8Array> {
  return new Uint8Array(await invoke<ArrayBuffer>('read_pdf', { path }));
}

export const savePages = (source: string, output: string, ops: PageOp[]) =>
  runJob('Saving the PDF', (jobId) =>
    invoke<SavedFile>('save_pages', { source, output, ops, jobId })
  );

export const openSaved = (path: string) => invoke<void>('open_saved', { path });
/** Opens a project page ('repository' or 'issues') in the default browser. */
export const openLink = (name: 'repository' | 'issues') => invoke<void>('open_link', { name });
export const revealSaved = (path: string) => invoke<void>('reveal_saved', { path });

export interface Settings {
  /** `null` follows Windows. */
  theme: 'light' | 'dark' | null;
}

export const getSettings = () => invoke<Settings>('get_settings');
export const setThemeSetting = (theme: 'light' | 'dark' | null) =>
  invoke<void>('set_theme', { theme });

// Merge, Split, Extract and Rotate ---------------------------------------------

/** One file for Merge. `pages: null` takes every page as it is. */
export interface MergeInput {
  path: string;
  pages: PageOp[] | null;
}

export const mergePdfs = (inputs: MergeInput[], output: string, bookmarks: boolean) =>
  runJob('Merging PDFs', (jobId) =>
    invoke<SavedFile>('merge_pdfs', { inputs, output, bookmarks, jobId })
  );

export type SplitMode =
  | { kind: 'every'; n: number }
  | { kind: 'ranges'; text: string }
  | { kind: 'at'; after: number[] }
  | { kind: 'pages'; pages: number[] };

export const splitPdf = (source: string, outDir: string, mode: SplitMode) =>
  runJob('Splitting the PDF', (jobId) =>
    invoke<SavedFile[]>('split_pdf', { source, outDir, mode, jobId })
  );

/** Inclusive 1-based `[first, last]` ranges; rejects with a plain-language message. */
export const parsePageRanges = (text: string, pageCount: number) =>
  invoke<Array<[number, number]>>('parse_page_ranges', { text, pageCount });

export type RotateTarget =
  { kind: 'all' } | { kind: 'odd' } | { kind: 'even' } | { kind: 'pages'; text: string };

export type RotateAction =
  | { kind: 'turn'; target: RotateTarget; degrees: number }
  | { kind: 'orient'; orientation: 'portrait' | 'landscape' };

/** New per-page turns (0/90/180/270) after applying `action` on top of `current`. */
export const planRotation = (source: string, current: number[], action: RotateAction) =>
  invoke<number[]>('plan_rotation', { source, current, action });

// Images to PDF and Resize ---------------------------------------------------------

/** A page size; custom sizes are in millimetres. */
export type Paper =
  | { kind: 'a3' | 'a4' | 'a5' | 'letter' | 'legal' }
  | { kind: 'custom'; width_mm: number; height_mm: number };

export type PageOrientation = 'auto' | 'portrait' | 'landscape';

export interface OpenedImage {
  /** Pixels as displayed (EXIF orientation applied). */
  width: number;
  height: number;
  size: number;
  /** Milliseconds since the Unix epoch; 0 if unknown. */
  modified: number;
}

export interface ImageInput {
  path: string;
  /** Extra clockwise turn: 0, 90, 180 or 270. */
  rotate: number;
}

export interface ImageOptions {
  /** `null`: each page is as large as its image. */
  page: Paper | null;
  orientation: PageOrientation;
  margin_mm: number;
  fit: 'inside' | 'fill' | 'original';
  quality: 'keep' | 'reduce';
}

export const openImage = (path: string) => invoke<OpenedImage>('open_image', { path });

/** A small preview as JPEG or PNG bytes. */
export async function imageThumbnail(path: string, maxEdge: number): Promise<Uint8Array> {
  return new Uint8Array(await invoke<ArrayBuffer>('image_thumbnail', { path, maxEdge }));
}

export const imagesToPdf = (inputs: ImageInput[], output: string, options: ImageOptions) =>
  runJob('Creating the PDF', (jobId) =>
    invoke<SavedFile>('images_to_pdf', { inputs, output, options, jobId })
  );

export const imagesToFolder = (inputs: ImageInput[], outDir: string, options: ImageOptions) =>
  runJob('Creating the PDFs', (jobId) =>
    invoke<SavedFile[]>('images_to_folder', { inputs, outDir, options, jobId })
  );

export interface ResizeOptions {
  paper: Paper;
  orientation: PageOrientation;
  /** Fit inside, fill (crop), or change the page size only. */
  mode: 'fit' | 'fill' | 'size';
  /** Where unscaled content sits ('size' mode only). */
  anchor: 'center' | 'topleft';
  margin_mm: number;
  target: RotateTarget;
}

/** New `[width, height]` in points of each page the options change, `null` for the rest. */
export const planResize = (source: string, options: ResizeOptions) =>
  invoke<Array<[number, number] | null>>('plan_resize', { source, options });

export const resizePdf = (source: string, output: string, options: ResizeOptions) =>
  runJob('Resizing pages', (jobId) =>
    invoke<SavedFile>('resize_pdf', { source, output, options, jobId })
  );

// Compress ---------------------------------------------------------------------------

export type CompressPreset = 'lossless' | 'balanced' | 'strong';

export interface CompressOptions {
  preset: CompressPreset;
  /** Also turn colour images gray (Strong only). */
  grayscale: boolean;
}

export interface CompressResult {
  /** Where the smaller copy was saved; `null` when it would not have been smaller. */
  path: string | null;
  before: number;
  after: number;
  written: boolean;
  images_found: number;
  images_recompressed: number;
}

export const compressPdf = (source: string, output: string, options: CompressOptions) =>
  runJob('Compressing the PDF', (jobId) =>
    invoke<CompressResult>('compress_pdf', { source, output, options, jobId })
  );

// Unlock, Protect and the password prompt ---------------------------------------------

/** What `openPdf` rejects with when the file needs a password nobody has typed yet. */
export const NEEDS_PASSWORD = 'PASSWORD_REQUIRED';

/** Checks a typed password and keeps it for this session so every tool can open the file. */
export const unlockSession = (path: string, password: string) =>
  invoke<void>('unlock_session', { path, password });

export interface ProtectionInfo {
  protection: 'none' | 'restrictions_only' | 'open_password';
  /** "RC4 40-bit", "RC4 128-bit", "AES-128" or "AES-256"; `null` if unknown. */
  algorithm: string | null;
  /** Plain-language names of the actions the file blocks. */
  blocked: string[];
}

export interface CopyHints {
  /** 1-based pages that are only a full-page picture. */
  scanned_pages: number[];
  fonts_without_unicode: number;
}

export interface UnlockResult {
  saved: SavedFile;
  protection: ProtectionInfo;
  hints: CopyHints;
}

export interface ProtectOptions {
  open_password: string;
  /** Required, and different from the open password, when anything is blocked. */
  owner_password: string | null;
  allow_printing: boolean;
  allow_copying: boolean;
  allow_editing: boolean;
}

export const protectionInfo = (source: string) =>
  invoke<ProtectionInfo>('protection_info', { source });

export const unlockPdf = (source: string, output: string) =>
  runJob('Removing protection', (jobId) =>
    invoke<UnlockResult>('unlock_pdf', { source, output, jobId })
  );

export const protectPdf = (source: string, output: string, options: ProtectOptions) =>
  runJob('Protecting the PDF', (jobId) =>
    invoke<SavedFile>('protect_pdf', { source, output, options, jobId })
  );

// PDF to Images, Page numbers, Watermark and Properties --------------------------------

export interface PageNumberOptions {
  position:
    'top-left' | 'top-center' | 'top-right' | 'bottom-left' | 'bottom-center' | 'bottom-right';
  margin_mm: number;
  font_size: number;
  /** `#rrggbb`. */
  color: string;
  /** Text with `{n}` and `{total}`. */
  format: string;
  start: number;
  /** Page-range text; blank numbers every page. */
  pages: string | null;
}

export type WatermarkKind =
  | { kind: 'text'; text: string; font_size: number; color: string }
  | { kind: 'image'; path: string; width_percent: number };

export type WatermarkOptions = WatermarkKind & {
  opacity: number;
  /** Counter-clockwise degrees. */
  rotation: number;
  place: 'center' | 'tiled' | 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right';
  layer: 'over' | 'under';
  margin_mm: number;
  pages: string | null;
};

export type Stamping =
  ({ tool: 'page_numbers' } & PageNumberOptions) | ({ tool: 'watermark' } & WatermarkOptions);

export const stampPdf = (source: string, output: string, stamping: Stamping) =>
  runJob('Adding to pages', (jobId) =>
    invoke<SavedFile>('stamp_pdf', { source, output, stamping, jobId })
  );

/** A one-page PDF showing `page` (1-based) as it would be saved. */
export async function stampPreview(
  source: string,
  stamping: Stamping,
  page: number
): Promise<Uint8Array> {
  return new Uint8Array(await invoke<ArrayBuffer>('stamp_preview', { source, stamping, page }));
}

export interface Properties {
  title: string;
  author: string;
  subject: string;
  keywords: string;
  creator: string;
  producer: string;
  created: string;
  modified: string;
  pdf_version: string;
  pages: PageInfo[];
  size: number;
  protection: ProtectionInfo;
  has_xmp: boolean;
}

export interface PropertiesEdit {
  title: string;
  author: string;
  subject: string;
  keywords: string;
  creator: string;
  remove_all: boolean;
}

export const readProperties = (source: string) => invoke<Properties>('read_properties', { source });

export const saveProperties = (source: string, output: string, edit: PropertiesEdit) =>
  runJob('Saving properties', (jobId) =>
    invoke<SavedFile>('save_properties', { source, output, edit, jobId })
  );

/** Starts an export into a folder, or into one ZIP file at `target`. */
export const exportImagesBegin = (
  source: string,
  target: string,
  zip: boolean,
  pageCount: number
) => invoke<void>('export_images_begin', { source, target, zip, pageCount });

/** Sends one rendered page picture (PNG or JPEG bytes) to be written. */
export const exportImagesAdd = (page: number, ext: 'png' | 'jpg', bytes: Uint8Array) =>
  invoke<void>('export_images_add', bytes, { headers: { page: String(page), ext } });

export const exportImagesFinish = () => invoke<SavedFile[]>('export_images_finish');
export const exportImagesCancel = () => invoke<void>('export_images_cancel');
