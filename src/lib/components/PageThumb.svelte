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
    selected?: boolean;
    /** The page the arrow keys move from; ringed while the grid has keyboard focus. */
    current?: boolean;
    onselect?: (e: MouseEvent) => void;
    onpreview?: () => void;
    onrotate?: (degrees: number) => void;
    ondelete?: () => void;
    /** Hides the rotate and delete buttons (view-only grids). */
    readonly?: boolean;
    /** Alternating background tint, used to show which output file a page lands in. */
    tint?: 'a' | 'b' | null;
    /** Text under the thumbnail instead of the position number. */
    label?: string;
    /** On-screen size and rotation, for pages that are not part of the Pages tool's document. */
    size?: { width: number; height: number };
    rotation?: number;
    /** Thumbnail width in pixels (Page Editor zoom). */
    width?: number;
    /** A page number drawn on the thumbnail, as it will be placed on the saved page. */
    overlay?: { text: string; position: string; color: string } | null;
    /** Right-click on the thumbnail. */
    oncontext?: (e: MouseEvent) => void;
  }

  let {
    item,
    position,
    pdf,
    docKey,
    selected = false,
    current = false,
    onselect,
    onpreview,
    onrotate,
    ondelete,
    readonly = false,
    tint = null,
    label,
    size: sizeOverride,
    rotation: rotationOverride,
    width = THUMB_WIDTH,
    overlay = null,
    oncontext
  }: Props = $props();

  let cell: HTMLElement;
  let canvas = $state<HTMLCanvasElement>();
  let visible = $state(false);

  const size = $derived(sizeOverride ?? visualSize(item));
  const rotation = $derived(rotationOverride ?? displayRotation(item));
  const aspect = $derived(size.width / size.height);
  const height = $derived(Math.min(Math.round(width / aspect), Math.round(width * 1.625)));

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
    if (!(e.target as HTMLElement).closest('.thumb-btn')) onpreview?.();
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
  class:current
  class:tint-a={tint === 'a'}
  class:tint-b={tint === 'b'}
  data-id={item.id}
  bind:this={cell}
  role="option"
  aria-selected={selected}
  tabindex="-1"
  onclick={onselect}
  ondblclick={dblclick}
  oncontextmenu={oncontext}
  style:--thumb-w="{width}px"
>
  <div class="paper" style:aspect-ratio={aspect} style:max-height="{height}px">
    {#if item.kind === 'page'}
      <canvas bind:this={canvas} aria-label="Page {position}"></canvas>
    {:else}
      <span class="blank-label">Blank</span>
    {/if}
    {#if overlay}
      <span
        class="stamp {overlay.position}"
        style:color={overlay.color}
        style:font-size="{Math.max(7, Math.round(width / 16))}px"
        aria-hidden="true">{overlay.text}</span
      >
    {/if}
    {#if !readonly}
      <div class="actions">
        <button
          class="thumb-btn"
          title="Rotate left (Shift+R)"
          aria-label="Rotate page {position} left"
          onclick={() => onrotate?.(-90)}
        >
          <RotateCcw size={14} strokeWidth={1.75} />
        </button>
        <button
          class="thumb-btn"
          title="Rotate right (R)"
          aria-label="Rotate page {position} right"
          onclick={() => onrotate?.(90)}
        >
          <RotateCw size={14} strokeWidth={1.75} />
        </button>
        <button
          class="thumb-btn danger"
          title="Delete (Del)"
          aria-label="Delete page {position}"
          onclick={ondelete}
        >
          <Trash2 size={14} strokeWidth={1.75} />
        </button>
      </div>
    {/if}
  </div>
  <span class="number" title={label}>{label ?? position}</span>
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
  :global(.grid:focus-visible) .cell.current {
    outline: 2px solid var(--focus-ring);
    outline-offset: 1px;
  }
  .cell.tint-a {
    background: color-mix(in srgb, var(--primary) 14%, transparent);
  }
  .cell.tint-b {
    background: color-mix(in srgb, var(--warning) 18%, transparent);
  }
  .paper {
    position: relative;
    display: grid;
    place-items: center;
    width: var(--thumb-w, 160px);
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
    max-width: var(--thumb-w, 160px);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-muted);
    font-size: var(--text-caption);
    font-variant-numeric: tabular-nums;
  }
  .selected .number {
    color: var(--primary);
    font-weight: 600;
  }
  /* Page-number preview: a small label near the edge the number will be placed at. */
  .stamp {
    position: absolute;
    padding: 0 2px;
    border-radius: 2px;
    background: rgb(255 255 255 / 70%);
    font-family: Helvetica, Arial, sans-serif;
    line-height: 1.3;
    white-space: nowrap;
    pointer-events: none;
  }
  .stamp.top-left,
  .stamp.top-center,
  .stamp.top-right {
    top: 4%;
  }
  .stamp.bottom-left,
  .stamp.bottom-center,
  .stamp.bottom-right {
    bottom: 4%;
  }
  .stamp.top-left,
  .stamp.bottom-left {
    left: 5%;
  }
  .stamp.top-right,
  .stamp.bottom-right {
    right: 5%;
  }
  .stamp.top-center,
  .stamp.bottom-center {
    left: 50%;
    transform: translateX(-50%);
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
