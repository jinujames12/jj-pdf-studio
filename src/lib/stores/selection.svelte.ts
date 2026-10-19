// Page selection for the single-document tools (keys are 1-based page numbers).
// The Set is only ever replaced, never mutated, so a plain Set is enough.
/* eslint-disable svelte/prefer-svelte-reactivity */

export function createSelection() {
  let selected = $state.raw<ReadonlySet<number>>(new Set());
  /** Last clicked page: the start of Shift-range selection. */
  let anchor = $state<number | null>(null);

  return {
    get selected() {
      return selected;
    },
    get anchor() {
      return anchor;
    },
    /** Click replaces the selection; Ctrl toggles one page; Shift selects a range. */
    click(page: number, mode: 'replace' | 'toggle' | 'range' = 'replace') {
      if (mode === 'range' && anchor !== null) {
        const [from, to] = anchor < page ? [anchor, page] : [page, anchor];
        selected = new Set(Array.from({ length: to - from + 1 }, (_, i) => from + i));
        return;
      }
      if (mode === 'toggle') {
        const next = new Set(selected);
        if (!next.delete(page)) next.add(page);
        selected = next;
      } else {
        selected = new Set([page]);
      }
      anchor = page;
    },
    set(pages: Iterable<number>) {
      selected = new Set(pages);
      anchor = null;
    },
    selectAll(count: number) {
      selected = new Set(Array.from({ length: count }, (_, i) => i + 1));
    },
    clear() {
      selected = new Set();
      anchor = null;
    }
  };
}
