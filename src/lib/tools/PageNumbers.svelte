<script lang="ts">
  import { onMount } from 'svelte';
  import { save } from '@tauri-apps/plugin-dialog';
  import FolderOpen from 'lucide-svelte/icons/folder-open';
  import Hash from 'lucide-svelte/icons/hash';
  import ToolFrame from '../components/ToolFrame.svelte';
  import StampPreview from '../components/StampPreview.svelte';
  import { stampPdf, type PageNumberOptions, type Stamping } from '../api';
  import { fmtSize, plural } from '../format';
  import { folderOf, stemOf } from '../paths';
  import { announceSaved, report } from '../saved';
  import { browseForPdf, documents } from '../stores/documents.svelte';

  type Position = PageNumberOptions['position'];

  const doc = $derived(documents.current);
  const count = $derived(doc?.info.pages.length ?? 0);

  const FORMATS: Array<[string, string]> = [
    ['{n}', '1'],
    ['Page {n}', 'Page 1'],
    ['Page {n} of {total}', 'Page 1 of N'],
    ['{n} / {total}', '1 / N']
  ];
  const SPOTS: Array<[Position, string]> = [
    ['top-left', 'Top left'],
    ['top-center', 'Top centre'],
    ['top-right', 'Top right'],
    ['bottom-left', 'Bottom left'],
    ['bottom-center', 'Bottom centre'],
    ['bottom-right', 'Bottom right']
  ];

  let position = $state<Position>('bottom-center');
  let margin = $state(10);
  let fontSize = $state(11);
  let color = $state('#000000');
  let preset = $state('Page {n} of {total}');
  let custom = $state('Page {n}');
  let start = $state(1);
  let range = $state<'all' | 'pages'>('all');
  let pagesText = $state('');
  let saving = $state(false);
  let problem = $state('');

  const format = $derived(preset === 'custom' ? custom : preset);
  const marginOk = $derived(Number.isFinite(margin) && margin >= 0 && margin <= 100);
  const sizeOk = $derived(Number.isFinite(fontSize) && fontSize >= 1 && fontSize <= 500);
  const startOk = $derived(Number.isInteger(start) && Math.abs(start) <= 1_000_000);
  const complete = $derived(
    marginOk && sizeOk && startOk && format.trim() !== '' && (range === 'all' || !!pagesText.trim())
  );

  const stamping = $derived<Stamping | null>(
    complete
      ? {
          tool: 'page_numbers',
          position,
          margin_mm: margin,
          font_size: fontSize,
          color,
          format,
          start,
          pages: range === 'pages' ? pagesText : null
        }
      : null
  );
  const ready = $derived(!!doc && !!stamping && !problem && !saving);

  async function saveAs() {
    if (!doc || !stamping || !ready) return;
    const output = await save({
      title: 'Save numbered PDF',
      defaultPath: `${folderOf(doc.path)}${stemOf(doc.path)} (numbered).pdf`,
      filters: [{ name: 'PDF', extensions: ['pdf'] }]
    });
    if (!output) return;
    saving = true;
    try {
      announceSaved(await stampPdf(doc.path, output, stamping));
    } catch (e) {
      report(e);
    } finally {
      saving = false;
    }
  }

  function onkeydown(e: KeyboardEvent) {
    if (!(e.ctrlKey || e.metaKey)) return;
    const key = e.key.toLowerCase();
    if (key === 'o') {
      e.preventDefault();
      void browseForPdf();
    } else if (key === 's' && doc) {
      e.preventDefault();
      void saveAs();
    }
  }

  onMount(() => {
    window.addEventListener('keydown', onkeydown);
    return () => window.removeEventListener('keydown', onkeydown);
  });
</script>

<ToolFrame
  title="Page numbers"
  subtitle={doc ? doc.name : 'Add page numbers to every page.'}
  empty={!doc}
  emptyHint="Your original is never changed. The numbered copy is saved as a new file."
  onopen={browseForPdf}
>
  {#snippet actions()}
    <button class="btn" title="Open (Ctrl+O)" onclick={browseForPdf}>
      <FolderOpen size={16} strokeWidth={1.75} /> Open
    </button>
    {#if doc}
      <button class="btn primary" title="Save (Ctrl+S)" disabled={!ready} onclick={saveAs}>
        <Hash size={16} strokeWidth={1.75} />
        {saving ? 'Saving…' : 'Save as…'}
      </button>
    {/if}
  {/snippet}

  {#if doc}
    <div class="opts-layout">
      <form class="opts" onsubmit={(e) => e.preventDefault()}>
        <div class="field">
          <span>Position</span>
          <div class="spots" role="radiogroup" aria-label="Position">
            {#each SPOTS as [value, label] (value)}
              <label class="spot" class:on={position === value} title={label}>
                <input type="radio" name="spot" {value} bind:group={position} />
                <span class="dot"></span>
                <span class="sr">{label}</span>
              </label>
            {/each}
          </div>
        </div>

        <label class="field">
          <span>Format</span>
          <select bind:value={preset}>
            {#each FORMATS as [value, label] (value)}
              <option {value}>{label}</option>
            {/each}
            <option value="custom">Custom text…</option>
          </select>
        </label>
        {#if preset === 'custom'}
          <label class="field">
            <span>Text ({'{n}'} is the number, {'{total}'} the last number)</span>
            <input type="text" bind:value={custom} class:invalid={!custom.trim()} />
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
              bind:value={fontSize}
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
              bind:value={margin}
              class:invalid={!marginOk}
            />
          </label>
          <label class="field">
            <span>Colour</span>
            <input type="color" bind:value={color} />
          </label>
        </div>

        <div class="row">
          <label class="field">
            <span>Start at</span>
            <input type="number" step="1" bind:value={start} class:invalid={!startOk} />
          </label>
          <label class="field">
            <span>Number</span>
            <select bind:value={range}>
              <option value="all">All pages</option>
              <option value="pages">Only these pages…</option>
            </select>
          </label>
        </div>
        {#if range === 'pages'}
          <label class="field">
            <span>Pages</span>
            <input
              type="text"
              placeholder="2-end"
              bind:value={pagesText}
              class:invalid={!pagesText.trim()}
            />
          </label>
          <p class="note">
            To skip the cover, type 2-end. The first numbered page gets the start number.
          </p>
        {/if}
        <p class="note">
          Numbers use the standard Helvetica font, so they work in every viewer. Rotated and
          mixed-size pages are numbered where they appear on screen.
        </p>
      </form>

      <StampPreview
        source={doc.path}
        pageCount={count}
        rotations={doc.info.pages.map((p) => p.rotate)}
        {stamping}
        bind:error={problem}
      />
    </div>
  {/if}

  {#snippet footer()}
    <span>{plural(count, 'page')}</span>
    {#if doc}<span>Source {fmtSize(doc.size)}</span>{/if}
  {/snippet}
</ToolFrame>

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
