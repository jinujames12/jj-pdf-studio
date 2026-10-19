<script lang="ts">
  import { onMount } from 'svelte';
  import { open as openDialog, save } from '@tauri-apps/plugin-dialog';
  import FolderOpen from 'lucide-svelte/icons/folder-open';
  import Images from 'lucide-svelte/icons/images';
  import PageThumb from '../components/PageThumb.svelte';
  import Preview from '../components/Preview.svelte';
  import ToolFrame from '../components/ToolFrame.svelte';
  import {
    errorMessage,
    exportImagesAdd,
    exportImagesBegin,
    exportImagesCancel,
    exportImagesFinish,
    parsePageRanges
  } from '../api';
  import { fmtSize, formatRanges, plural } from '../format';
  import { folderOf, stemOf } from '../paths';
  import { pixelSize, renderPicture, tooLarge, type PictureFormat } from '../pdf/export';
  import { announceFiles, report } from '../saved';
  import { toast } from '../stores/toast.svelte';
  import { browseForPdf, documents } from '../stores/documents.svelte';
  import type { PageItem } from '../stores/pageEdit.svelte';
  import { preview } from '../stores/preview.svelte';
  import { createSelection } from '../stores/selection.svelte';

  const doc = $derived(documents.current);
  const count = $derived(doc?.info.pages.length ?? 0);
  const items = $derived<PageItem[]>(
    Array.from({ length: count }, (_, i) => ({ id: i + 1, kind: 'page', src: i + 1, rotate: 0 }))
  );

  const selection = createSelection();
  const selected = $derived(selection.selected);

  let scope = $state<'all' | 'selected'>('all');
  let format = $state<PictureFormat>('png');
  let quality = $state(85);
  let dpiChoice = $state<'72' | '150' | '300' | 'custom'>('150');
  let customDpi = $state(200);
  let destination = $state<'folder' | 'zip'>('folder');
  let text = $state('');
  let textError = $state('');

  let working = $state(false);
  let done = $state(0);
  let total = $state(0);
  let cancelled = false;

  const dpi = $derived(dpiChoice === 'custom' ? customDpi : Number(dpiChoice));
  const dpiOk = $derived(Number.isFinite(dpi) && dpi >= 10 && dpi <= 600);
  const pages = $derived(
    scope === 'all'
      ? Array.from({ length: count }, (_, i) => i + 1)
      : [...selected].sort((a, b) => a - b)
  );

  /** On-screen size of a page in points (its /Rotate applied). */
  function shown(page: number) {
    const p = doc!.info.pages[page - 1];
    const sideways = p.rotate % 180 !== 0;
    return { page, width: sideways ? p.height : p.width, height: sideways ? p.width : p.height };
  }

  const sizeProblem = $derived(
    doc && dpiOk && pages.length ? tooLarge(pages.map(shown), dpi) : null
  );
  /** "1240 × 1754 px" for the first page to export, as a hint. */
  const sizeHint = $derived.by(() => {
    if (!doc || !dpiOk || !pages.length) return '';
    const s = shown(pages[0]);
    const px = pixelSize(s.width, s.height, dpi);
    return `Page ${pages[0]} will be ${px.width} × ${px.height} pixels.`;
  });
  const ready = $derived(
    !!doc && !working && dpiOk && pages.length > 0 && !sizeProblem && !textError
  );

  // A newly opened document starts with every page and nothing selected.
  let seenKey = '';
  $effect(() => {
    const key = doc?.key ?? '';
    if (key === seenKey) return;
    seenKey = key;
    selection.clear();
    scope = 'all';
    text = '';
    textError = '';
  });

  function syncText() {
    text = formatRanges(selection.selected);
    textError = '';
  }

  function click(page: number, e: MouseEvent) {
    scope = 'selected';
    selection.click(page, e.shiftKey ? 'range' : e.ctrlKey || e.metaKey ? 'toggle' : 'replace');
    syncText();
  }

  let parseRun = 0;
  async function typed() {
    const run = ++parseRun;
    scope = 'selected';
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

  async function exportPictures() {
    if (!doc || !ready) return;
    const target =
      destination === 'zip'
        ? await save({
            title: 'Save pictures as a ZIP file',
            defaultPath: `${folderOf(doc.path)}${stemOf(doc.path)} (images).zip`,
            filters: [{ name: 'ZIP file', extensions: ['zip'] }]
          })
        : await openDialog({
            directory: true,
            title: 'Choose a folder for the pictures',
            defaultPath: folderOf(doc.path)
          });
    if (!target) return;

    const list = pages;
    working = true;
    cancelled = false;
    done = 0;
    total = list.length;
    try {
      await exportImagesBegin(doc.path, target, destination === 'zip', count);
      // One page at a time, so memory stays at a single picture.
      for (const page of list) {
        if (cancelled) break;
        const bytes = await renderPicture(doc.pdf, page, dpi, format, quality);
        if (cancelled) break;
        await exportImagesAdd(page, format, bytes);
        done++;
      }
      if (cancelled) {
        await exportImagesCancel();
        toast('info', 'Export cancelled. No files were kept.');
      } else {
        announceFiles(await exportImagesFinish());
      }
    } catch (e) {
      await exportImagesCancel().catch(() => {});
      report(e);
    } finally {
      working = false;
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
      void exportPictures();
    } else if (ctrl && key === 'a') {
      e.preventDefault();
      scope = 'selected';
      selection.selectAll(count);
      syncText();
    } else if (key === 'escape') {
      selection.clear();
      syncText();
    }
  }

  onMount(() => {
    window.addEventListener('keydown', onkeydown);
    return () => {
      window.removeEventListener('keydown', onkeydown);
      cancelled = true;
      preview.close();
    };
  });
</script>

<ToolFrame
  title="PDF to Images"
  subtitle={doc ? doc.name : 'Save pages as PNG or JPG pictures.'}
  empty={!doc}
  emptyHint="Your original is never changed. The pictures are saved to a folder or a ZIP file."
  onopen={browseForPdf}
>
  {#snippet actions()}
    <button class="btn" title="Open (Ctrl+O)" onclick={browseForPdf} disabled={working}>
      <FolderOpen size={16} strokeWidth={1.75} /> Open
    </button>
    {#if doc}
      {#if working}
        <button class="btn" onclick={() => (cancelled = true)}>Cancel</button>
      {/if}
      <button
        class="btn primary"
        title="Export (Ctrl+S)"
        disabled={!ready}
        onclick={exportPictures}
      >
        <Images size={16} strokeWidth={1.75} />
        {working ? `Page ${Math.min(done + 1, total)} of ${total}…` : 'Export…'}
      </button>
    {/if}
  {/snippet}

  {#snippet toolbar()}
    <div class="segmented" role="group" aria-label="Pages to export">
      <button aria-pressed={scope === 'all'} onclick={() => (scope = 'all')}>All pages</button>
      <button aria-pressed={scope === 'selected'} onclick={() => (scope = 'selected')}>
        Selected pages
      </button>
    </div>
    {#if scope === 'selected'}
      <input
        type="text"
        size="18"
        class:invalid={!!textError}
        placeholder="2, 5-7"
        aria-label="Pages to export"
        bind:value={text}
        oninput={typed}
      />
    {/if}
    <label>
      Format
      <select bind:value={format}>
        <option value="png">PNG (sharp, larger)</option>
        <option value="jpg">JPG (smaller)</option>
      </select>
    </label>
    {#if format === 'jpg'}
      <label>
        Quality
        <input type="range" min="10" max="100" step="5" bind:value={quality} />
        {quality}
      </label>
    {/if}
    <label>
      Resolution
      <select bind:value={dpiChoice}>
        <option value="72">72 DPI (screen)</option>
        <option value="150">150 DPI</option>
        <option value="300">300 DPI (print)</option>
        <option value="custom">Custom…</option>
      </select>
    </label>
    {#if dpiChoice === 'custom'}
      <input
        type="number"
        min="10"
        max="600"
        class:invalid={!dpiOk}
        aria-label="DPI"
        bind:value={customDpi}
      />
    {/if}
    <label>
      Save to
      <select bind:value={destination}>
        <option value="folder">A folder</option>
        <option value="zip">One ZIP file</option>
      </select>
    </label>
    {#if textError}<span class="field-error" role="alert">{textError}</span>{/if}
    {#if sizeProblem}<span class="field-error" role="alert">{sizeProblem}</span>{/if}
  {/snippet}

  <div class="page-grid" role="listbox" aria-multiselectable="true" aria-label="Pages">
    {#each items as item (item.id)}
      <PageThumb
        {item}
        position={item.src}
        pdf={doc!.pdf}
        docKey={doc!.key}
        readonly
        selected={scope === 'selected' && selected.has(item.src)}
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
    <span>{plural(pages.length, 'page')} to export</span>
    {#if doc}<span>Source {fmtSize(doc.size)}</span>{/if}
    {#if sizeHint}<span>{sizeHint}</span>{/if}
    <span
      >Click pages to choose some. Files are named {doc
        ? stemOf(doc.path)
        : 'name'}_p001.{format}.</span
    >
  {/snippet}
</ToolFrame>

{#if working}
  <progress class="export-progress" max={total} value={done} aria-label="Export progress"
  ></progress>
{/if}

<style>
  .export-progress {
    position: fixed;
    right: var(--space-6);
    bottom: var(--space-6);
    width: 240px;
    accent-color: var(--primary);
  }
</style>
