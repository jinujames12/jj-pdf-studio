// Typed wrappers around Tauri's invoke(), so components never use string
// command names directly.
import { invoke, isTauri } from '@tauri-apps/api/core';

/** Version reported by the pdf-core crate (proves frontend -> Rust -> pdf-core). */
export async function pingCore(): Promise<string> {
  return invoke<string>('ping_core');
}

/** True when running inside the Tauri window (false in a plain browser tab). */
export const inTauri = (): boolean => isTauri();
