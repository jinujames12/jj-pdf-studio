<script lang="ts">
  import { onMount } from 'svelte';
  import { confirm, open as openDialog, save } from '@tauri-apps/plugin-dialog';
  import FileOutput from 'lucide-svelte/icons/file-output';
  import FolderOpen from 'lucide-svelte/icons/folder-open';
  import PageThumb from '../components/PageThumb.svelte';
  import Preview from '../components/Preview.svelte';
  import ToolFrame from '../components/ToolFrame.svelte';
  import { errorMessage, parsePageRanges, savePages, splitPdf } from '../api';
  import { fmtSize, formatRanges, plural } from '../format';
  import { folderOf, stemOf } from '../paths';
  import { announceFiles, announceSaved, report } from '../saved';
  import { browseForPdf, documents } from '../stores/documents.svelte';
  import type { PageItem } from '../stores/pageEdit.svelte';
  import { preview } from '../stores/preview.svelte';
  import { createSelection } from '../stores/selection.svelte';

  const MANY_FILES = 100;

  const doc = $derived(documents.current);
  const count = $derived(doc?.info.pages.length ?? 0);
  const items = $derived<PageItem[]>(
    Array.from({ length: count }, (_, i) => ({ id: i + 1, kind: 'page', src: i + 1, rotate: 0 }))
  );

  const selection = createSelection();
  const selected = $derived(selection.selected);
  const pages = $derived([...selected].sort((a, b) => a - b));

  let output = $state<'one' | 'each'>('one');
  let text = $state('');
  let textError = $state('');
  let saving = $state(false);

  // A newly opened document starts with nothing selected.
  let seenKey = '';
  $effect(() => {
    const key = doc?.key ?? '';
    if (key === seenKey) return;
    seenKey = key;
    selection.clear();
    text = '';
    textError = '';
  });

  /** The text box follows clicks on the grid. */
  function syncText() {
    text = formatRanges(selection.selected);
    textError = '';
  }

  function click(page: number, e: MouseEvent) {
    selection.click(page, e.shiftKey ? 'range' : e.ctrlKey || e.metaKey ? 'toggle' : 'replace');
    syncText();
  }

  /** ...and clicks on the grid follow the text box, as soon as it is valid. */
  let parseRun = 0;
  async function typed() {
    const run = ++parseRun;
    if (!text.trim()) {
      selection.clear();
      textError = '';
      return;
    }
    try {
      const ranges = await parsePageRanges(text, count);
      if (run !== parseRun) return;
      selection.set(ranges.flatMap(([a, b]) => Array.from({ length: b - a + 1 }, (_, i) => a + i)));
      textError = '';
    } catch (e) {
      if (run === parseRun) textError = errorMessage(e);
    }
  }

  async function extract() {
    if (!doc || !pages.length || saving) return;
    const stem = stemOf(doc.path);
    saving = true;
    try {
      if (output === 'one') {
        const target = await save({
          title: 'Save extracted pages',
          defaultPath: `${folderOf(doc.path)}${stem} (extracted).pdf`,
          filters: [{ name: 'PDF', extensions: ['pdf'] }]
        });
        if (!target) return;
        const ops = pages.map((page) => ({ kind: 'page' as const, page, rotate: 0 }));
        announceSaved(await savePages(doc.path, target, ops));
      } else {
        if (pages.length > MANY_FILES) {
          const go = await confirm(`This will create ${pages.length} files. Continue?`, {
            title: 'Many files',
            kind: 'warning',
            okLabel: `Create ${pages.length} files`,
            cancelLabel: 'Cancel'
          });
          if (!go) return;
        }
        const folder = await openDialog({
          directory: true,
          title: 'Choose a folder for the extracted pages',
          defaultPath: folderOf(doc.path)
        });
        if (!folder) return;
        announceFiles(await splitPdf(doc.path, folder, { kind: 'pages', pages }));
      }
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
    const ctrl = e.ctrlKey || e.metaKey;
    const key = e.key.toLowerCase();
    if (ctrl && key === 'o') {
      e.preventDefault();
      void browseForPdf();
    } else if (!doc) {
      return;
    } else if (ctrl && key === 's') {
      e.preventDefault();
      void extract();
    } else if (ctrl && key === 'a') {
      e.preventDefault();
      selection.selectAll(count);
      syncText();
    } else if (e.key === 'Escape') {
      selection.clear();
      syncText();
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
  title="Extract pages"
  subtitle={doc ? doc.name : 'Pull selected pages out into a new PDF.'}
  empty={!doc}
  emptyHint="Your original is never changed. The pages are saved to new files."
  onopen={browseForPdf}
>
  {#snippet actions()}
    <button class="btn" title="Open (Ctrl+O)" onclick={browseForPdf}>
      <FolderOpen size={16} strokeWidth={1.75} /> Open
    </button>
    {#if doc}
      <button
        class="btn primary"
        title="Extract (Ctrl+S)"
        disabled={!pages.length || saving}
        onclick={extract}
      >
        <FileOutput size={16} strokeWidth={1.75} />
        {saving ? 'Extracting…' : 'Extract…'}
      </button>
    {/if}
  {/snippet}

  {#snippet toolbar()}
    <label>
      Pages
      <input
        type="text"
        size="24"
        class:invalid={!!textError}
        placeholder="2, 5-7"
        aria-invalid={!!textError}
        bind:value={text}
        oninput={typed}
      />
    </label>
    {#if textError}<span class="field-error" role="alert">{textError}</span>{/if}
    <button
      class="btn"
      onclick={() => {
        selection.selectAll(count);
        syncText();
      }}
    >
      Select all
    </button>
    <button
      class="btn"
      disabled={!pages.length}
      onclick={() => {
        selection.clear();
        syncText();
      }}
    >
      Clear
    </button>
    <div class="segmented" role="group" aria-label="Output">
      <button aria-pressed={output === 'one'} onclick={() => (output = 'one')}> One PDF </button>
      <button aria-pressed={output === 'each'} onclick={() => (output = 'each')}>
        One PDF per page
      </button>
    </div>
  {/snippet}

  <div class="page-grid" role="listbox" aria-multiselectable="true" aria-label="Pages">
    {#each items as item (item.id)}
      <PageThumb
        {item}
        position={item.src}
        pdf={doc!.pdf}
        docKey={doc!.key}
        readonly
        selected={selected.has(item.src)}
        onselect={(e) => click(item.src, e)}
        onpreview={() => preview.open(item.id)}
      />
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
    <span>{pages.length} selected</span>
    {#if doc}<span>Source {fmtSize(doc.size)}</span>{/if}
    <span>Click to select. Shift selects a range, Ctrl adds single pages.</span>
  {/snippet}
</ToolFrame>
