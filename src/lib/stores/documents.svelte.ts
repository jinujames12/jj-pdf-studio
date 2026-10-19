// The PDF currently open in the app, plus the helpers that open one.
import { open as openDialog, confirm } from '@tauri-apps/plugin-dialog';
import { errorMessage, readPdf, type DocInfo } from '../api';
import { openPdfAsking, PasswordCancelled } from '../openPdf';
import { clearThumbs, loadPdf, type PdfDoc } from '../pdf/render';
import { fileName, isImage } from '../paths';
import { images } from './images.svelte';
import { merge } from './merge.svelte';
import { go, nav } from './nav.svelte';
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
/** Set by tools that hold unsaved choices of their own (Rotate). */
let toolDirty = $state(false);
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
  },
  set toolDirty(v: boolean) {
    toolDirty = v;
  }
};

/** Tools that keep working on the current document instead of jumping to Pages. */
const STAY_IN_TOOL: string[] = [
  'split',
  'extract',
  'rotate',
  'resize',
  'compress',
  'unlock',
  'protect',
  'toimages',
  'pagenumbers',
  'watermark',
  'properties'
];

export { folderOf } from '../paths';

/** Opens `path` as the current document. Returns true on success. */
export async function openPath(path: string): Promise<boolean> {
  if (pageEdit.dirty || toolDirty) {
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
    const info = await openPdfAsking(path);
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
    if (!(e instanceof PasswordCancelled)) toast('error', errorMessage(e));
    return false;
  } finally {
    loading = false;
  }
}

/**
 * Handles files the user picked or dropped. In Merge they are added to the list;
 * elsewhere the first PDF becomes the current document, shown in Pages unless a
 * tool that works on the current document is already open.
 */
export async function openFiles(paths: string[]) {
  const pdfs = paths.filter((p) => /\.pdf$/i.test(p));
  const pictures = paths.filter(isImage);
  if (nav.view === 'images') {
    if (pictures.length) await images.addPaths(pictures);
    else toast('info', 'Only images can be added here. Use Merge or Pages for PDFs.');
    return;
  }
  if (pdfs.length === 0) {
    if (pictures.length) {
      // Pictures dropped anywhere else belong in Images to PDF.
      go('images');
      await images.addPaths(pictures);
    } else {
      toast('info', 'Only PDF files and images can be opened here.');
    }
    return;
  }
  if (nav.view === 'merge') {
    await merge.addPaths(pdfs);
    return;
  }
  if (await openPath(pdfs[0])) {
    if (!STAY_IN_TOOL.includes(nav.view)) go('pages');
    if (pdfs.length > 1)
      toast('info', 'Opened the first file. Use Merge to combine several files.');
  }
}

export async function browseForPdf() {
  const picked = await openDialog({
    multiple: false,
    filters: [{ name: 'PDF', extensions: ['pdf'] }]
  });
  if (picked) await openFiles([picked]);
}
