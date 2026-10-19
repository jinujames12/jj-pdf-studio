<script lang="ts">
  import { confirm, open as openDialog } from '@tauri-apps/plugin-dialog';
  import Scissors from 'lucide-svelte/icons/scissors';
  import { errorMessage, parsePageRanges, splitPdf, type SplitMode } from '../api';
  import { plural } from '../format';
  import { folderOf } from '../paths';
  import { announceFiles, report } from '../saved';
  import { documents } from '../stores/documents.svelte';
  import { pageEdit } from '../stores/pageEdit.svelte';
  import { splitSettings as s, splitView } from '../stores/panels.svelte';
  import { splitFileCount, splitReady } from './logic';

  const MANY_FILES = 100;

  const doc = $derived(documents.current);
  const count = $derived(pageEdit.items.length);
  let saving = $state(false);

  // Live validation of the range text by the same parser Rust uses to split.
  let parseRun = 0;
  $effect(() => {
    const text = s.rangesText;
    const pages = count;
    const run = ++parseRun;
    if (s.mode !== 'ranges' || !pages || !text.trim()) {
      s.parsed = [];
      s.rangeError = '';
      return;
    }
    parsePageRanges(text, pages)
      .then((r) => {
        if (run === parseRun) [s.parsed, s.rangeError] = [r, ''];
      })
      .catch((e) => {
        if (run === parseRun) [s.parsed, s.rangeError] = [[], errorMessage(e)];
      });
  });

  const view = $derived(splitView(pageEdit.items.map((i) => i.id)));
  const fileCount = $derived(splitFileCount(view, count));
  const ready = $derived(splitReady(view, count));

  function currentMode(): SplitMode {
    if (s.mode === 'every') return { kind: 'every', n: view.perFile };
    if (s.mode === 'ranges') return { kind: 'ranges', text: s.rangesText };
    return { kind: 'at', after: [...view.cuts] };
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
      // The edited list goes along, so unsaved edits are part of the split.
      announceFiles(await splitPdf(doc.path, folder, currentMode(), pageEdit.toOps()));
    } catch (e) {
      report(e);
    } finally {
      saving = false;
    }
  }
</script>

<form class="opts" onsubmit={(e) => e.preventDefault()}>
  <div class="segmented" role="group" aria-label="Split mode">
    <button type="button" aria-pressed={s.mode === 'every'} onclick={() => (s.mode = 'every')}>
      Every N pages
    </button>
    <button type="button" aria-pressed={s.mode === 'ranges'} onclick={() => (s.mode = 'ranges')}>
      By ranges
    </button>
    <button type="button" aria-pressed={s.mode === 'at'} onclick={() => (s.mode = 'at')}>
      At markers
    </button>
  </div>

  {#if s.mode === 'every'}
    <label class="field">
      <span>Pages per file</span>
      <input type="number" min="1" max={count} bind:value={s.every} />
    </label>
  {:else if s.mode === 'ranges'}
    <label class="field">
      <span>Ranges</span>
      <input
        type="text"
        class:invalid={!!s.rangeError}
        placeholder="1-3, 4-8, 9-end"
        aria-invalid={!!s.rangeError}
        bind:value={s.rangesText}
      />
    </label>
    {#if s.rangeError}<span class="field-error" role="alert">{s.rangeError}</span>{/if}
  {:else}
    <p class="note">Click between two pages on the grid to cut there.</p>
    <button
      type="button"
      class="btn"
      disabled={!s.markers.size}
      onclick={() => (s.markers = new Set())}
    >
      Clear markers
    </button>
  {/if}

  {#if ready}<p class="note">→ {plural(fileCount, 'file')}</p>{/if}
  {#if fileCount > MANY_FILES}
    <p class="note warn" role="alert">That is a lot of files; you will be asked to confirm.</p>
  {/if}

  <button type="button" class="btn primary wide" disabled={!ready || saving} onclick={split}>
    <Scissors size={16} strokeWidth={1.75} />
    {saving ? 'Splitting…' : ready ? `Split into ${plural(fileCount, 'file')}…` : 'Split…'}
  </button>

  <p class="note">
    Splits the pages as you see them now, including edits you have not saved. Files are named like
    name_part01.pdf (name_p1-3.pdf for ranges) and never overwrite existing ones.
  </p>
</form>

<style>
  .wide {
    justify-content: center;
  }
  .warn {
    color: var(--warning);
  }
</style>
