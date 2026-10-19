<script lang="ts">
  import { onMount } from 'svelte';
  import RotateCcw from 'lucide-svelte/icons/rotate-ccw';
  import RotateCw from 'lucide-svelte/icons/rotate-cw';
  import Trash2 from 'lucide-svelte/icons/trash-2';
  import { renderThumb, THUMB_WIDTH } from '../pdf/render';
  import type { PdfDoc } from '../pdf/render';
  import { displayRotation, visualSize, type Item } from '../stores/pageEdit.svelte';

  interface Props {
    item: Item;
    /** 1-based position in the output. */
    position: number;
    pdf: PdfDoc;
    docKey: string;
    selected: boolean;
    onselect: (e: MouseEvent) => void;
    onpreview: () => void;
    onrotate: (degrees: number) => void;
    ondelete: () => void;
  }

  let { item, position, pdf, docKey, selected, onselect, onpreview, onrotate, ondelete }: Props =
    $props();

  let cell: HTMLElement;
  let canvas = $state<HTMLCanvasElement>();
  let visible = $state(false);

  const size = $derived(visualSize(item));
  const rotation = $derived(displayRotation(item));
  const aspect = $derived(size.width / size.height);
  const height = $derived(Math.min(Math.round(THUMB_WIDTH / aspect), 260));

  // Render lazily: only while the cell is near the viewport. Leaving it cancels
  // a render that has not started yet; the drawn canvas is kept as it is.
  onMount(() => {
    const io = new IntersectionObserver(
      (entries) => {
        visible = entries[entries.length - 1].isIntersecting;
      },
      { rootMargin: '400px' }
    );
    io.observe(cell);
    return () => io.disconnect();
  });

  // Buttons on the thumbnail have their own meaning; a double click on them
  // must not also open the preview.
  function dblclick(e: MouseEvent) {
    if (!(e.target as HTMLElement).closest('.thumb-btn')) onpreview();
  }

  $effect(() => {
    if (!visible || !canvas || item.kind !== 'page') return;
    const target = canvas;
    const src = item.src;
    const rot = rotation;
    let cancelled = false;
    const abort = new AbortController();
    renderThumb(pdf, docKey, src, rot, abort.signal)
      .then((bmp) => {
        if (cancelled) return;
        target.width = bmp.width;
        target.height = bmp.height;
        target.getContext('2d')?.drawImage(bmp, 0, 0);
      })
      .catch(() => {
        // A page pdf.js cannot draw keeps its blank placeholder.
      });
    return () => {
      cancelled = true;
      abort.abort();
    };
  });
</script>

<!-- Keyboard use is handled for the whole grid by the Pages tool. -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div
  class="cell"
  class:selected
  data-id={item.id}
  bind:this={cell}
  role="option"
  aria-selected={selected}
  tabindex="-1"
  onclick={onselect}
  ondblclick={dblclick}
>
  <div class="paper" style:aspect-ratio={aspect} style:max-height="{height}px">
    {#if item.kind === 'page'}
      <canvas bind:this={canvas} aria-label="Page {position}"></canvas>
    {:else}
      <span class="blank-label">Blank</span>
    {/if}
    <div class="actions">
      <button class="thumb-btn" title="Rotate left (Shift+R)" onclick={() => onrotate(-90)}>
        <RotateCcw size={14} strokeWidth={1.75} />
      </button>
      <button class="thumb-btn" title="Rotate right (R)" onclick={() => onrotate(90)}>
        <RotateCw size={14} strokeWidth={1.75} />
      </button>
      <button class="thumb-btn danger" title="Delete (Del)" onclick={ondelete}>
        <Trash2 size={14} strokeWidth={1.75} />
      </button>
    </div>
  </div>
  <span class="number">{position}</span>
</div>

<style>
  .cell {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-1);
    padding: var(--space-2);
    border: 2px solid transparent;
    border-radius: var(--radius-card);
    cursor: pointer;
    transition:
      background var(--duration-fast) var(--ease),
      border-color var(--duration-fast) var(--ease);
  }
  .cell:hover {
    background: var(--hover-bg);
  }
  .cell.selected {
    border-color: var(--primary);
    background: var(--selected-bg);
  }
  .paper {
    position: relative;
    display: grid;
    place-items: center;
    width: 160px;
    border-radius: var(--radius-thumb);
    background: #fff;
    box-shadow: var(--shadow-card);
    overflow: hidden;
  }
  canvas {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: contain;
  }
  .blank-label {
    color: #8a90a6;
    font-size: var(--text-caption);
  }
  .number {
    color: var(--text-muted);
    font-size: var(--text-caption);
    font-variant-numeric: tabular-nums;
  }
  .selected .number {
    color: var(--primary);
    font-weight: 600;
  }
  .actions {
    position: absolute;
    inset: auto 4px 4px auto;
    display: flex;
    gap: 2px;
    opacity: 0;
    transition: opacity var(--duration-fast) var(--ease);
  }
  .cell:hover .actions,
  .cell:focus-within .actions {
    opacity: 1;
  }
  .thumb-btn {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border: 1px solid var(--border);
    border-radius: var(--radius-control);
    background: var(--surface);
    color: var(--text);
    cursor: pointer;
  }
  .thumb-btn:hover {
    background: var(--surface-2);
  }
  .thumb-btn.danger:hover {
    color: var(--danger);
  }
</style>
