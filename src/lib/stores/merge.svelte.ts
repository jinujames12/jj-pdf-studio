// State of the Merge tool: the list of files, and (in page view) the list of
// individual pages. Nothing is written until Save (SPEC section 3).
import { confirm } from '@tauri-apps/plugin-dialog';
import { errorMessage, readPdf, type DocInfo, type MergeInput } from '../api';
import { openPdfAsking, PasswordCancelled } from '../openPdf';
import { fileName, stemOf } from '../paths';
import { clearThumbs, loadPdf, type PdfDoc } from '../pdf/render';
import { toast } from './toast.svelte';

export interface MergeFile {
  id: number;
  /** Thumbnail cache key, unique per added file. */
  key: string;
  path: string;
  name: string;
  /** Bookmark title: the file name without its extension. */
  stem: string;
  size: number;
  info: DocInfo;
  pdf: PdfDoc;
}

/** One page in page view. */
export interface PageEntry {
  id: number;
  fileId: number;
  /** 1-based page number in its file. */
  src: number;
  /** Extra turn on top of the page's own rotation. */
  rotate: number;
}

// $state.raw: pdf.js proxies must not be wrapped in a reactive proxy.
let files = $state.raw<MergeFile[]>([]);
/** The page list while page view is open, otherwise null. */
let entries = $state.raw<PageEntry[] | null>(null);
let pagesDirty = $state(false);
let loading = $state(false);
let nextId = 1;

const fileById = (id: number) => files.find((f) => f.id === id);

function expand(list: readonly MergeFile[]): PageEntry[] {
  return list.flatMap((f) =>
    f.info.pages.map((_, i) => ({ id: nextId++, fileId: f.id, src: i + 1, rotate: 0 }))
  );
}

function move<T extends { id: number }>(list: readonly T[], id: number, to: number): T[] {
  const from = list.findIndex((x) => x.id === id);
  if (from < 0) return [...list];
  const next = [...list];
  const [item] = next.splice(from, 1);
  next.splice(to, 0, item);
  return next;
}

export const merge = {
  get files() {
    return files;
  },
  get entries() {
    return entries;
  },
  get pageView() {
    return entries !== null;
  },
  get loading() {
    return loading;
  },
  get pageCount() {
    return entries ? entries.length : files.reduce((n, f) => n + f.info.pages.length, 0);
  },
  get totalSize() {
    return files.reduce((n, f) => n + f.size, 0);
  },
  file: fileById,

  /** Adds PDFs to the end of the list. Files that fail to open are reported and skipped. */
  async addPaths(paths: string[]) {
    loading = true;
    const added: MergeFile[] = [];
    try {
      for (const path of paths) {
        try {
          const info = await openPdfAsking(path);
          const bytes = await readPdf(path);
          const pdf = await loadPdf(bytes);
          const id = nextId++;
          added.push({
            id,
            key: `merge${id}`,
            path,
            name: fileName(path),
            stem: stemOf(path),
            size: bytes.byteLength,
            info,
            pdf
          });
        } catch (e) {
          if (!(e instanceof PasswordCancelled))
            toast('error', `${fileName(path)}: ${errorMessage(e)}`);
        }
      }
    } finally {
      loading = false;
    }
    if (!added.length) return;
    files = [...files, ...added];
    if (entries) entries = [...entries, ...expand(added)];
  },

  removeFile(id: number) {
    const gone = fileById(id);
    if (!gone) return;
    files = files.filter((f) => f.id !== id);
    if (entries) {
      entries = entries.filter((e) => e.fileId !== id);
      if (!entries.length) entries = null;
    }
    clearThumbs(gone.key);
    void gone.pdf.loadingTask.destroy();
  },

  moveFile(id: number, to: number) {
    files = move(files, id, to);
  },

  clear() {
    for (const f of files) {
      clearThumbs(f.key);
      void f.pdf.loadingTask.destroy();
    }
    files = [];
    entries = null;
    pagesDirty = false;
  },

  // Page view ---------------------------------------------------------------

  get pagesDirty() {
    return pagesDirty;
  },

  showPages() {
    if (entries || !files.length) return;
    entries = expand(files);
    pagesDirty = false;
  },

  /** Back to file view. Returns false if the user chose to keep their page edits. */
  async showFiles(): Promise<boolean> {
    if (!entries) return true;
    if (pagesDirty) {
      const discard = await confirm(
        'Going back to file view discards the page changes you made (order, rotation, removed pages).',
        {
          title: 'Discard page changes?',
          kind: 'warning',
          okLabel: 'Discard changes',
          cancelLabel: 'Keep editing'
        }
      );
      if (!discard) return false;
    }
    entries = null;
    pagesDirty = false;
    return true;
  },

  movePage(id: number, to: number) {
    if (!entries) return;
    entries = move(entries, id, to);
    pagesDirty = true;
  },

  rotatePage(id: number, degrees: number) {
    if (!entries) return;
    entries = entries.map((e) =>
      e.id === id ? { ...e, rotate: (((e.rotate + degrees) % 360) + 360) % 360 } : e
    );
    pagesDirty = true;
  },

  /** Removes a page; refuses to remove the last one. Returns success. */
  removePage(id: number): boolean {
    if (!entries || entries.length <= 1) return false;
    entries = entries.filter((e) => e.id !== id);
    pagesDirty = true;
    return true;
  },

  /**
   * What to send to Rust. File view merges whole files. Page view sends runs of
   * consecutive pages from the same file, so untouched files stay one bookmark.
   */
  toInputs(): MergeInput[] {
    if (!entries) return files.map((f) => ({ path: f.path, pages: null }));
    const inputs: MergeInput[] = [];
    let lastFile = -1;
    for (const e of entries) {
      const file = fileById(e.fileId);
      if (!file) continue;
      const op = { kind: 'page' as const, page: e.src, rotate: e.rotate };
      if (e.fileId === lastFile) inputs[inputs.length - 1].pages!.push(op);
      else inputs.push({ path: file.path, pages: [op] });
      lastFile = e.fileId;
    }
    return inputs;
  }
};
