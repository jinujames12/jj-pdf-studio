// The PDF currently open in the app, plus the helpers that open one.
import { open as openDialog, confirm } from '@tauri-apps/plugin-dialog';
import { errorMessage, openPdf, readPdf, type DocInfo } from '../api';
import { clearThumbs, loadPdf, type PdfDoc } from '../pdf/render';
import { go } from './nav.svelte';
import { pageEdit } from './pageEdit.svelte';
import { toast } from './toast.svelte';

export interface OpenDoc {
  /** Unique per open, so thumbnail caches never mix up two loads of one path. */
  key: string;
  path: string;
  name: string;
  /** Size of the source file in bytes. */
  size: number;
  info: DocInfo;
  pdf: PdfDoc;
}

// $state.raw: the pdf.js proxy must not be wrapped in a reactive proxy.
let current = $state.raw<OpenDoc | null>(null);
let loading = $state(false);
let dragging = $state(false);
let counter = 0;

export const documents = {
  get current() {
    return current;
  },
  get loading() {
    return loading;
  },
  /** True while files are being dragged over the window. */
  get dragging() {
    return dragging;
  },
  set dragging(v: boolean) {
    dragging = v;
  }
};

export const fileName = (path: string) => path.split(/[\\/]/).pop() ?? path;
export const folderOf = (path: string) =>
  path.slice(0, Math.max(path.lastIndexOf('\\'), path.lastIndexOf('/')) + 1);

/** Opens `path` as the current document. Returns true on success. */
export async function openPath(path: string): Promise<boolean> {
  if (pageEdit.dirty) {
    const discard = await confirm('Opening another file will discard your unsaved changes.', {
      title: 'Unsaved changes',
      kind: 'warning',
      okLabel: 'Discard changes',
      cancelLabel: 'Keep editing'
    });
    if (!discard) return false;
  }
  loading = true;
  try {
    const info = await openPdf(path);
    const bytes = await readPdf(path);
    const pdf = await loadPdf(bytes);
    if (current) {
      clearThumbs(current.key);
      void current.pdf.loadingTask.destroy();
    }
    current = {
      key: `doc${++counter}`,
      path,
      name: fileName(path),
      size: bytes.byteLength,
      info,
      pdf
    };
    pageEdit.reset(info);
    return true;
  } catch (e) {
    toast('error', errorMessage(e));
    return false;
  } finally {
    loading = false;
  }
}

/** Opens a PDF chosen from files the user picked or dropped, then shows it in Pages. */
export async function openFiles(paths: string[]) {
  const pdfs = paths.filter((p) => /\.pdf$/i.test(p));
  if (pdfs.length === 0) {
    toast('info', 'Only PDF files can be opened here. Images will work in Images to PDF.');
    return;
  }
  if (await openPath(pdfs[0])) {
    go('pages');
    if (pdfs.length > 1) toast('info', 'Opened the first file. Merging several files comes later.');
  }
}

export async function browseForPdf() {
  const picked = await openDialog({
    multiple: false,
    filters: [{ name: 'PDF', extensions: ['pdf'] }]
  });
  if (picked) await openFiles([picked]);
}
