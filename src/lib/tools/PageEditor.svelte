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
  import Plus from 'lucide-svelte/icons/plus';
  import Scissors from 'lucide-svelte/icons/scissors';
  import FileOutput from 'lucide-svelte/icons/file-output';
  import Hash from 'lucide-svelte/icons/hash';
  import ListChecks from 'lucide-svelte/icons/list-checks';
  import ChevronDown from 'lucide-svelte/icons/chevron-down';
  import ZoomIn from 'lucide-svelte/icons/zoom-in';
  import Save from 'lucide-svelte/icons/save';
  import X from 'lucide-svelte/icons/x';
  import ContextMenu, { type MenuItem } from '../components/ContextMenu.svelte';
  import PageThumb from '../components/PageThumb.svelte';
  import Preview from '../components/Preview.svelte';
  import ExtractOptions from '../pageEditor/ExtractOptions.svelte';
  import PageNumberOptions from '../pageEditor/PageNumberOptions.svelte';
  import RotateOptions from '../pageEditor/RotateOptions.svelte';
  import SplitOptions from '../pageEditor/SplitOptions.svelte';
  import {
    editedName,
    numberLabels,
    rangesToPages,
    splitPartLabel,
    splitTint
  } from '../pageEditor/logic';
  import { parsePageRanges, savePages } from '../api';
  import { fmtSize, plural } from '../format';
  import { folderOf, stemOf } from '../paths';
  import { announceSaved, report } from '../saved';
  import { browseForPdf, documents } from '../stores/documents.svelte';
  import { nav, type PanelId } from '../stores/nav.svelte';
  import { BLANK_SIZES, pageEdit, type BlankSize } from '../stores/pageEdit.svelte';
  import { numberOptions, splitSettings, splitView } from '../stores/panels.svelte';
  import { preview } from '../stores/preview.svelte';
  import { toast } from '../stores/toast.svelte';

  const doc = $derived(documents.current);
  const edit = pageEdit;

  const PANELS: Record<PanelId, string> = {
    split: 'Split',
    extract: 'Extract pages',
    rotate: 'Rotate',
    numbers: 'Page numbers'
  };

  let grid = $state<HTMLElement>();
  let blankSize = $state<BlankSize>('same');
  let saving = $state(false);
  let zoom = $state(160);
  // Bumped after every drag so the grid is rebuilt from the state; Sortable's own
  // DOM moves must never be what the user sees.
  let gridVersion = $state(0);

  let menu = $state<{ x: number; y: number; items: MenuItem[]; label: string } | null>(null);
  let rangePrompt = $state<{ x: number; y: number } | null>(null);
  let rangeText = $state('');
  let rangeError = $state('');
  let selectButton = $state<HTMLElement>();

  const reducedMotion = () => window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  // Drag-to-reorder. Sortable moves DOM nodes itself; we put the node back and
  // apply the move to the state instead, so Svelte stays the only DOM owner.
  // forceFallback uses pointer events, because Tauri's native file-drop handling
  // disables HTML5 drag and drop on Windows.
  $effect(() => {
    if (!grid) return;
    const sortable = Sortable.create(grid, {
      draggable: '.slot',
      filter: '.thumb-btn, .gap-btn',
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
        gridVersion++;
      }
    });
    return () => sortable.destroy();
  });

  // Panels ------------------------------------------------------------------

  const panel = $derived(nav.panel);
  const togglePanel = (p: PanelId) => (nav.panel = nav.panel === p ? null : p);
  const openPanel = (p: PanelId) => (nav.panel = p);

  /** Closes the options panel and puts keyboard focus back on the toolbar button that opened it. */
  async function closePanel() {
    const was = nav.panel;
    nav.panel = null;
    if (!was) return;
    await tick();
    document.querySelector<HTMLElement>(`.toolbar [data-panel="${was}"]`)?.focus();
  }

  // Live preview on the grid --------------------------------------------------

  const ids = $derived(edit.items.map((i) => i.id));
  const splitting = $derived(panel === 'split' ? splitView(ids) : null);

  // Numbers: what the form says while the panel is open, otherwise what is applied.
  const shownNumbering = $derived((panel === 'numbers' ? numberOptions() : null) ?? edit.numbering);
  let numbered = $state.raw<number[] | null>(null);
  let numberRun = 0;
  $effect(() => {
    const o = shownNumbering;
    const count = edit.items.length;
    const run = ++numberRun;
    if (!o?.pages?.trim()) {
      numbered = null;
      return;
    }
    parsePageRanges(o.pages, count)
      .then((r) => {
        if (run === numberRun) numbered = [...new Set(rangesToPages(r))].sort((a, b) => a - b);
      })
      .catch(() => {
        if (run === numberRun) numbered = [];
      });
  });
  const labels = $derived(
    shownNumbering ? numberLabels(shownNumbering, edit.items.length, numbered) : null
  );

  function overlayOf(position: number) {
    const text = labels?.get(position);
    return text !== undefined && shownNumbering
      ? { text, position: shownNumbering.position, color: shownNumbering.color }
      : null;
  }

  function toggleMarker(id: number) {
    // eslint-disable-next-line svelte/prefer-svelte-reactivity -- copied, then replaced
    const next = new Set(splitSettings.markers);
    if (!next.delete(id)) next.add(id);
    splitSettings.markers = next;
  }

  // Selecting and acting ------------------------------------------------------

  function click(id: number, e: MouseEvent) {
    edit.select(id, e.shiftKey ? 'range' : e.ctrlKey || e.metaKey ? 'toggle' : 'replace');
  }

  function removeOrWarn(set?: ReadonlySet<number>) {
    if (!edit.remove(set)) toast('info', 'At least one page must remain.');
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

  function openContext(id: number, e: MouseEvent) {
    e.preventDefault();
    if (!edit.selected.has(id)) edit.select(id);
    const first = edit.items.find((i) => edit.selected.has(i.id));
    const last = edit.items.findLast((i) => edit.selected.has(i.id));
    const none = !edit.selected.size;
    menu = {
      x: e.clientX,
      y: e.clientY,
      label: 'Page actions',
      items: [
        { label: 'Rotate left', shortcut: 'Shift+R', disabled: none, run: () => edit.rotate(-90) },
        { label: 'Rotate right', shortcut: 'R', disabled: none, run: () => edit.rotate(90) },
        { label: 'Rotate 180°', disabled: none, run: () => edit.rotate(180) },
        { label: 'Delete', shortcut: 'Del', disabled: none, separator: true, run: removeOrWarn },
        { label: 'Duplicate', shortcut: 'Ctrl+D', disabled: none, run: () => edit.duplicate() },
        {
          label: 'Move to start',
          disabled: none,
          separator: true,
          run: () => edit.moveToEdge('start')
        },
        { label: 'Move to end', disabled: none, run: () => edit.moveToEdge('end') },
        {
          label: 'Insert blank before',
          separator: true,
          run: () => first && edit.insertBlankNextTo(first.id, 'before', blankSize)
        },
        {
          label: 'Insert blank after',
          run: () => last && edit.insertBlankNextTo(last.id, 'after', blankSize)
        },
        {
          label: 'Split…',
          shortcut: 'Ctrl+Shift+S',
          separator: true,
          run: () => openPanel('split')
        },
        { label: 'Extract selected pages…', shortcut: 'Ctrl+E', run: () => openPanel('extract') },
        { label: 'Page numbers…', shortcut: 'Ctrl+Shift+N', run: () => openPanel('numbers') }
      ]
    };
  }

  function openSelectMenu() {
    const r = selectButton?.getBoundingClientRect();
    menu = {
      x: r?.left ?? 0,
      y: r?.bottom ?? 0,
      label: 'Select pages',
      items: [
        { label: 'All', shortcut: 'Ctrl+A', run: () => edit.selectAll() },
        { label: 'None', run: () => edit.clearSelection() },
        { label: 'Invert', run: () => edit.invertSelection() },
        { label: 'Odd pages', separator: true, run: () => edit.selectWhere((_, n) => n % 2 === 1) },
        { label: 'Even pages', run: () => edit.selectWhere((_, n) => n % 2 === 0) },
        {
          label: 'Range…',
          run: () => {
            rangeText = '';
            rangeError = '';
            rangePrompt = { x: r?.left ?? 0, y: r?.bottom ?? 0 };
          }
        },
        {
          label: 'Landscape pages',
          separator: true,
          run: () => edit.selectOrientation('landscape')
        },
        { label: 'Portrait pages', run: () => edit.selectOrientation('portrait') }
      ]
    };
  }

  async function applyRange() {
    try {
      const r = await parsePageRanges(rangeText, edit.items.length);
      edit.selectPositions(rangesToPages(r));
      rangePrompt = null;
    } catch (e) {
      rangeError = typeof e === 'string' ? e : 'That is not a page range.';
    }
  }

  // Keyboard ------------------------------------------------------------------

  /** Left/Right/Home/End move between the toolbar's controls, as for any ARIA toolbar. */
  function toolbarKeys(e: KeyboardEvent) {
    const t = e.target as HTMLElement;
    // The list and the slider use the arrows themselves.
    if (t.tagName === 'SELECT' || (t instanceof HTMLInputElement && t.type === 'range')) return;
    if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(e.key)) return;
    const controls = [
      ...(e.currentTarget instanceof HTMLElement
        ? e.currentTarget.querySelectorAll<HTMLElement>('button:not(:disabled), select, input')
        : [])
    ];
    const at = controls.indexOf(t);
    if (at < 0) return;
    e.preventDefault();
    e.stopPropagation(); // not the grid's arrow keys
    const to =
      e.key === 'Home'
        ? 0
        : e.key === 'End'
          ? controls.length - 1
          : (at + (e.key === 'ArrowRight' ? 1 : -1) + controls.length) % controls.length;
    controls[to].focus();
  }

  /** The menu key and Shift+F10 open the page menu at the current page. */
  function contextKey() {
    const id = edit.anchor ?? edit.items.find((i) => edit.selected.has(i.id))?.id;
    const el = id === undefined ? null : grid?.querySelector<HTMLElement>(`[data-id="${id}"]`);
    if (!el) return false;
    const r = el.getBoundingClientRect();
    openContext(
      id!,
      new MouseEvent('contextmenu', {
        clientX: r.left + r.width / 2,
        clientY: r.top + r.height / 2
      })
    );
    return true;
  }

  function onkeydown(e: KeyboardEvent) {
    if (preview.id !== null || menu) return; // the preview and menus handle their own keys
    const t = e.target as HTMLElement | null;
    const typing = !!t && /^(INPUT|SELECT|TEXTAREA)$/.test(t.tagName);
    if (typing) {
      // Inside a field only Esc is ours: it closes the options panel.
      if (e.key === 'Escape' && panel && !rangePrompt) {
        void closePanel();
      } else if (e.key === 'Escape' && rangePrompt) {
        rangePrompt = null;
      }
      return;
    }
    const ctrl = e.ctrlKey || e.metaKey;
    const key = e.key.toLowerCase();
    if (ctrl && key === 'o') {
      e.preventDefault();
      void browseForPdf();
    } else if (!doc) {
      return;
    } else if ((e.key === 'ContextMenu' || (e.shiftKey && e.key === 'F10')) && contextKey()) {
      e.preventDefault();
    } else if (ctrl && e.shiftKey && key === 's') {
      e.preventDefault();
      togglePanel('split');
    } else if (ctrl && e.shiftKey && key === 'n') {
      e.preventDefault();
      togglePanel('numbers');
    } else if (ctrl && key === 's') {
      e.preventDefault();
      void saveAs();
    } else if (ctrl && key === 'e') {
      e.preventDefault();
      togglePanel('extract');
    } else if (ctrl && key === 'd') {
      e.preventDefault();
      edit.duplicate();
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
      if (rangePrompt) rangePrompt = null;
      else if (panel) void closePanel();
      else edit.clearSelection();
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
    const output = await save({
      title: 'Save edited PDF',
      defaultPath: `${folderOf(doc.path)}${editedName(stemOf(doc.path))}`,
      filters: [{ name: 'PDF', extensions: ['pdf'] }]
    });
    if (!output) return;
    saving = true;
    try {
      announceSaved(await savePages(doc.path, output, edit.toOps(), edit.numbering));
    } catch (e) {
      report(e);
    } finally {
      saving = false;
    }
  }
</script>

<section class="tool">
  <header>
    <div class="heading">
      <h1>Page Editor</h1>
      <p>
        {#if doc}
          {doc.name} · {plural(edit.items.length, 'page')} · {edit.selected.size} selected
        {:else}
          Reorder, rotate, delete, split, extract and number pages.
        {/if}
      </p>
    </div>
    <div class="header-actions">
      {#if doc}
        <button
          class="btn"
          title="Undo (Ctrl+Z)"
          aria-label="Undo"
          disabled={!edit.canUndo}
          onclick={() => edit.undo()}
        >
          <Undo2 size={16} strokeWidth={1.75} />
        </button>
        <button
          class="btn"
          title="Redo (Ctrl+Y)"
          aria-label="Redo"
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
    <div
      class="toolbar"
      role="toolbar"
      aria-label="Page actions"
      tabindex="-1"
      onkeydown={toolbarKeys}
    >
      <button
        class="btn"
        title="Rotate left (Shift+R)"
        disabled={!edit.selected.size}
        onclick={() => edit.rotate(-90)}
      >
        <RotateCcw size={16} strokeWidth={1.75} /> Rotate left
      </button>
      <button
        class="btn"
        title="Rotate right (R)"
        disabled={!edit.selected.size}
        onclick={() => edit.rotate(90)}
      >
        <RotateCw size={16} strokeWidth={1.75} /> Rotate right
      </button>
      <button
        class="btn"
        title="Delete (Del)"
        disabled={!edit.selected.size}
        onclick={() => removeOrWarn()}
      >
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
      <span class="sep"></span>
      <button
        class="btn"
        aria-pressed={panel === 'rotate'}
        data-panel="rotate"
        title="Rotate all, odd, even or chosen pages"
        onclick={() => togglePanel('rotate')}
      >
        <RotateCw size={16} strokeWidth={1.75} /> Rotate…
      </button>
      <button
        class="btn"
        aria-pressed={panel === 'split'}
        data-panel="split"
        title="Split (Ctrl+Shift+S)"
        onclick={() => togglePanel('split')}
      >
        <Scissors size={16} strokeWidth={1.75} /> Split
      </button>
      <button
        class="btn"
        aria-pressed={panel === 'extract'}
        data-panel="extract"
        title="Extract (Ctrl+E)"
        onclick={() => togglePanel('extract')}
      >
        <FileOutput size={16} strokeWidth={1.75} /> Extract
      </button>
      <button
        class="btn"
        aria-pressed={panel === 'numbers'}
        data-panel="numbers"
        title="Page numbers (Ctrl+Shift+N)"
        onclick={() => togglePanel('numbers')}
      >
        <Hash size={16} strokeWidth={1.75} /> Page numbers
      </button>
      <span class="sep"></span>
      <button
        class="btn"
        bind:this={selectButton}
        aria-haspopup="menu"
        aria-expanded={!!menu && menu.label === 'Select pages'}
        onclick={openSelectMenu}
      >
        <ListChecks size={16} strokeWidth={1.75} /> Select <ChevronDown
          size={14}
          strokeWidth={1.75}
        />
      </button>
      <label class="zoom">
        <ZoomIn size={16} strokeWidth={1.75} />
        <input type="range" min="100" max="240" step="10" bind:value={zoom} aria-label="Zoom" />
      </label>
    </div>

    <div class="work">
      <div class="scroll">
        <div
          class="grid"
          style:--cell-w="{zoom + 24}px"
          bind:this={grid}
          role="listbox"
          aria-multiselectable="true"
          aria-label="Pages"
          tabindex="0"
        >
          {#key gridVersion}
            {#each edit.items as item, i (item.id)}
              {@const position = i + 1}
              {@const partLabel = splitting ? splitPartLabel(splitting, position) : null}
              <div class="slot" data-id={item.id}>
                <PageThumb
                  {item}
                  {position}
                  pdf={doc.pdf}
                  docKey={doc.key}
                  width={zoom}
                  selected={edit.selected.has(item.id)}
                  current={edit.anchor === item.id}
                  tint={splitting ? splitTint(splitting, position) : null}
                  label={partLabel ? `${position} · ${partLabel}` : undefined}
                  overlay={overlayOf(position)}
                  onselect={(e) => click(item.id, e)}
                  oncontext={(e) => openContext(item.id, e)}
                  onpreview={() => preview.open(item.id)}
                  onrotate={(deg) => edit.rotate(deg, new Set([item.id]))}
                  ondelete={() => removeOrWarn(new Set([item.id]))}
                />
                {#if splitting?.kind === 'at'}
                  {#if i < edit.items.length - 1}
                    <button
                      class="gap-btn cut"
                      class:on={splitSettings.markers.has(item.id)}
                      aria-pressed={splitSettings.markers.has(item.id)}
                      title="Split after page {position}"
                      aria-label="Split after page {position}"
                      onclick={() => toggleMarker(item.id)}
                    >
                      <Scissors size={12} strokeWidth={1.75} />
                    </button>
                  {/if}
                {:else}
                  <button
                    class="gap-btn add"
                    title="Insert a blank page after page {position}"
                    aria-label="Insert a blank page after page {position}"
                    onclick={() => edit.insertBlankNextTo(item.id, 'after', blankSize)}
                  >
                    <Plus size={12} strokeWidth={2} />
                  </button>
                {/if}
              </div>
            {/each}
          {/key}
        </div>
      </div>

      {#if panel}
        <aside class="panel" aria-label="{PANELS[panel]} options">
          <div class="panel-head">
            <h2>{PANELS[panel]}</h2>
            <button
              class="btn icon"
              aria-label="Close panel"
              title="Close (Esc)"
              onclick={closePanel}
            >
              <X size={16} strokeWidth={1.75} />
            </button>
          </div>
          <div class="panel-body">
            {#if panel === 'split'}
              <SplitOptions />
            {:else if panel === 'extract'}
              <ExtractOptions />
            {:else if panel === 'rotate'}
              <RotateOptions />
            {:else}
              <PageNumberOptions />
            {/if}
          </div>
        </aside>
      {/if}
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

    {#if menu}
      <ContextMenu
        x={menu.x}
        y={menu.y}
        items={menu.items}
        label={menu.label}
        onclose={() => (menu = null)}
      />
    {/if}

    {#if rangePrompt}
      <form
        class="range-prompt"
        style:left="{rangePrompt.x}px"
        style:top="{rangePrompt.y + 4}px"
        onsubmit={(e) => {
          e.preventDefault();
          void applyRange();
        }}
      >
        <label>
          Select pages
          <!-- svelte-ignore a11y_autofocus -->
          <input
            type="text"
            placeholder="2, 5-7"
            class:invalid={!!rangeError}
            autofocus
            bind:value={rangeText}
          />
        </label>
        <button class="btn primary" type="submit">Select</button>
        <button class="btn" type="button" onclick={() => (rangePrompt = null)}>Cancel</button>
        {#if rangeError}<span class="field-error" role="alert">{rangeError}</span>{/if}
      </form>
    {/if}

    <footer>
      <span>{plural(edit.items.length, 'page')}</span>
      <span>{edit.selected.size} selected</span>
      <span>Source {fmtSize(doc.size)}</span>
      {#if edit.numbering}<span>Page numbers will be added on save</span>{/if}
      {#if edit.dirty}<span class="unsaved">Unsaved changes</span>{/if}
      {#if panel === 'split' || panel === 'extract'}
        <span>Split and Extract use the pages as shown, including unsaved edits.</span>
      {/if}
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
  .btn[aria-pressed='true'] {
    border-color: var(--primary);
    background: var(--selected-bg);
    color: var(--primary);
  }
  .btn.icon {
    width: 32px;
    padding: 0;
    justify-content: center;
  }
  .zoom {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    margin-left: auto;
    color: var(--text-muted);
  }
  .zoom input {
    width: 110px;
    accent-color: var(--primary);
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
  .work {
    display: flex;
    flex: 1;
    gap: var(--space-4);
    min-height: 0;
  }
  .scroll {
    flex: 1;
    min-width: 0;
    overflow: auto;
    border: 1px solid var(--border);
    border-radius: var(--radius-card);
    background: var(--surface);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(var(--cell-w, 184px), 1fr));
    justify-items: center;
    align-items: start;
    gap: var(--space-2);
    padding: var(--space-4);
  }
  .slot {
    position: relative;
  }
  /* Sortable's placeholder while dragging; the fallback ghost is its own node. */
  .grid :global(.ghost) {
    opacity: 0.35;
  }
  /* The gap between a page and the next one: a thin strip that shows "+" (insert a
     blank page) or the scissors (split marker) when pointed at. */
  .gap-btn {
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
  .gap-btn:hover,
  .gap-btn:focus-visible {
    background: var(--selected-bg);
    color: var(--primary);
  }
  .gap-btn.cut.on {
    background: var(--primary);
    color: var(--on-primary);
  }
  .panel {
    display: flex;
    flex: none;
    flex-direction: column;
    width: 320px;
    min-height: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-card);
    background: var(--surface);
  }
  .panel-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-2) var(--space-3);
    border-bottom: 1px solid var(--border);
  }
  .panel-head h2 {
    margin: 0;
    font-size: var(--text-header);
    font-weight: 600;
  }
  .panel-body {
    flex: 1;
    min-height: 0;
    padding: var(--space-3);
    overflow: auto;
  }
  .range-prompt {
    position: fixed;
    z-index: 91;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-card);
    background: var(--surface);
    box-shadow: var(--shadow-card);
  }
  .range-prompt label {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
  }
  .range-prompt input {
    height: 32px;
    width: 140px;
    padding: 0 var(--space-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-control);
    background: var(--surface-2);
    color: var(--text);
    font: inherit;
  }
  .range-prompt input.invalid {
    border-color: var(--danger);
  }
  footer {
    display: flex;
    flex-wrap: wrap;
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
