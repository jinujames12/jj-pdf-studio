// Edit state of the Page Editor: the operation list, page numbering, undo/redo and
// selection. Nothing here touches the PDF; Save sends the list to Rust (SPEC section 3).

// The selection Set is never mutated in place, only replaced, so a plain Set is
// enough (SvelteSet would add reactivity we do not use).
/* eslint-disable svelte/prefer-svelte-reactivity */
import type { DocInfo, PageInfo, PageNumberOptions, PageOp } from '../api';

export type Item =
  | { id: number; kind: 'page'; src: number; rotate: number }
  | { id: number; kind: 'blank'; width: number; height: number; rotate: number };

/** An item that is a page of the source document. */
export type PageItem = Extract<Item, { kind: 'page' }>;

/** Blank page sizes in PDF points. */
export const BLANK_SIZES = {
  A4: [595, 842],
  A3: [842, 1191],
  A5: [420, 595],
  Letter: [612, 792],
  Legal: [612, 1008]
} as const;
export type BlankSize = 'same' | keyof typeof BLANK_SIZES;

const HISTORY_LIMIT = 200;

/** One undo step: the page list and the page numbering that go with it. */
interface Snapshot {
  items: Item[];
  numbering: PageNumberOptions | null;
}

let info: DocInfo = { pages: [] };
let nextId = 1;
let items = $state.raw<Item[]>([]);
let numbering = $state.raw<PageNumberOptions | null>(null);
let past = $state.raw<Snapshot[]>([]);
let future = $state.raw<Snapshot[]>([]);
let selected = $state.raw<ReadonlySet<number>>(new Set());
/** Last clicked item, the start of Shift-range selection and arrow navigation. */
let anchor = $state<number | null>(null);

const nativeRotate = (src: number) => info.pages[src - 1]?.rotate ?? 0;
const wrap = (degrees: number) => ((degrees % 360) + 360) % 360;

/** Total rotation to draw: the page's own /Rotate plus the user's turns. */
export function displayRotation(item: Item): number {
  const base = item.kind === 'page' ? nativeRotate(item.src) : 0;
  return wrap(base + item.rotate);
}

/** On-screen size in points, after rotation. */
export function visualSize(item: Item): { width: number; height: number } {
  const p: PageInfo =
    item.kind === 'page'
      ? (info.pages[item.src - 1] ?? { width: 612, height: 792, rotate: 0 })
      : { width: item.width, height: item.height, rotate: 0 };
  const swap = displayRotation(item) % 180 !== 0;
  // For source pages the base rotation is already part of displayRotation, and
  // pdf.js reports the viewport rotated by it, so swap only on odd quarter turns.
  return swap ? { width: p.height, height: p.width } : { width: p.width, height: p.height };
}

function opsOf(list: readonly Item[]): PageOp[] {
  return list.map((i) =>
    i.kind === 'page'
      ? { kind: 'page', page: i.src, rotate: i.rotate }
      : { kind: 'blank', width: i.width, height: i.height, rotate: i.rotate }
  );
}

function commit(next: Item[], nextNumbering: PageNumberOptions | null = numbering) {
  past = [...past, { items, numbering }].slice(-HISTORY_LIMIT);
  future = [];
  items = next;
  numbering = nextNumbering;
  pruneSelection();
}

function pruneSelection() {
  const ids = new Set(items.map((i) => i.id));
  const kept = [...selected].filter((id) => ids.has(id));
  if (kept.length !== selected.size) selected = new Set(kept);
  if (anchor !== null && !ids.has(anchor)) anchor = kept[0] ?? null;
}

