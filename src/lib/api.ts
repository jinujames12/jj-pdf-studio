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
