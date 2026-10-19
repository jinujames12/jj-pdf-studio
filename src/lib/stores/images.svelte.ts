// State of the Images to PDF tool: the ordered list of images and their turns.
// Nothing is converted until Create PDF.
import { errorMessage, openImage, type ImageInput } from '../api';
import { forgetThumb, clearThumbs } from '../imageThumbs';
import { fileName, isImage, stemOfAny } from '../paths';
import { toast } from './toast.svelte';

export interface ImageItem {
  id: number;
  path: string;
  name: string;
  stem: string;
  /** Pixels as displayed (EXIF orientation applied). */
  width: number;
  height: number;
  size: number;
  modified: number;
  /** Extra clockwise turn on top of the EXIF orientation: 0, 90, 180 or 270. */
  rotate: number;
}

let items = $state.raw<ImageItem[]>([]);
let loading = $state(false);
let nextId = 1;
let sortKey: 'name' | 'date' | null = null;
let sortDescending = false;

const collator = new Intl.Collator(undefined, { numeric: true, sensitivity: 'base' });

export const images = {
  get items() {
    return items;
  },
  get loading() {
    return loading;
  },
  get totalSize() {
    return items.reduce((n, i) => n + i.size, 0);
  },

  /** Adds images to the end of the list. Files that fail are reported and skipped. */
  async addPaths(paths: string[]) {
    const wanted = paths.filter(isImage);
    if (wanted.length < paths.length)
      toast('info', 'Some files were skipped: only JPG, PNG, BMP, GIF, TIFF and WebP are added.');
    if (!wanted.length) return;
    loading = true;
    const added: ImageItem[] = [];
    try {
      for (const path of wanted) {
        try {
          const opened = await openImage(path);
          added.push({
            id: nextId++,
            path,
            name: fileName(path),
            stem: stemOfAny(path),
            rotate: 0,
            ...opened
          });
        } catch (e) {
          toast('error', `${fileName(path)}: ${errorMessage(e)}`);
        }
      }
    } finally {
      loading = false;
    }
    if (added.length) items = [...items, ...added];
  },

  remove(id: number) {
    const gone = items.find((i) => i.id === id);
    items = items.filter((i) => i.id !== id);
    // Another entry may use the same file; keep its preview then.
    if (gone && !items.some((i) => i.path === gone.path)) forgetThumb(gone.path);
  },

  move(id: number, to: number) {
    const from = items.findIndex((i) => i.id === id);
    if (from < 0) return;
    const next = [...items];
    const [item] = next.splice(from, 1);
    next.splice(to, 0, item);
    items = next;
    sortKey = null;
  },

  rotate(id: number, degrees: number) {
    items = items.map((i) =>
      i.id === id ? { ...i, rotate: (((i.rotate + degrees) % 360) + 360) % 360 } : i
    );
  },

  /** Sorts by name or by date; sorting by the same key again reverses it. */
  sortBy(key: 'name' | 'date') {
    sortDescending = sortKey === key ? !sortDescending : false;
    sortKey = key;
    const dir = sortDescending ? -1 : 1;
    items = [...items].sort(
      (a, b) =>
        dir * (key === 'name' ? collator.compare(a.name, b.name) : a.modified - b.modified) ||
        collator.compare(a.name, b.name)
    );
  },

  clear() {
    items = [];
    sortKey = null;
    clearThumbs();
  },

  toInputs(): ImageInput[] {
    return items.map((i) => ({ path: i.path, rotate: i.rotate }));
  }
};
