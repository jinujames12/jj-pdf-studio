// The success toast shown after a tool writes files.
import { errorMessage, openSaved, revealSaved, type SavedFile } from './api';
import { fmtSize, plural } from './format';
import { toast } from './stores/toast.svelte';

export const report = (e: unknown) => toast('error', errorMessage(e));

/** One saved file: "Open file" and "Show in folder". */
export function announceSaved(saved: SavedFile) {
  toast('success', `Saved ${fmtSize(saved.size)} to ${saved.path}`, [
    { label: 'Open file', run: () => void openSaved(saved.path).catch(report) },
    { label: 'Show in folder', run: () => void revealSaved(saved.path).catch(report) }
  ]);
}

/** Several files written into one folder: "Show in folder" selects the first. */
export function announceFiles(files: SavedFile[]) {
  if (files.length === 1) return announceSaved(files[0]);
  const total = files.reduce((sum, f) => sum + f.size, 0);
  toast('success', `Saved ${plural(files.length, 'file')} (${fmtSize(total)} in all).`, [
    { label: 'Show in folder', run: () => void revealSaved(files[0].path).catch(report) }
  ]);
}
