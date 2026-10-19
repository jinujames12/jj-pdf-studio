<script lang="ts">
  import { onMount } from 'svelte';
  import ChevronLeft from 'lucide-svelte/icons/chevron-left';
  import ChevronRight from 'lucide-svelte/icons/chevron-right';
  import { errorMessage, stampPreview, type Stamping } from '../api';
  import { loadPdf, renderPreview, type PreviewHandle, type PdfDoc } from '../pdf/render';

  interface Props {
    /** The open document. */
    source: string;
    pageCount: number;
    /** On-screen rotation of each source page (its own /Rotate), to draw the preview upright. */
    rotations: number[];
    /** What would be saved; `null` while the options are incomplete. */
    stamping: Stamping | null;
    /** Rust's complaint about the options (shown here), empty when they are fine. */
    error?: string;
  }

  let { source, pageCount, rotations, stamping, error = $bindable('') }: Props = $props();

  let page = $state(1);
  let box = $state<HTMLElement>();
  let canvas = $state<HTMLCanvasElement>();
  let busy = $state(false);

  // Keep the page number valid when another document is opened.
  $effect(() => {
    if (page > pageCount) page = Math.max(1, pageCount);
  });

  let latest = 0;
  let current: { pdf: PdfDoc; render?: PreviewHandle } | undefined;

  function discard() {
    current?.render?.cancel();
    void current?.pdf.loadingTask.destroy();
    current = undefined;
  }

  // Ask Rust for the stamped page and draw it. Only the newest request wins, so
  // typing quickly never shows an out-of-date preview.
  $effect(() => {
    const options = stamping;
    const n = page;
    const file = source;
    if (!options) {
      error = '';
      return;
    }
    const ticket = ++latest;
    const timer = setTimeout(async () => {
      busy = true;
      try {
        const bytes = await stampPreview(file, options, n);
        if (ticket !== latest) return;
        const pdf = await loadPdf(bytes);
        if (ticket !== latest || !canvas || !box) {
          void pdf.loadingTask.destroy();
          return;
        }
        discard();
        current = { pdf };
        const rect = box.getBoundingClientRect();
        current.render = renderPreview(
          pdf,
          canvas,
          1,
          rotations[n - 1] ?? 0,
          Math.max(rect.width - 2, 120),
          Math.max(Math.min(rect.width * 1.5, 640), 240)
        );
        await current.render.promise;
        if (ticket === latest) error = '';
      } catch (e) {
        if (ticket !== latest || (e as Error)?.name === 'RenderingCancelledException') return;
        error = errorMessage(e);
      } finally {
        if (ticket === latest) busy = false;
      }
    }, 200);
    return () => clearTimeout(timer);
  });

  onMount(() => discard);

  function go(delta: number) {
    page = Math.min(pageCount, Math.max(1, page + delta));
  }
</script>

<div class="stage">
  <div class="bar">
    <button class="btn" aria-label="Previous page" disabled={page <= 1} onclick={() => go(-1)}>
      <ChevronLeft size={16} strokeWidth={1.75} />
    </button>
    <span>
      Preview of page
      <input
        type="number"
        min="1"
        max={pageCount}
        aria-label="Preview page"
        bind:value={page}
        onchange={() => go(0)}
      />
      of {pageCount}
    </span>
    <button class="btn" aria-label="Next page" disabled={page >= pageCount} onclick={() => go(1)}>
      <ChevronRight size={16} strokeWidth={1.75} />
    </button>
  </div>
  <div class="paper" bind:this={box} class:busy>
    <canvas bind:this={canvas}></canvas>
    {#if error}
      <p class="problem" role="alert">{error}</p>
    {/if}
  </div>
</div>

<style>
  .stage {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    min-width: 0;
  }
  .bar {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-3);
    color: var(--text-muted);
  }
  .bar input {
    width: 64px;
    height: 28px;
    padding: 0 var(--space-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-control);
    background: var(--surface-2);
    color: var(--text);
    font: inherit;
  }
  .paper {
    position: relative;
    display: grid;
    place-items: center;
    min-height: 240px;
    padding: var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-card);
    background: var(--surface-2);
    transition: opacity var(--duration-base) var(--ease);
  }
  .paper.busy canvas {
    opacity: 0.6;
  }
  canvas {
    max-width: 100%;
    background: white;
    box-shadow: var(--shadow-card);
  }
  .problem {
    position: absolute;
    inset: auto var(--space-3) var(--space-3);
    margin: 0;
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-control);
    background: var(--surface);
    color: var(--danger);
    box-shadow: var(--shadow-card);
  }
</style>
