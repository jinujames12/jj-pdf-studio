<script lang="ts">
  import { onMount, tick } from 'svelte';
  import Sortable from 'sortablejs';
  import { save } from '@tauri-apps/plugin-dialog';
  import FolderOpen from 'lucide-svelte/icons/folder-open';
  import Undo2 from 'lucide-svelte/icons/undo-2';
  import Redo2 from 'lucide-svelte/icons/redo-2';
  import RotateCcw from 'lucide-svelte/icons/rotate-ccw';
  import RotateCw from 'lucide-svelte/icons/rotate-cw';
  import Trash2 from 'lucide-svelte/icons/trash-2';
  import FilePlus from 'lucide-svelte/icons/file-plus';
  import Save from 'lucide-svelte/icons/save';
  import PageThumb from '../components/PageThumb.svelte';
  import Preview from '../components/Preview.svelte';
  import { errorMessage, openSaved, revealSaved, savePages } from '../api';
  import { browseForPdf, documents, folderOf } from '../stores/documents.svelte';
  import { BLANK_SIZES, pageEdit, type BlankSize } from '../stores/pageEdit.svelte';
  import { preview } from '../stores/preview.svelte';
  import { toast } from '../stores/toast.svelte';

  const doc = $derived(documents.current);
  const edit = pageEdit;

  let grid = $state<HTMLElement>();
  let blankSize = $state<BlankSize>('same');
  let saving = $state(false);

  const reducedMotion = () => window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  // Drag-to-reorder. Sortable moves DOM nodes itself; we put the node back and
  // apply the move to the state instead, so Svelte stays the only DOM owner.
  // forceFallback uses pointer events, because Tauri's native file-drop handling
  // disables HTML5 drag and drop on Windows.
  $effect(() => {
    if (!grid) return;
    const sortable = Sortable.create(grid, {
      draggable: '.cell',
      filter: '.thumb-btn',
      preventOnFilter: false,
      forceFallback: true,
      fallbackTolerance: 4,
      animation: reducedMotion() ? 0 : 150,
      ghostClass: 'ghost',
      onEnd(evt) {
        const { item, from, oldIndex, newIndex } = evt;
        if (oldIndex === undefined || newIndex === undefined) return;
        from.removeChild(item);
        from.insertBefore(item, from.children[oldIndex] ?? null);
        const id = Number(item.dataset.id);
        if (oldIndex !== newIndex) edit.move(id, newIndex);
      }
    });
    return () => sortable.destroy();
  });

  function click(id: number, e: MouseEvent) {
    edit.select(id, e.shiftKey ? 'range' : e.ctrlKey || e.metaKey ? 'toggle' : 'replace');
  }

  function removeOrWarn(ids?: ReadonlySet<number>) {
    if (!edit.remove(ids)) toast('info', 'At least one page must remain.');
  }

  /** Items per grid row, read from the rendered layout (for Up/Down keys). */
  function columns(): number {
    if (!grid) return 1;
    return Math.max(1, getComputedStyle(grid).gridTemplateColumns.split(' ').length);
  }

  async function moveSelection(delta: number) {
    const items = edit.items;
    if (!items.length) return;
    const from = items.findIndex((i) => i.id === edit.anchor);
    const to = Math.min(
      items.length - 1,
      Math.max(0, (from < 0 ? (delta > 0 ? -1 : items.length) : from) + delta)
    );
    const id = items[to].id;
    edit.select(id);
    await tick();
    grid?.querySelector(`[data-id="${id}"]`)?.scrollIntoView({ block: 'nearest' });
  }

  function onkeydown(e: KeyboardEvent) {
    if (preview.id !== null) return; // the preview dialog handles its own keys
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
      void saveAs();
    } else if (ctrl && key === 'z') {
      e.preventDefault();
      edit.undo();
    } else if (ctrl && key === 'y') {
      e.preventDefault();
      edit.redo();
    } else if (ctrl && key === 'a') {
      e.preventDefault();
      edit.selectAll();
    } else if (!ctrl && key === 'r') {
      edit.rotate(e.shiftKey ? -90 : 90);
    } else if (e.key === 'Delete') {
      removeOrWarn();
    } else if ((e.key === 'Enter' || e.key === ' ') && t?.tagName !== 'BUTTON') {
      // Preview the last clicked page. Buttons keep Enter/Space for themselves.
      if (edit.anchor !== null) {
        e.preventDefault();
        preview.open(edit.anchor);
      }
    } else if (e.key === 'Escape') {
      edit.clearSelection();
    } else if (e.key === 'ArrowRight' || e.key === 'ArrowLeft') {
      e.preventDefault();
      void moveSelection(e.key === 'ArrowRight' ? 1 : -1);
    } else if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      void moveSelection(e.key === 'ArrowDown' ? columns() : -columns());
    }
  }

  onMount(() => {
    window.addEventListener('keydown', onkeydown);
    return () => {
      window.removeEventListener('keydown', onkeydown);
      preview.close();
    };
  });

  async function saveAs() {
    if (!doc || saving) return;
    const stem = doc.name.replace(/\.pdf$/i, '');
    const output = await save({
      title: 'Save edited PDF',
      defaultPath: `${folderOf(doc.path)}${stem} (edited).pdf`,
      filters: [{ name: 'PDF', extensions: ['pdf'] }]
    });
    if (!output) return;
    saving = true;
    try {
      const saved = await savePages(doc.path, output, edit.toOps());
      toast('success', `Saved ${fmtSize(saved.size)} to ${output}`, [
        { label: 'Open file', run: () => void openSaved(saved.path).catch(report) },
        { label: 'Show in folder', run: () => void revealSaved(saved.path).catch(report) }
      ]);
    } catch (e) {
      report(e);
    } finally {
      saving = false;
    }
  }

  const report = (e: unknown) => toast('error', errorMessage(e));

  function fmtSize(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
    return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  }

  const plural = (n: number, word: string) => `${n} ${word}${n === 1 ? '' : 's'}`;
