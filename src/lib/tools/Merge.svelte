<script lang="ts">
  import { onMount } from 'svelte';
  import { open as openDialog, save } from '@tauri-apps/plugin-dialog';
  import FolderOpen from 'lucide-svelte/icons/folder-open';
  import Merge from 'lucide-svelte/icons/merge';
  import X from 'lucide-svelte/icons/x';
  import PageThumb from '../components/PageThumb.svelte';
  import ToolFrame from '../components/ToolFrame.svelte';
  import { mergePdfs } from '../api';
  import { fmtSize, plural } from '../format';
  import { folderOf } from '../paths';
  import { announceSaved, report } from '../saved';
  import { sortable } from '../actions/sortable';
  import { merge, type MergeFile, type PageEntry } from '../stores/merge.svelte';
  import { toast } from '../stores/toast.svelte';

  let bookmarks = $state(true);
  let saving = $state(false);

  const files = $derived(merge.files);

  async function addFiles() {
    const picked = await openDialog({
      multiple: true,
      title: 'Add PDFs to merge',
      filters: [{ name: 'PDF', extensions: ['pdf'] }]
    });
    if (picked?.length) await merge.addPaths(picked);
  }

  async function saveMerged() {
    if (saving || !files.length) return;
    const output = await save({
      title: 'Save merged PDF',
      defaultPath: `${folderOf(files[0].path)}${files[0].stem} (merged).pdf`,
      filters: [{ name: 'PDF', extensions: ['pdf'] }]
    });
    if (!output) return;
    saving = true;
    try {
      announceSaved(await mergePdfs(merge.toInputs(), output, bookmarks));
    } catch (e) {
      report(e);
    } finally {
      saving = false;
    }
  }

  async function setView(page: boolean) {
    if (page) merge.showPages();
    else await merge.showFiles();
  }

  function removePage(id: number) {
    if (!merge.removePage(id)) toast('info', 'At least one page must remain.');
  }

  /** On-screen size of a page of `file`, after its own /Rotate and the user's turn. */
  function geometry(file: MergeFile, src: number, turn: number) {
    const p = file.info.pages[src - 1];
    const rotation = (((p.rotate + turn) % 360) + 360) % 360;
    const swap = rotation % 180 !== 0;
    return {
      rotation,
      size: swap ? { width: p.height, height: p.width } : { width: p.width, height: p.height }
    };
  }

  function entryFile(e: PageEntry): MergeFile {
    return merge.file(e.fileId)!;
  }

  function onkeydown(e: KeyboardEvent) {
    const t = e.target as HTMLElement | null;
    if (t && /^(INPUT|SELECT|TEXTAREA)$/.test(t.tagName)) return;
    if (!(e.ctrlKey || e.metaKey)) return;
    const key = e.key.toLowerCase();
    if (key === 'o') {
      e.preventDefault();
      void addFiles();
    } else if (key === 's') {
      e.preventDefault();
      void saveMerged();
    }
  }

  onMount(() => {
    window.addEventListener('keydown', onkeydown);
    return () => window.removeEventListener('keydown', onkeydown);
  });
</script>

<ToolFrame
  title="Merge PDFs"
  subtitle={files.length
    ? `${plural(files.length, 'file')} · ${plural(merge.pageCount, 'page')}`
    : 'Combine several PDFs into one document.'}
  empty={files.length === 0}
  emptyTitle={merge.loading ? 'Opening…' : 'Drop PDFs here or click to browse'}
  emptyHint="Add two or more files, put them in order, then save one merged PDF."
  onopen={addFiles}
>
  {#snippet actions()}
    <button class="btn" title="Add files (Ctrl+O)" onclick={addFiles}>
      <FolderOpen size={16} strokeWidth={1.75} />
      {merge.loading ? 'Adding…' : 'Add files'}
    </button>
    {#if files.length}
      <button
        class="btn primary"
        title="Save (Ctrl+S)"
        disabled={saving || merge.loading}
        onclick={saveMerged}
      >
        <Merge size={16} strokeWidth={1.75} />
        {saving ? 'Merging…' : 'Save merged PDF'}
      </button>
    {/if}
  {/snippet}

  {#snippet toolbar()}
    <div class="segmented" role="group" aria-label="View">
      <button aria-pressed={!merge.pageView} onclick={() => setView(false)}>File view</button>
      <button aria-pressed={merge.pageView} onclick={() => setView(true)}>Page view</button>
    </div>
    <label>
      <input type="checkbox" bind:checked={bookmarks} />
      Add bookmarks for each file
    </label>
    <button class="btn" onclick={() => merge.clear()}>Clear all</button>
  {/snippet}

  {#if merge.entries}
    {@const entries = merge.entries}
    <div
      class="page-grid"
      role="list"
      aria-label="Pages to merge"
      use:sortable={{ draggable: '.cell', onmove: (id, to) => merge.movePage(id, to) }}
    >
      {#each entries as entry, i (entry.id)}
        {@const file = entryFile(entry)}
        {@const geo = geometry(file, entry.src, entry.rotate)}
        <PageThumb
          item={{ id: entry.id, kind: 'page', src: entry.src, rotate: entry.rotate }}
          position={i + 1}
          pdf={file.pdf}
          docKey={file.key}
          size={geo.size}
          rotation={geo.rotation}
          label="{i + 1} · {file.stem} p{entry.src}"
          onrotate={(deg) => merge.rotatePage(entry.id, deg)}
          ondelete={() => removePage(entry.id)}
        />
      {/each}
    </div>
  {:else}
    <div
      class="page-grid"
      role="list"
      aria-label="Files to merge"
      use:sortable={{ draggable: '.file-card', onmove: (id, to) => merge.moveFile(id, to) }}
    >
      {#each files as file, i (file.id)}
        {@const geo = geometry(file, 1, 0)}
        <div class="file-card" role="listitem" data-id={file.id}>
          <PageThumb
            item={{ id: file.id, kind: 'page', src: 1, rotate: 0 }}
            position={i + 1}
            pdf={file.pdf}
            docKey={file.key}
            size={geo.size}
            rotation={geo.rotation}
            label="{i + 1}. {file.name}"
            readonly
          />
          <span class="meta">{plural(file.info.pages.length, 'page')} · {fmtSize(file.size)}</span>
          <button
            class="remove"
            title="Remove {file.name}"
            aria-label="Remove {file.name}"
            onclick={() => merge.removeFile(file.id)}
          >
            <X size={14} strokeWidth={1.75} />
          </button>
        </div>
      {/each}
    </div>
  {/if}

  {#snippet footer()}
    <span>{plural(files.length, 'file')}</span>
    <span>{plural(merge.pageCount, 'page')}</span>
    <span>Sources {fmtSize(merge.totalSize)}</span>
    <span>
      {merge.pageView
        ? 'Drag pages to reorder. Hover a page to rotate or remove it.'
        : 'Drag files to reorder.'}
    </span>
  {/snippet}
</ToolFrame>

<style>
  .file-card {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    padding-bottom: var(--space-2);
    border-radius: var(--radius-card);
    cursor: grab;
  }
  .meta {
    color: var(--text-muted);
    font-size: var(--text-caption);
  }
  .remove {
    position: absolute;
    top: 6px;
    right: 6px;
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border: 1px solid var(--border);
    border-radius: var(--radius-control);
    background: var(--surface);
    color: var(--text);
    cursor: pointer;
    opacity: 0;
    transition: opacity var(--duration-fast) var(--ease);
  }
  .file-card:hover .remove,
  .remove:focus-visible {
    opacity: 1;
  }
  .remove:hover {
    color: var(--danger);
  }
</style>
