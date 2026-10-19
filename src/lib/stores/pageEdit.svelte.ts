// Edit state of the Pages tool: the operation list, undo/redo, and selection.
// Nothing here touches the PDF; Save sends the list to Rust (SPEC section 3).

// The selection Set is never mutated in place, only replaced, so a plain Set is
// enough (SvelteSet would add reactivity we do not use).
/* eslint-disable svelte/prefer-svelte-reactivity */
import type { DocInfo, PageInfo, PageOp } from '../api';

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

let info: DocInfo = { pages: [] };
let nextId = 1;
let items = $state.raw<Item[]>([]);
let past = $state.raw<Item[][]>([]);
let future = $state.raw<Item[][]>([]);
let selected = $state.raw<ReadonlySet<number>>(new Set());
/** Last clicked item, the start of Shift-range selection and arrow navigation. */
let anchor = $state<number | null>(null);

const nativeRotate = (src: number) => info.pages[src - 1]?.rotate ?? 0;

/** Total rotation to draw: the page's own /Rotate plus the user's turns. */
export function displayRotation(item: Item): number {
  const base = item.kind === 'page' ? nativeRotate(item.src) : 0;
  return (((base + item.rotate) % 360) + 360) % 360;
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

function commit(next: Item[]) {
  past = [...past, items].slice(-HISTORY_LIMIT);
  future = [];
  items = next;
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
    past = [];
    future = [];
    selected = new Set();
    anchor = null;
  },

  undo() {
    if (!past.length) return;
    future = [...future, items];
    items = past[past.length - 1];
    past = past.slice(0, -1);
    pruneSelection();
  },
  redo() {
    if (!future.length) return;
    past = [...past, items];
    items = future[future.length - 1];
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

  // Edits (each one is a single undo step) -------------------------------------

  /** Rotates the selection by `degrees` (90, -90 or 180). `ids` overrides it. */
  rotate(degrees: number, ids: ReadonlySet<number> = selected) {
    if (!ids.size) return;
    commit(
      items.map((i) =>
        ids.has(i.id) ? { ...i, rotate: (((i.rotate + degrees) % 360) + 360) % 360 } : i
      )
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

  /** Inserts a blank page after the last selected page (or at the end). */
  insertBlank(size: BlankSize) {
    let at = items.length;
    for (let k = items.length - 1; k >= 0; k--) {
      if (selected.has(items[k].id)) {
        at = k + 1;
        break;
      }
    }
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

  /** The list to send to Rust on Save. */
  toOps(): PageOp[] {
    return items.map((i) =>
      i.kind === 'page'
        ? { kind: 'page', page: i.src, rotate: i.rotate }
        : { kind: 'blank', width: i.width, height: i.height, rotate: i.rotate }
    );
  }
};
