<script lang="ts">
  import { onMount } from 'svelte';
  import { confirm, open as openDialog } from '@tauri-apps/plugin-dialog';
  import FolderOpen from 'lucide-svelte/icons/folder-open';
  import Scissors from 'lucide-svelte/icons/scissors';
  import PageThumb from '../components/PageThumb.svelte';
  import Preview from '../components/Preview.svelte';
  import ToolFrame from '../components/ToolFrame.svelte';
  import { errorMessage, parsePageRanges, splitPdf, type SplitMode } from '../api';
  import { fmtSize, plural } from '../format';
  import { folderOf } from '../paths';
  import { announceFiles, report } from '../saved';
  import { browseForPdf, documents } from '../stores/documents.svelte';
  import type { PageItem } from '../stores/pageEdit.svelte';
  import { preview } from '../stores/preview.svelte';

  type Mode = 'every' | 'ranges' | 'at';

  const MANY_FILES = 100;

  const doc = $derived(documents.current);
  const count = $derived(doc?.info.pages.length ?? 0);
  const items = $derived<PageItem[]>(
    Array.from({ length: count }, (_, i) => ({ id: i + 1, kind: 'page', src: i + 1, rotate: 0 }))
  );

  let mode = $state<Mode>('every');
  let every = $state<number | null>(1);
  let rangesText = $state('');
  let parsed = $state.raw<Array<[number, number]>>([]);
  let rangeError = $state('');
  // Page numbers after which the document is cut. Only replaced, never mutated.
  let markers = $state.raw<ReadonlySet<number>>(new Set());
  let saving = $state(false);

  // A newly opened document starts with fresh settings.
  let seenKey = '';
  $effect(() => {
    const key = doc?.key ?? '';
    if (key === seenKey) return;
    seenKey = key;
    markers = new Set();
    rangesText = '';
  });

  // Live validation of the range text by the same parser Rust uses to split.
  let parseRun = 0;
  $effect(() => {
    const text = rangesText;
    const pages = count;
    const run = ++parseRun;
    if (mode !== 'ranges' || !pages || !text.trim()) {
      parsed = [];
      rangeError = '';
      return;
    }
    parsePageRanges(text, pages)
      .then((r) => {
        if (run === parseRun) [parsed, rangeError] = [r, ''];
      })
      .catch((e) => {
        if (run === parseRun) [parsed, rangeError] = [[], errorMessage(e)];
      });
  });

  const perFile = $derived(Math.max(1, Math.floor(every ?? 1)));
  const fileCount = $derived(
    mode === 'every'
      ? Math.ceil(count / perFile)
      : mode === 'ranges'
        ? parsed.length
        : markers.size + 1
  );
  const ready = $derived(
    mode === 'every' ? count > 0 : mode === 'ranges' ? parsed.length > 0 : markers.size > 0
  );

  /** Which output file a page lands in (-1: none), shown as alternating tints. */
  function groupOf(page: number): number {
    if (mode === 'every') return Math.floor((page - 1) / perFile);
    if (mode === 'ranges') return parsed.findIndex(([a, b]) => page >= a && page <= b);
    let g = 0;
    for (const m of markers) if (m < page) g++;
    return g;
  }

  function tintOf(page: number): 'a' | 'b' | null {
    const g = groupOf(page);
    return g < 0 ? null : g % 2 === 0 ? 'a' : 'b';
  }

  function toggleMarker(after: number) {
    // eslint-disable-next-line svelte/prefer-svelte-reactivity -- copied, then replaced
    const next = new Set(markers);
    if (!next.delete(after)) next.add(after);
    markers = next;
  }

  function currentMode(): SplitMode {
    if (mode === 'every') return { kind: 'every', n: perFile };
    if (mode === 'ranges') return { kind: 'ranges', text: rangesText };
    return { kind: 'at', after: [...markers] };
  }

  async function split() {
    if (!doc || !ready || saving) return;
    if (fileCount > MANY_FILES) {
      const go = await confirm(`This will create ${fileCount} files. Continue?`, {
        title: 'Many files',
        kind: 'warning',
        okLabel: `Create ${fileCount} files`,
        cancelLabel: 'Cancel'
      });
      if (!go) return;
    }
    const folder = await openDialog({
      directory: true,
      title: 'Choose a folder for the split files',
      defaultPath: folderOf(doc.path)
    });
    if (!folder) return;
    saving = true;
    try {
      announceFiles(await splitPdf(doc.path, folder, currentMode()));
    } catch (e) {
      report(e);
    } finally {
      saving = false;
    }
  }

  function onkeydown(e: KeyboardEvent) {
    if (preview.id !== null) return;
    const t = e.target as HTMLElement | null;
    if (t && /^(INPUT|SELECT|TEXTAREA)$/.test(t.tagName)) return;
    if (!(e.ctrlKey || e.metaKey)) return;
    const key = e.key.toLowerCase();
    if (key === 'o') {
      e.preventDefault();
      void browseForPdf();
    } else if (key === 's') {
      e.preventDefault();
      void split();
    }
  }

  onMount(() => {
    window.addEventListener('keydown', onkeydown);
    return () => {
      window.removeEventListener('keydown', onkeydown);
      preview.close();
    };
  });
