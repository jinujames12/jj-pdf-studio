<script lang="ts">
  import { onMount } from 'svelte';
  import { save } from '@tauri-apps/plugin-dialog';
  import FolderOpen from 'lucide-svelte/icons/folder-open';
  import Minimize2 from 'lucide-svelte/icons/minimize-2';
  import CompressResultDialog from '../components/CompressResult.svelte';
  import PageThumb from '../components/PageThumb.svelte';
  import Preview from '../components/Preview.svelte';
  import ToolFrame from '../components/ToolFrame.svelte';
  import { compressPdf, type CompressPreset, type CompressResult } from '../api';
  import { fmtSize, plural } from '../format';
  import { folderOf, stemOf } from '../paths';
  import { report } from '../saved';
  import { browseForPdf, documents } from '../stores/documents.svelte';
  import type { PageItem } from '../stores/pageEdit.svelte';
  import { preview } from '../stores/preview.svelte';

  const presets: Array<{ id: CompressPreset; name: string; detail: string }> = [
    {
      id: 'lossless',
      name: 'Lossless',
      detail:
        'Removes unused data and compresses what is not compressed yet. Nothing visibly changes.'
    },
    {
      id: 'balanced',
      name: 'Balanced',
      detail:
        'Lossless, plus images above 200 DPI are reduced to 150 DPI as JPEG (quality 75). Best for scans and photos.'
    },
    {
      id: 'strong',
      name: 'Strong',
      detail:
        'Lossless, plus images above 150 DPI are reduced to 100 DPI as JPEG (quality 60). Smallest file; images look softer.'
    }
  ];

  const doc = $derived(documents.current);
  const count = $derived(doc?.info.pages.length ?? 0);

  let preset = $state<CompressPreset>('balanced');
  let grayscale = $state(false);
  let working = $state(false);
  let result = $state<CompressResult | null>(null);

  const detail = $derived(presets.find((p) => p.id === preset)?.detail ?? '');
  const items = $derived<PageItem[]>(
    Array.from({ length: count }, (_, i) => ({ id: i + 1, kind: 'page', src: i + 1, rotate: 0 }))
  );

  // A newly opened document hides the result of the previous one.
  $effect(() => {
    void doc?.key;
    result = null;
  });

  async function compress() {
    if (!doc || working) return;
    const output = await save({
      title: 'Save compressed PDF',
      defaultPath: `${folderOf(doc.path)}${stemOf(doc.path)} (compressed).pdf`,
      filters: [{ name: 'PDF', extensions: ['pdf'] }]
    });
    if (!output) return;
    working = true;
    try {
      result = await compressPdf(doc.path, output, {
        preset,
        grayscale: preset === 'strong' && grayscale
      });
    } catch (e) {
      report(e);
    } finally {
      working = false;
    }
  }

  function onkeydown(e: KeyboardEvent) {
    if (preview.id !== null || result) return;
    const t = e.target as HTMLElement | null;
    if (t && /^(INPUT|SELECT|TEXTAREA)$/.test(t.tagName)) return;
    if (!(e.ctrlKey || e.metaKey)) return;
    const key = e.key.toLowerCase();
    if (key === 'o') {
      e.preventDefault();
      void browseForPdf();
    } else if (key === 's' && doc) {
      e.preventDefault();
      void compress();
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
  title="Compress"
  subtitle={doc ? doc.name : 'Make a PDF smaller for email and storage.'}
  empty={!doc}
  emptyHint="Your original is never changed. The smaller copy is saved as a new file."
  onopen={browseForPdf}
>
  {#snippet actions()}
    <button class="btn" title="Open (Ctrl+O)" onclick={browseForPdf}>
      <FolderOpen size={16} strokeWidth={1.75} /> Open
    </button>
    {#if doc}
      <button class="btn primary" title="Compress (Ctrl+S)" disabled={working} onclick={compress}>
        <Minimize2 size={16} strokeWidth={1.75} />
        {working ? 'Compressing…' : 'Compress…'}
      </button>
    {/if}
  {/snippet}

  {#snippet toolbar()}
    <div class="segmented" role="group" aria-label="Compression level">
      {#each presets as p (p.id)}
        <button aria-pressed={preset === p.id} onclick={() => (preset = p.id)}>{p.name}</button>
      {/each}
    </div>
    {#if preset === 'strong'}
      <label>
        <input type="checkbox" bind:checked={grayscale} />
        Convert colour images to grayscale
      </label>
    {/if}
    <span class="detail">{detail}</span>
  {/snippet}

  <div class="page-grid" role="list" aria-label="Pages">
    {#each items as item (item.id)}
      <PageThumb
        {item}
        position={item.src}
        pdf={doc!.pdf}
        docKey={doc!.key}
        readonly
        label={`${item.src}`}
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

  {#if result}
    <CompressResultDialog {result} onclose={() => (result = null)} />
  {/if}

  {#snippet footer()}
    <span>{plural(count, 'page')}</span>
    {#if doc}<span>Source {fmtSize(doc.size)}</span>{/if}
    <span>Text-only and vector PDFs shrink little. Fonts are kept as they are.</span>
  {/snippet}
</ToolFrame>

<style>
  .detail {
    flex-basis: 100%;
    color: var(--text-muted);
    font-size: var(--text-caption);
  }
</style>
