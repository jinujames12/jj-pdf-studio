<script lang="ts">
  import { onMount } from 'svelte';
  import { save } from '@tauri-apps/plugin-dialog';
  import FolderOpen from 'lucide-svelte/icons/folder-open';
  import Info from 'lucide-svelte/icons/info';
  import LockOpen from 'lucide-svelte/icons/lock-open';
  import PageThumb from '../components/PageThumb.svelte';
  import Preview from '../components/Preview.svelte';
  import ToolFrame from '../components/ToolFrame.svelte';
  import { protectionInfo, unlockPdf, type CopyHints, type ProtectionInfo } from '../api';
  import { plural } from '../format';
  import { folderOf, stemOf } from '../paths';
  import { announceSaved, report } from '../saved';
  import { browseForPdf, documents } from '../stores/documents.svelte';
  import type { PageItem } from '../stores/pageEdit.svelte';
  import { preview } from '../stores/preview.svelte';

  const doc = $derived(documents.current);
  const count = $derived(doc?.info.pages.length ?? 0);

  let info = $state<ProtectionInfo | null>(null);
  let working = $state(false);
  /** Why copying may still fail, from the last unlock. */
  let hints = $state<CopyHints | null>(null);

  const items = $derived<PageItem[]>(
    Array.from({ length: count }, (_, i) => ({ id: i + 1, kind: 'page', src: i + 1, rotate: 0 }))
  );
  const nothingToDo = $derived(info?.protection === 'none');

  // A newly opened document is inspected again and hides the previous notes.
  $effect(() => {
    const path = doc?.path;
    info = null;
    hints = null;
    if (!path) return;
    let current = true;
    protectionInfo(path)
      .then((p) => current && (info = p))
      .catch(report);
    return () => {
      current = false;
    };
  });

  const summary = $derived.by(() => {
    if (!info) return '';
    const how = info.algorithm ? ` (${info.algorithm})` : '';
    if (info.protection === 'open_password')
      return `This file needed a password to open${how}. Unlock saves a copy that opens without one.`;
    if (info.protection === 'restrictions_only')
      return info.blocked.length
        ? `This file opens freely but blocks ${info.blocked.join(', ')}${how}. Unlock saves a copy without these restrictions.`
        : `This file is encrypted${how} but blocks nothing. Unlock saves a copy without the encryption.`;
    return 'This file has no password or restrictions, so there is nothing to unlock.';
  });

  async function unlock() {
    if (!doc || working || nothingToDo) return;
    const output = await save({
      title: 'Save unlocked PDF',
      defaultPath: `${folderOf(doc.path)}${stemOf(doc.path)} (unlocked).pdf`,
      filters: [{ name: 'PDF', extensions: ['pdf'] }]
    });
    if (!output) return;
    working = true;
    try {
      const result = await unlockPdf(doc.path, output);
      hints = result.hints;
      announceSaved(result.saved);
    } catch (e) {
      report(e);
    } finally {
      working = false;
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
    } else if (key === 's' && doc) {
      e.preventDefault();
      void unlock();
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
  title="Unlock"
  subtitle={doc ? doc.name : 'Remove a password or restrictions from a PDF.'}
  empty={!doc}
  emptyHint="Your original is never changed. The unlocked copy is saved as a new file."
  onopen={browseForPdf}
>
  {#snippet actions()}
    <button class="btn" title="Open (Ctrl+O)" onclick={browseForPdf}>
      <FolderOpen size={16} strokeWidth={1.75} /> Open
    </button>
    {#if doc}
      <button
        class="btn primary"
        title="Unlock (Ctrl+S)"
        disabled={working || !info || nothingToDo}
        onclick={unlock}
      >
        <LockOpen size={16} strokeWidth={1.75} />
        {working ? 'Unlocking…' : 'Unlock…'}
      </button>
    {/if}
  {/snippet}

  {#snippet toolbar()}
    <span class="line">{summary}</span>
    <span class="line muted">
      Only remove protection from files you own or have permission to change.
    </span>
  {/snippet}

  {#if hints && (hints.scanned_pages.length || hints.fonts_without_unicode)}
    <div class="notice" role="status">
      <Info size={16} strokeWidth={1.75} />
      <div>
        <strong>Copying may still not work on this file.</strong>
        {#if hints.scanned_pages.length}
          <p>
            {plural(hints.scanned_pages.length, 'page')}
            ({hints.scanned_pages.join(', ')})
            {hints.scanned_pages.length === 1 ? 'is a scanned picture' : 'are scanned pictures'}, so
            there is no text to copy. Text recognition (OCR) would be needed; it is planned for a
            later version.
          </p>
        {/if}
        {#if hints.fonts_without_unicode}
          <p>
            {plural(hints.fonts_without_unicode, 'font')}
            {hints.fonts_without_unicode === 1 ? 'has' : 'have'} no Unicode map, so copied text may look
            garbled. Only OCR fixes that.
          </p>
        {/if}
      </div>
    </div>
  {/if}

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

  {#snippet footer()}
    <span>{plural(count, 'page')}</span>
  {/snippet}
</ToolFrame>

<style>
  .line {
    flex-basis: 100%;
  }
  .muted {
    color: var(--text-muted);
    font-size: var(--text-caption);
  }
  .notice {
    display: flex;
    gap: var(--space-3);
    margin: var(--space-4) var(--space-4) 0;
    padding: var(--space-3) var(--space-4);
    border: 1px solid var(--border);
    border-radius: var(--radius-card);
    background: var(--selected-bg);
  }
  .notice p {
    margin: var(--space-1) 0 0;
    color: var(--text-muted);
  }
</style>