</script>

<ToolFrame
  title="Split"
  subtitle={doc ? doc.name : 'Cut a PDF into several smaller files.'}
  empty={!doc}
  emptyHint="Your original is never changed. The parts are saved as new files."
  onopen={browseForPdf}
>
  {#snippet actions()}
    <button class="btn" title="Open (Ctrl+O)" onclick={browseForPdf}>
      <FolderOpen size={16} strokeWidth={1.75} /> Open
    </button>
    {#if doc}
      <button
        class="btn primary"
        title="Split (Ctrl+S)"
        disabled={!ready || saving}
        onclick={split}
      >
        <Scissors size={16} strokeWidth={1.75} />
        {saving ? 'Splitting…' : 'Split…'}
      </button>
    {/if}
  {/snippet}

  {#snippet toolbar()}
    <div class="segmented" role="group" aria-label="Split mode">
      <button aria-pressed={mode === 'every'} onclick={() => (mode = 'every')}>Every N pages</button
      >
      <button aria-pressed={mode === 'ranges'} onclick={() => (mode = 'ranges')}>By ranges</button>
      <button aria-pressed={mode === 'at'} onclick={() => (mode = 'at')}>At markers</button>
    </div>
    {#if mode === 'every'}
      <label>
        Pages per file
        <input type="number" min="1" max={count} bind:value={every} />
      </label>
    {:else if mode === 'ranges'}
      <label>
        Ranges
        <input
          type="text"
          size="28"
          class:invalid={!!rangeError}
          placeholder="1-3, 4-8, 9-end"
          aria-invalid={!!rangeError}
          bind:value={rangesText}
        />
      </label>
      {#if rangeError}<span class="field-error" role="alert">{rangeError}</span>{/if}
    {:else}
      <span class="hint">Click between two pages to cut there.</span>
      <button class="btn" disabled={!markers.size} onclick={() => (markers = new Set())}>
        Clear markers
      </button>
    {/if}
    {#if ready}<span class="hint">→ {plural(fileCount, 'file')}</span>{/if}
  {/snippet}

  <div class="page-grid" role="list" aria-label="Pages">
    {#each items as item (item.id)}
      <div class="slot" role="listitem">
        <PageThumb
          {item}
          position={item.src}
          pdf={doc!.pdf}
          docKey={doc!.key}
          readonly
          tint={tintOf(item.src)}
          onpreview={() => preview.open(item.id)}
        />
        {#if mode === 'at' && item.src < count}
          <button
            class="cut"
            class:on={markers.has(item.src)}
            aria-pressed={markers.has(item.src)}
            title="Split after page {item.src}"
            aria-label="Split after page {item.src}"
            onclick={() => toggleMarker(item.src)}
          >
            <Scissors size={12} strokeWidth={1.75} />
          </button>
        {/if}
      </div>
    {/each}
  </div>

  {#if doc && preview.id !== null}
    <Preview
      pdf={doc.pdf}
      {items}
      id={preview.id}
      onnavigate={(id) => preview.open(id)}
      onclose={() => preview.close()}
    />
  {/if}

  {#snippet footer()}
    <span>{plural(count, 'page')}</span>
    {#if doc}<span>Source {fmtSize(doc.size)}</span>{/if}
    <span>Files are named like name_part01.pdf and never overwrite existing ones.</span>
  {/snippet}
</ToolFrame>

<style>
  .hint {
    color: var(--text-muted);
    font-size: var(--text-caption);
  }
  .slot {
    position: relative;
  }
  /* The gap between a page and the next one: a thin line that becomes a cut mark. */
  .cut {
    position: absolute;
    top: 8px;
    right: -9px;
    z-index: 1;
    display: grid;
    place-items: center;
    width: 18px;
    height: calc(100% - 16px);
    padding: 0;
    border: 0;
    border-radius: 9px;
    background: transparent;
    color: transparent;
    cursor: pointer;
    transition:
      background var(--duration-fast) var(--ease),
      color var(--duration-fast) var(--ease);
  }
  .cut:hover,
  .cut:focus-visible {
    background: var(--selected-bg);
    color: var(--primary);
  }
  .cut.on {
    background: var(--primary);
    color: var(--on-primary);
  }
</style>
