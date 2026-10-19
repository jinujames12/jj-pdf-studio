// Typed wrappers around Tauri's invoke(), so components never use string
// command names directly.
import { invoke, isTauri } from '@tauri-apps/api/core';

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
  invoke<SavedFile>('save_pages', { source, output, ops });

export const openSaved = (path: string) => invoke<void>('open_saved', { path });
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
  invoke<SavedFile>('merge_pdfs', { inputs, output, bookmarks });

export type SplitMode =
  | { kind: 'every'; n: number }
  | { kind: 'ranges'; text: string }
  | { kind: 'at'; after: number[] }
  | { kind: 'pages'; pages: number[] };

export const splitPdf = (source: string, outDir: string, mode: SplitMode) =>
  invoke<SavedFile[]>('split_pdf', { source, outDir, mode });

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
  invoke<SavedFile>('images_to_pdf', { inputs, output, options });

export const imagesToFolder = (inputs: ImageInput[], outDir: string, options: ImageOptions) =>
  invoke<SavedFile[]>('images_to_folder', { inputs, outDir, options });

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
  invoke<SavedFile>('resize_pdf', { source, output, options });