</script>

<section class="tool">
  <header>
    <div class="heading">
      <h1>Pages</h1>
      <p>{doc ? doc.name : 'Reorder, rotate, delete or insert pages.'}</p>
    </div>
    <div class="header-actions">
      {#if doc}
        <button
          class="btn"
          title="Undo (Ctrl+Z)"
          disabled={!edit.canUndo}
          onclick={() => edit.undo()}
        >
          <Undo2 size={16} strokeWidth={1.75} />
        </button>
        <button
          class="btn"
          title="Redo (Ctrl+Y)"
          disabled={!edit.canRedo}
          onclick={() => edit.redo()}
        >
          <Redo2 size={16} strokeWidth={1.75} />
        </button>
      {/if}
      <button class="btn" title="Open (Ctrl+O)" onclick={browseForPdf}>
        <FolderOpen size={16} strokeWidth={1.75} /> Open
      </button>
      {#if doc}
        <button class="btn primary" title="Save (Ctrl+S)" disabled={saving} onclick={saveAs}>
          <Save size={16} strokeWidth={1.75} />
          {saving ? 'Saving…' : 'Save as…'}
        </button>
      {/if}
    </div>
  </header>

  {#if doc}
    <div class="toolbar" role="toolbar" aria-label="Page actions">
      <button class="btn" disabled={!edit.selected.size} onclick={() => edit.rotate(-90)}>
        <RotateCcw size={16} strokeWidth={1.75} /> Rotate left
      </button>
      <button class="btn" disabled={!edit.selected.size} onclick={() => edit.rotate(90)}>
        <RotateCw size={16} strokeWidth={1.75} /> Rotate right
      </button>
      <button class="btn" disabled={!edit.selected.size} onclick={() => removeOrWarn()}>
        <Trash2 size={16} strokeWidth={1.75} /> Delete
      </button>
      <span class="sep"></span>
      <button class="btn" onclick={() => edit.insertBlank(blankSize)}>
        <FilePlus size={16} strokeWidth={1.75} /> Insert blank
      </button>
      <select bind:value={blankSize} aria-label="Blank page size">
        <option value="same">Same as previous</option>
        {#each Object.keys(BLANK_SIZES) as name (name)}
          <option value={name}>{name}</option>
        {/each}
      </select>
    </div>

    <div class="scroll">
      <div
        class="grid"
        bind:this={grid}
        role="listbox"
        aria-multiselectable="true"
        aria-label="Pages"
        tabindex="-1"
      >
        {#each edit.items as item, i (item.id)}
          <PageThumb
            {item}
            position={i + 1}
            pdf={doc.pdf}
            docKey={doc.key}
            selected={edit.selected.has(item.id)}
            onselect={(e) => click(item.id, e)}
            onpreview={() => preview.open(item.id)}
            onrotate={(deg) => edit.rotate(deg, new Set([item.id]))}
            ondelete={() => removeOrWarn(new Set([item.id]))}
          />
        {/each}
      </div>
    </div>

    {#if preview.id !== null}
      <Preview
        pdf={doc.pdf}
        items={edit.items}
        id={preview.id}
        onnavigate={(id) => {
          preview.open(id);
          edit.select(id);
        }}
        onclose={() => preview.close()}
      />
    {/if}

    <footer>
      <span>{plural(edit.items.length, 'page')}</span>
      <span>{edit.selected.size} selected</span>
      <span>Source {fmtSize(doc.size)}</span>
      {#if edit.dirty}<span class="unsaved">Unsaved changes</span>{/if}
    </footer>
  {:else}
    <button class="empty" class:hovering={documents.dragging} onclick={browseForPdf}>
      <span class="badge"><FolderOpen size={28} strokeWidth={1.75} /></span>
      <span class="title">
        {documents.loading ? 'Opening…' : 'Drop a PDF here or click to browse'}
      </span>
      <span class="hint">Your original is never changed. Edits are saved to a new file.</span>
    </button>
  {/if}
</section>

<style>
  .tool {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: var(--space-6);
    gap: var(--space-4);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
  }
  .heading {
    min-width: 0;
  }
  h1 {
    margin: 0;
    font-size: var(--text-title);
    font-weight: 600;
  }
  header p {
    margin: var(--space-1) 0 0;
    overflow: hidden;
    color: var(--text-muted);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .header-actions,
  .toolbar {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }
  .toolbar {
    flex-wrap: wrap;
    padding: var(--space-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-card);
    background: var(--surface);
  }
  .sep {
    width: 1px;
    height: 20px;
    margin: 0 var(--space-1);
    background: var(--border);
  }
  .btn {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    height: 32px;
    padding: 0 var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-control);
    background: var(--surface);
    cursor: pointer;
    transition: background var(--duration-fast) var(--ease);
  }
  .btn:hover:not(:disabled) {
    background: var(--surface-2);
  }
  .btn:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .btn.primary {
    border-color: var(--primary);
    background: var(--primary);
    color: var(--on-primary);
    font-weight: 600;
  }
  .btn.primary:hover:not(:disabled) {
    background: var(--primary-hover);
  }
  .btn.primary:active:not(:disabled) {
    background: var(--primary-pressed);
  }
  select {
    height: 32px;
    padding: 0 var(--space-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-control);
    background: var(--surface-2);
    color: var(--text);
    font: inherit;
  }
  .scroll {
    flex: 1;
    min-height: 0;
    overflow: auto;
    border: 1px solid var(--border);
    border-radius: var(--radius-card);
    background: var(--surface);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(184px, 1fr));
    justify-items: center;
    align-items: start;
    gap: var(--space-2);
    padding: var(--space-4);
  }
  /* Sortable's placeholder while dragging; the fallback ghost is its own node. */
  .grid :global(.ghost) {
    opacity: 0.35;
  }
  footer {
    display: flex;
    gap: var(--space-4);
    color: var(--text-muted);
    font-size: var(--text-caption);
  }
  .unsaved {
    color: var(--warning);
    font-weight: 500;
  }
  .empty {
    display: flex;
    flex: 1;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    border: 2px dashed var(--border);
    border-radius: var(--radius-card);
    background: var(--surface);
    cursor: pointer;
    transition:
      border-color var(--duration-base) var(--ease),
      background var(--duration-base) var(--ease);
  }
  .empty:hover,
  .empty.hovering {
    border-color: var(--primary);
    background: var(--selected-bg);
  }
  .badge {
    display: grid;
    place-items: center;
    width: 56px;
    height: 56px;
    border-radius: var(--radius-card);
    background: var(--selected-bg);
    color: var(--primary);
  }
  .title {
    font-size: var(--text-header);
    font-weight: 600;
  }
  .hint {
    color: var(--text-muted);
    font-size: var(--text-caption);
  }
</style>
