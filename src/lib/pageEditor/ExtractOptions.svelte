<script lang="ts">
  import { confirm, open as openDialog, save } from '@tauri-apps/plugin-dialog';
  import FileOutput from 'lucide-svelte/icons/file-output';
  import { errorMessage, parsePageRanges, savePages, splitPdf } from '../api';
  import { formatRanges, plural } from '../format';
  import { folderOf, stemOf } from '../paths';
  import { announceFiles, announceSaved, report } from '../saved';
  import { documents } from '../stores/documents.svelte';
  import { pageEdit } from '../stores/pageEdit.svelte';
  import { extractSettings as s } from '../stores/panels.svelte';
  import { rangesToPages } from './logic';

  const MANY_FILES = 100;

  const doc = $derived(documents.current);
  const count = $derived(pageEdit.items.length);
  const pages = $derived(pageEdit.selectedPositions);
  let saving = $state(false);

  // The text box follows the selection on the grid...
  let fromText = false;
  $effect(() => {
    const text = formatRanges(pages);
    if (fromText) {
      fromText = false;
      return;
    }
    s.text = text;
    s.textError = '';
  });

  // ...and the grid follows the text box, as soon as it is valid.
  let parseRun = 0;
  async function typed() {
    const run = ++parseRun;
    if (!s.text.trim()) {
      fromText = true;
      pageEdit.clearSelection();
      s.textError = '';
      return;
    }
    try {
      const ranges = await parsePageRanges(s.text, count);
      if (run !== parseRun) return;
      fromText = true;
      pageEdit.selectPositions(rangesToPages(ranges));
      s.textError = '';
    } catch (e) {
      if (run === parseRun) s.textError = errorMessage(e);
    }
  }

  async function extract() {
    if (!doc || !pages.length || saving) return;
    const stem = stemOf(doc.path);
    // Extract works on the edited list, so unsaved edits are part of the result.
    const edited = pageEdit.toOps();
    saving = true;
    try {
      if (s.output === 'one') {
        const target = await save({
          title: 'Save extracted pages',
          defaultPath: `${folderOf(doc.path)}${stem} (extracted).pdf`,
          filters: [{ name: 'PDF', extensions: ['pdf'] }]
        });
        if (!target) return;
        announceSaved(await savePages(doc.path, target, pageEdit.opsFor()));
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
        announceFiles(await splitPdf(doc.path, folder, { kind: 'pages', pages }, edited));
      }
    } catch (e) {
      report(e);
    } finally {
      saving = false;
    }
  }
</script>

<form class="opts" onsubmit={(e) => e.preventDefault()}>
  <label class="field">
    <span>Pages</span>
    <input
      type="text"
      class:invalid={!!s.textError}
      placeholder="2, 5-7"
      aria-invalid={!!s.textError}
      bind:value={s.text}
      oninput={typed}
    />
  </label>
  {#if s.textError}<span class="field-error" role="alert">{s.textError}</span>{/if}
  <p class="note">Select pages on the grid or type them here. Both stay in step.</p>

  <div class="row">
    <button type="button" class="btn" onclick={() => pageEdit.selectAll()}>Select all</button>
    <button
      type="button"
      class="btn"
      disabled={!pages.length}
      onclick={() => pageEdit.clearSelection()}
    >
      Clear
    </button>
  </div>

  <div class="segmented" role="group" aria-label="Output">
    <button type="button" aria-pressed={s.output === 'one'} onclick={() => (s.output = 'one')}>
      One PDF
    </button>
    <button type="button" aria-pressed={s.output === 'each'} onclick={() => (s.output = 'each')}>
      One PDF per page
    </button>
  </div>

  <button
    type="button"
    class="btn primary wide"
    disabled={!pages.length || saving}
    onclick={extract}
  >
    <FileOutput size={16} strokeWidth={1.75} />
    {saving
      ? 'Extracting…'
      : pages.length
        ? `Extract ${plural(pages.length, 'page')}…`
        : 'Extract…'}
  </button>

  <p class="note">
    Extracts the pages as you see them now, including edits you have not saved. The open document
    does not change.
  </p>
</form>

<style>
  .wide {
    justify-content: center;
  }
</style>
