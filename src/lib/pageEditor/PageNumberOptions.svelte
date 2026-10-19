<script lang="ts">
  import Hash from 'lucide-svelte/icons/hash';
  import StampPreview from '../components/StampPreview.svelte';
  import type { PageNumberOptions, Stamping } from '../api';
  import { documents } from '../stores/documents.svelte';
  import { pageEdit } from '../stores/pageEdit.svelte';
  import { numberOptions, numberSettings as s } from '../stores/panels.svelte';
  import { NUMBER_FORMATS } from './logic';

  type Position = PageNumberOptions['position'];

  const SPOTS: Array<[Position, string]> = [
    ['top-left', 'Top left'],
    ['top-center', 'Top centre'],
    ['top-right', 'Top right'],
    ['bottom-left', 'Bottom left'],
    ['bottom-center', 'Bottom centre'],
    ['bottom-right', 'Bottom right']
  ];

  const doc = $derived(documents.current);
  const count = $derived(doc?.info.pages.length ?? 0);
  let problem = $state('');

  const marginOk = $derived(Number.isFinite(s.margin) && s.margin >= 0 && s.margin <= 100);
  const sizeOk = $derived(Number.isFinite(s.fontSize) && s.fontSize >= 1 && s.fontSize <= 500);
  const startOk = $derived(Number.isInteger(s.start) && Math.abs(s.start) <= 1_000_000);

  const options = $derived(numberOptions());
  const stamping = $derived<Stamping | null>(options ? { tool: 'page_numbers', ...options } : null);
  const ready = $derived(!!options && !problem);
  const applied = $derived(pageEdit.numbering);

  // The live preview draws one original page. Pages that were moved or deleted number
  // differently, so say so instead of letting the preview mislead.
  const reordered = $derived(
    pageEdit.items.length !== count ||
      pageEdit.items.some((i, k) => i.kind !== 'page' || i.src !== k + 1)
  );

  function apply() {
    if (options && ready) pageEdit.setNumbering(options);
  }
</script>

<form class="opts" onsubmit={(e) => e.preventDefault()}>
  <div class="field">
    <span>Position</span>
    <div class="spots" role="radiogroup" aria-label="Position">
      {#each SPOTS as [value, label] (value)}
        <label class="spot" class:on={s.position === value} title={label}>
          <input type="radio" name="spot" {value} bind:group={s.position} />
          <span class="dot"></span>
          <span class="sr">{label}</span>
        </label>
      {/each}
    </div>
  </div>

  <label class="field">
    <span>Format</span>
    <select bind:value={s.preset}>
      {#each NUMBER_FORMATS as [value, label] (value)}
        <option {value}>{label}</option>
      {/each}
      <option value="custom">Custom text…</option>
    </select>
  </label>
  {#if s.preset === 'custom'}
    <label class="field">
      <span>Text ({'{n}'} is the number, {'{total}'} the last number)</span>
      <input type="text" bind:value={s.custom} class:invalid={!s.custom.trim()} />
    </label>
  {/if}

  <div class="row">
    <label class="field">
      <span>Font size (pt)</span>
      <input
        type="number"
        min="1"
        max="500"
        step="1"
        bind:value={s.fontSize}
        class:invalid={!sizeOk}
      />
    </label>
    <label class="field">
      <span>Margin (mm)</span>
      <input
        type="number"
        min="0"
        max="100"
        step="1"
        bind:value={s.margin}
        class:invalid={!marginOk}
      />
    </label>
    <label class="field">
      <span>Colour</span>
      <input type="color" bind:value={s.color} />
    </label>
  </div>

  <div class="row">
    <label class="field">
      <span>Start at</span>
      <input type="number" step="1" bind:value={s.start} class:invalid={!startOk} />
    </label>
    <label class="field">
      <span>Number</span>
      <select bind:value={s.range}>
        <option value="all">All pages</option>
        <option value="pages">Only these pages…</option>
      </select>
    </label>
  </div>
  {#if s.range === 'pages'}
    <label class="field">
      <span>Pages</span>
      <input
        type="text"
        placeholder="2-end"
        bind:value={s.pagesText}
        class:invalid={!s.pagesText.trim()}
      />
    </label>
    <p class="note">
      To skip the cover, type 2-end. The first numbered page gets the start number.
    </p>
  {/if}
  <p class="note">
    Numbers use the standard Helvetica font, so they work in every viewer. Rotated and mixed-size
    pages are numbered where they appear on screen.
  </p>

  <div class="row">
    <button type="button" class="btn primary" disabled={!ready} onclick={apply}>
      <Hash size={16} strokeWidth={1.75} />
      {applied ? 'Update page numbers' : 'Add page numbers'}
    </button>
    {#if applied}
      <button type="button" class="btn" onclick={() => pageEdit.setNumbering(null)}>
        Remove page numbers
      </button>
    {/if}
  </div>
  <p class="note">
    {applied
      ? 'Page numbers are added to the saved file, in the order of the pages you see. Undo removes them.'
      : 'Numbers appear on the grid as a preview and are written when you save.'}
  </p>

  {#if doc}
    <StampPreview
      source={doc.path}
      pageCount={count}
      rotations={doc.info.pages.map((p) => p.rotate)}
      {stamping}
      bind:error={problem}
    />
    {#if reordered}
      <p class="note">
        The preview shows the original pages. On the grid, numbers follow your edited order.
      </p>
    {/if}
  {/if}
</form>

<style>
  .spots {
    display: grid;
    grid-template-columns: repeat(3, 40px);
    grid-template-rows: repeat(2, 40px);
    gap: var(--space-1);
    width: max-content;
    padding: var(--space-1);
    border: 1px solid var(--border);
    border-radius: var(--radius-control);
    background: var(--surface-2);
  }
  .spot {
    position: relative;
    display: grid;
    place-items: center;
    border-radius: 4px;
    cursor: pointer;
  }
  .spot:hover {
    background: var(--surface);
  }
  .spot input {
    position: absolute;
    opacity: 0;
    inset: 0;
    margin: 0;
    cursor: pointer;
  }
  .dot {
    width: 12px;
    height: 12px;
    border: 2px solid var(--text-muted);
    border-radius: 50%;
  }
  .spot.on .dot {
    border-color: var(--primary);
    background: var(--primary);
  }
  .spot:has(input:focus-visible) {
    outline: 2px solid var(--brand-500);
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
  }
</style>
