<script lang="ts">
  import { onMount } from 'svelte';
  import ChevronLeft from 'lucide-svelte/icons/chevron-left';
  import ChevronRight from 'lucide-svelte/icons/chevron-right';
  import X from 'lucide-svelte/icons/x';
  import { renderPreview } from '../pdf/render';
  import type { PdfDoc } from '../pdf/render';
  import { displayRotation, visualSize, type Item } from '../stores/pageEdit.svelte';

  interface Props {
    pdf: PdfDoc;
    /** Pages in output order, so Next/Previous follow the grid. */
    items: readonly Item[];
    /** Id of the item being shown. */
    id: number;
    onnavigate: (id: number) => void;
    onclose: () => void;
  }

  let { pdf, items, id, onnavigate, onclose }: Props = $props();

  let dialog: HTMLElement;
  let stageWidth = $state(0);
  let stageHeight = $state(0);
  let canvas = $state<HTMLCanvasElement>();
  let failed = $state(false);

  const index = $derived(items.findIndex((i) => i.id === id));
  const item = $derived(items[index]);

  // The page vanished (for example the document was replaced): nothing to show.
  $effect(() => {
    if (index < 0) onclose();
  });

  // Take focus so keys go to the dialog, and give it back to the grid on close.
  onMount(() => {
    const before = document.activeElement as HTMLElement | null;
    dialog.focus();
    return () => before?.focus?.();
  });

  $effect(() => {
    if (!canvas || !item || item.kind !== 'page' || !stageWidth || !stageHeight) return;
    const handle = renderPreview(
      pdf,
      canvas,
      item.src,
      displayRotation(item),
      stageWidth,
      stageHeight
    );
    failed = false;
    handle.promise.catch(() => {
      // A cancelled render is expected when moving on; anything else is a real failure.
      if (canvas) failed = true;
    });
    return () => handle.cancel();
  });

  // Blank pages have nothing to render: draw a white sheet at the right shape.
  const blankStyle = $derived.by(() => {
    if (!item || item.kind !== 'blank' || !stageWidth || !stageHeight) return '';
    const { width, height } = visualSize(item);
    const fit = Math.min(stageWidth / width, stageHeight / height);
    return `width:${Math.floor(width * fit)}px;height:${Math.floor(height * fit)}px`;
  });

  function go(delta: number) {
    const next = items[index + delta];
    if (next) onnavigate(next.id);
  }

  function onkeydown(e: KeyboardEvent) {
    // Handled here and stopped, so the Pages shortcuts never see these keys.
    e.stopPropagation();
    if (e.key === 'Escape') onclose();
    else if (e.key === 'ArrowRight' || e.key === 'ArrowDown' || e.key === 'PageDown') go(1);
    else if (e.key === 'ArrowLeft' || e.key === 'ArrowUp' || e.key === 'PageUp') go(-1);
    else if (e.key === 'Home') onnavigate(items[0].id);
    else if (e.key === 'End') onnavigate(items[items.length - 1].id);
    else return;
    e.preventDefault();
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="backdrop" onclick={onclose}>
  <div
    class="dialog"
    role="dialog"
    aria-modal="true"
    aria-label="Page preview"
    tabindex="-1"
    bind:this={dialog}
    {onkeydown}
    onclick={(e) => e.stopPropagation()}
  >
    <div class="stage" bind:clientWidth={stageWidth} bind:clientHeight={stageHeight}>
      {#if item}
        {#key item.id}
          {#if item.kind === 'page'}
            <canvas bind:this={canvas} aria-label="Page {index + 1}"></canvas>
            {#if failed}<span class="note">This page could not be drawn.</span>{/if}
          {:else}
            <div class="blank" style={blankStyle}><span>Blank page</span></div>
          {/if}
        {/key}
      {/if}
    </div>
    <div class="bar">
      <button
        class="icon"
        title="Previous page"
        aria-label="Previous page"
        disabled={index <= 0}
        onclick={() => go(-1)}
      >
        <ChevronLeft size={18} strokeWidth={1.75} />
      </button>
      <span class="count">Page {index + 1} of {items.length}</span>
      <button
        class="icon"
        title="Next page"
        aria-label="Next page"
        disabled={index >= items.length - 1}
        onclick={() => go(1)}
      >
        <ChevronRight size={18} strokeWidth={1.75} />
      </button>
      <button class="icon close" title="Close (Esc)" aria-label="Close preview" onclick={onclose}>
        <X size={18} strokeWidth={1.75} />
      </button>
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: grid;
    place-items: center;
    background: rgba(10, 12, 24, 0.6);
  }
  .dialog {
    display: flex;
    flex-direction: column;
    width: min(92vw, 1100px);
    height: min(90vh, 1000px);
    border: 1px solid var(--border);
    border-radius: var(--radius-card);
    background: var(--surface);
    box-shadow: var(--shadow-dialog);
    outline: none;
    overflow: hidden;
  }
  .stage {
    position: relative;
    display: grid;
    flex: 1;
    min-height: 0;
    place-items: center;
    margin: var(--space-4);
  }
  canvas {
    display: block;
    background: #fff;
    box-shadow: var(--shadow-card);
  }
  .blank {
    display: grid;
    place-items: center;
    background: #fff;
    box-shadow: var(--shadow-card);
    color: #8a90a6;
  }
  .note {
    position: absolute;
    color: var(--danger);
  }
  .bar {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-4);
    border-top: 1px solid var(--border);
    background: var(--surface-2);
  }
  .count {
    min-width: 110px;
    text-align: center;
    font-variant-numeric: tabular-nums;
  }
  .icon {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    border: 1px solid var(--border);
    border-radius: var(--radius-control);
    background: var(--surface);
    color: var(--text);
    cursor: pointer;
  }
  .icon:hover:not(:disabled) {
    background: var(--selected-bg);
  }
  .icon:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .close {
    margin-left: var(--space-4);
  }
</style>