export const pageEdit = {
  get items() {
    return items;
  },
  get selected() {
    return selected;
  },
  get anchor() {
    return anchor;
  },
  /** Page numbers added on Save, or `null`. Part of the undo history. */
  get numbering() {
    return numbering;
  },
  get canUndo() {
    return past.length > 0;
  },
  get canRedo() {
    return future.length > 0;
  },
  /** True when the list differs from the untouched document. */
  get dirty() {
    return past.length > 0;
  },

  /** Starts a fresh edit session for a newly opened document. */
  reset(docInfo: DocInfo) {
    info = docInfo;
    nextId = 1;
    items = docInfo.pages.map((_, i) => ({ id: nextId++, kind: 'page', src: i + 1, rotate: 0 }));
    numbering = null;
    past = [];
    future = [];
    selected = new Set();
    anchor = null;
  },

  undo() {
    if (!past.length) return;
    future = [...future, { items, numbering }];
    ({ items, numbering } = past[past.length - 1]);
    past = past.slice(0, -1);
    pruneSelection();
  },
  redo() {
    if (!future.length) return;
    past = [...past, { items, numbering }];
    ({ items, numbering } = future[future.length - 1]);
    future = future.slice(0, -1);
    pruneSelection();
  },

  // Selection ---------------------------------------------------------------

  /** Click: replace the selection. Ctrl toggles one item; Shift selects a range. */
  select(id: number, mode: 'replace' | 'toggle' | 'range' = 'replace') {
    if (mode === 'range' && anchor !== null) {
      const a = items.findIndex((i) => i.id === anchor);
      const b = items.findIndex((i) => i.id === id);
      if (a >= 0 && b >= 0) {
        const [from, to] = a < b ? [a, b] : [b, a];
        selected = new Set(items.slice(from, to + 1).map((i) => i.id));
        return;
      }
    }
    if (mode === 'toggle') {
      const next = new Set(selected);
      if (!next.delete(id)) next.add(id);
      selected = next;
    } else {
      selected = new Set([id]);
    }
    anchor = id;
  },
  selectAll() {
    selected = new Set(items.map((i) => i.id));
  },
  clearSelection() {
    selected = new Set();
  },
  /** Replaces the selection with the items at these 1-based grid positions. */
  selectPositions(positions: Iterable<number>) {
    const ids: number[] = [];
    for (const p of positions) if (items[p - 1]) ids.push(items[p - 1].id);
    selected = new Set(ids);
    anchor = ids[0] ?? null;
  },
  invertSelection() {
    selected = new Set(items.filter((i) => !selected.has(i.id)).map((i) => i.id));
  },
  /** Selects every item for which `test(item, position)` holds (position is 1-based). */
  selectWhere(test: (item: Item, position: number) => boolean) {
    selected = new Set(items.filter((it, k) => test(it, k + 1)).map((i) => i.id));
    anchor = null;
  },
  /** Selects the pages that are wider than tall (landscape) or not (portrait), as shown. */
  selectOrientation(orientation: 'landscape' | 'portrait') {
    this.selectWhere((it) => {
      const { width, height } = visualSize(it);
      return orientation === 'landscape' ? width > height : height >= width;
    });
  },
  /** 1-based grid positions of the selection, ascending. */
  get selectedPositions(): number[] {
    return items.flatMap((it, k) => (selected.has(it.id) ? [k + 1] : []));
  },

  // Edits (each one is a single undo step) -------------------------------------

  /** Rotates the selection by `degrees` (90, -90 or 180). `ids` overrides it. */
  rotate(degrees: number, ids: ReadonlySet<number> = selected) {
    if (!ids.size) return;
    commit(items.map((i) => (ids.has(i.id) ? { ...i, rotate: wrap(i.rotate + degrees) } : i)));
  },

  /** Sets every item's extra turn (0/90/180/270), as planned by Rust. One undo step. */
  setTurns(turns: readonly number[]) {
    const next = items.map((i, k) => ({ ...i, rotate: wrap(turns[k] ?? 0) }));
    if (next.every((it, k) => it.rotate === items[k].rotate)) return;
    commit(next);
  },
  /** Current extra turns, one per item. */
  get turns(): number[] {
    return items.map((i) => i.rotate);
  },
  /** Geometry of the current list (before rotation), for planning rotations on it. */
  pageGeometry(): PageInfo[] {
    return items.map((i) =>
      i.kind === 'page'
        ? (info.pages[i.src - 1] ?? { width: 612, height: 792, rotate: 0 })
        : { width: i.width, height: i.height, rotate: 0 }
    );
  },

  /** Deletes the selection (or `ids`). Refuses to delete every page. Returns success. */
  remove(ids: ReadonlySet<number> = selected): boolean {
    if (!ids.size) return false;
    const next = items.filter((i) => !ids.has(i.id));
    if (next.length === 0) return false;
    commit(next);
    return true;
  },

  /**
   * Moves `id` (and the whole selection with it, if `id` is part of a multi
   * selection) so that `id` ends up at index `to` of the resulting list.
   */
  move(id: number, to: number) {
    const group = selected.has(id) && selected.size > 1 ? selected : new Set([id]);
    const moving = items.filter((i) => group.has(i.id));
    const rest = items.filter((i) => !group.has(i.id));
    // Items that must sit before the drop slot: of the `to` items preceding
    // the dragged one in the final DOM order, those not travelling with it.
    const without = items.filter((i) => i.id !== id);
    const before = without.slice(0, to).filter((i) => !group.has(i.id)).length;
    const next = [...rest.slice(0, before), ...moving, ...rest.slice(before)];
    if (next.every((it, k) => it.id === items[k].id)) return;
    commit(next);
  },

  /** Moves the selection (or `ids`) to the start or the end, keeping its order. */
  moveToEdge(edge: 'start' | 'end', ids: ReadonlySet<number> = selected) {
    if (!ids.size) return;
    const moving = items.filter((i) => ids.has(i.id));
    const rest = items.filter((i) => !ids.has(i.id));
    const next = edge === 'start' ? [...moving, ...rest] : [...rest, ...moving];
    if (next.every((it, k) => it.id === items[k].id)) return;
    commit(next);
  },

  /** Copies the selection (or `ids`); each copy sits right after its original and is selected. */
  duplicate(ids: ReadonlySet<number> = selected) {
    if (!ids.size) return;
    const copies: number[] = [];
    const next = items.flatMap((i) => {
      if (!ids.has(i.id)) return [i];
      const copy = { ...i, id: nextId++ };
      copies.push(copy.id);
      return [i, copy];
    });
    commit(next);
    selected = new Set(copies);
    anchor = copies[0] ?? null;
  },

  /** Inserts a blank page after the last selected page (or at the end). */
  insertBlank(size: BlankSize) {
    let at = items.length;
    for (let k = items.length - 1; k >= 0; k--) {
      if (selected.has(items[k].id)) {
        at = k + 1;
        break;
      }
    }
    this.insertBlankAt(at, size);
  },

  /** Inserts a blank page so that it ends up at list index `at` (0 = first). */
  insertBlankAt(at: number, size: BlankSize) {
    at = Math.min(items.length, Math.max(0, at));
    let width: number, height: number;
    if (size === 'same') {
      const neighbour = items[at - 1] ?? items[at];
      ({ width, height } = neighbour ? visualSize(neighbour) : { width: 595, height: 842 });
    } else {
      [width, height] = BLANK_SIZES[size];
    }
    const blank: Item = {
      id: nextId++,
      kind: 'blank',
      width: Math.round(width),
      height: Math.round(height),
      rotate: 0
    };
    commit([...items.slice(0, at), blank, ...items.slice(at)]);
    selected = new Set([blank.id]);
    anchor = blank.id;
  },

  /** Inserts a blank page directly before or after the item `id`. */
  insertBlankNextTo(id: number, side: 'before' | 'after', size: BlankSize) {
    const k = items.findIndex((i) => i.id === id);
    if (k >= 0) this.insertBlankAt(side === 'before' ? k : k + 1, size);
  },

  /** Adds page numbers on Save (or removes them with `null`). One undo step. */
  setNumbering(options: PageNumberOptions | null) {
    if (JSON.stringify(options) === JSON.stringify(numbering)) return;
    commit(items, options);
  },

  /** The list to send to Rust on Save. */
  toOps(): PageOp[] {
    return opsOf(items);
  },

  /** The edited pages with these ids (default: the selection), in grid order. */
  opsFor(ids: ReadonlySet<number> = selected): PageOp[] {
    return opsOf(items.filter((i) => ids.has(i.id)));
  }
};
