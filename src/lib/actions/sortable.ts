// Drag-to-reorder for a list of elements carrying `data-id`. Sortable moves DOM
// nodes itself; we put the node back and report the move instead, so Svelte
// stays the only owner of the DOM. forceFallback uses pointer events, because
// Tauri's native file-drop handling disables HTML5 drag and drop on Windows.
import Sortable from 'sortablejs';

export interface SortableOptions {
  /** CSS selector of the draggable children. */
  draggable: string;
  /** Called with the dragged element's `data-id` and its new index. */
  onmove: (id: number, newIndex: number) => void;
  /** Called after every drag, moved or not; views bump a key to rebuild the list. */
  onend?: () => void;
}

export function sortable(node: HTMLElement, options: SortableOptions) {
  const reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  const instance = Sortable.create(node, {
    draggable: options.draggable,
    filter: 'button, input, select',
    preventOnFilter: false,
    forceFallback: true,
    fallbackTolerance: 4,
    animation: reduced ? 0 : 150,
    ghostClass: 'ghost',
    onEnd(evt) {
      const { item, from, oldIndex, newIndex } = evt;
      if (oldIndex === undefined || newIndex === undefined) return;
      from.removeChild(item);
      from.insertBefore(item, from.children[oldIndex] ?? null);
      if (oldIndex !== newIndex) options.onmove(Number(item.dataset.id), newIndex);
      options.onend?.();
    }
  });
  return { destroy: () => instance.destroy() };
}
