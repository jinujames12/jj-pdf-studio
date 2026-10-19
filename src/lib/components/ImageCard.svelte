<script lang="ts">
  import { onMount } from 'svelte';
  import RotateCcw from 'lucide-svelte/icons/rotate-ccw';
  import RotateCw from 'lucide-svelte/icons/rotate-cw';
  import X from 'lucide-svelte/icons/x';
  import { fmtSize } from '../format';
  import { thumbUrl } from '../imageThumbs';
  import type { ImageItem } from '../stores/images.svelte';

  interface Props {
    item: ImageItem;
    position: number;
    onrotate: (degrees: number) => void;
    onremove: () => void;
  }

  let { item, position, onrotate, onremove }: Props = $props();

  const BOX = { width: 148, height: 124 };

  let card: HTMLElement;
  let url = $state<string | null>(null);
  let failed = $state(false);

  // Load the preview once the card is near the viewport.
  onMount(() => {
    const io = new IntersectionObserver(
      (entries) => {
        if (!entries.some((e) => e.isIntersecting)) return;
        io.disconnect();
        thumbUrl(item.path)
          .then((u) => (url = u))
          .catch(() => (failed = true));
      },
      { rootMargin: '300px' }
    );
    io.observe(card);
    return () => io.disconnect();
  });

  // The preview already has the EXIF orientation. The user's turn is a CSS
  // rotation, so the element keeps its own shape and is scaled so the *turned*
  // picture fits the box.
  const sideways = $derived(item.rotate % 180 !== 0);
  const scale = $derived.by(() => {
    const [w, h] = sideways ? [item.height, item.width] : [item.width, item.height];
    return Math.min(BOX.width / w, BOX.height / h);
  });
  const imgWidth = $derived(Math.round(item.width * scale));
  const imgHeight = $derived(Math.round(item.height * scale));
</script>

<div class="image-card" role="listitem" data-id={item.id} bind:this={card}>
  <div class="frame" style:width="{BOX.width}px" style:height="{BOX.height}px">
    {#if url}
      <img
        src={url}
        alt={item.name}
        draggable="false"
        style:width="{imgWidth}px"
        style:height="{imgHeight}px"
        style:transform="rotate({item.rotate}deg)"
      />
    {:else}
      <span class="placeholder">{failed ? 'No preview' : ''}</span>
    {/if}
    <span class="position">{position}</span>
    <div class="actions">
      <button class="icon-btn" title="Rotate left" onclick={() => onrotate(-90)}>
        <RotateCcw size={14} strokeWidth={1.75} />
      </button>
      <button class="icon-btn" title="Rotate right" onclick={() => onrotate(90)}>
        <RotateCw size={14} strokeWidth={1.75} />
      </button>
    </div>
    <button
      class="icon-btn remove"
      title="Remove {item.name}"
      aria-label="Remove {item.name}"
      onclick={onremove}
    >
      <X size={14} strokeWidth={1.75} />
    </button>
  </div>
  <span class="name" title={item.name}>{item.name}</span>
  <span class="meta">{item.width} × {item.height} px · {fmtSize(item.size)}</span>
</div>

<style>
  .image-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-1);
    max-width: 164px;
    padding: var(--space-2);
    border-radius: var(--radius-card);
    cursor: grab;
    transition: background var(--duration-fast) var(--ease);
  }
  .image-card:hover {
    background: var(--hover-bg);
  }
  .frame {
    position: relative;
    display: grid;
    place-items: center;
    overflow: hidden;
    border-radius: var(--radius-thumb);
    background: repeating-conic-gradient(var(--surface-2) 0% 25%, var(--surface) 0% 50%) 0 0 / 12px
      12px;
    box-shadow: var(--shadow-card);
  }
  img {
    display: block;
    pointer-events: none;
    transition: transform var(--duration-base) var(--ease);
  }
  .placeholder {
    color: var(--text-muted);
    font-size: var(--text-caption);
  }
  .position {
    position: absolute;
    top: 4px;
    left: 4px;
    min-width: 20px;
    padding: 0 6px;
    border-radius: 10px;
    background: color-mix(in srgb, var(--surface) 88%, transparent);
    color: var(--text);
    font-size: var(--text-caption);
    font-variant-numeric: tabular-nums;
    text-align: center;
  }
  .actions {
    position: absolute;
    right: 4px;
    bottom: 4px;
    display: flex;
    gap: 2px;
  }
  .remove {
    position: absolute;
    top: 4px;
    right: 4px;
  }
  .actions,
  .remove {
    opacity: 0;
    transition: opacity var(--duration-fast) var(--ease);
  }
  .image-card:hover .actions,
  .image-card:hover .remove,
  .actions:focus-within,
  .remove:focus-visible {
    opacity: 1;
  }
  .icon-btn {
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
  .icon-btn:hover {
    background: var(--surface-2);
  }
  .remove:hover {
    color: var(--danger);
  }
  .name {
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--text-caption);
  }
  .meta {
    color: var(--text-muted);
    font-size: var(--text-caption);
  }
</style>
