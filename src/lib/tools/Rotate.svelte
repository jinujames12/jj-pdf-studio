<script lang="ts">
  import { onMount } from 'svelte';
  import { save } from '@tauri-apps/plugin-dialog';
  import FolderOpen from 'lucide-svelte/icons/folder-open';
  import RotateCcw from 'lucide-svelte/icons/rotate-ccw';
  import RotateCw from 'lucide-svelte/icons/rotate-cw';
  import Save from 'lucide-svelte/icons/save';
  import PageThumb from '../components/PageThumb.svelte';
  import Preview from '../components/Preview.svelte';
  import ToolFrame from '../components/ToolFrame.svelte';
  import {
    planRotation,
    savePages,
    type PageOp,
    type RotateAction,
    type RotateTarget
  } from '../api';
  import { fmtSize, plural } from '../format';
  import { folderOf, stemOf } from '../paths';
  import { announceSaved, report } from '../saved';
  import { browseForPdf, documents } from '../stores/documents.svelte';
  import type { PageItem } from '../stores/pageEdit.svelte';
  import { preview } from '../stores/preview.svelte';

  type TargetKind = 'all' | 'odd' | 'even' | 'pages';

  const doc = $derived(documents.current);
  const count = $derived(doc?.info.pages.length ?? 0);

  /** Extra turn per page, on top of the page's own rotation. */
  let turns = $state.raw<number[]>([]);
  let targetKind = $state<TargetKind>('all');
  let pagesText = $state('');
  let saving = $state(false);

  const dirty = $derived(turns.some((t) => t !== 0));
  const items = $derived<PageItem[]>(
    Array.from({ length: count }, (_, i) => ({
      id: i + 1,
      kind: 'page',
      src: i + 1,
      rotate: turns[i] ?? 0
    }))
  );

  // A newly opened document starts unrotated.
  let seenKey = '';
  $effect(() => {
    const key = doc?.key ?? '';
    if (key === seenKey) return;
    seenKey = key;
    turns = Array.from({ length: count }, () => 0);
    pagesText = '';
  });

  // Opening another file asks before throwing away turns that were not saved.
  $effect(() => {
    documents.toolDirty = dirty;
    return () => {
      documents.toolDirty = false;
    };
  });

  function target(): RotateTarget {
    return targetKind === 'pages' ? { kind: 'pages', text: pagesText } : { kind: targetKind };
  }

  /** Rust decides which pages an action touches; the grid just shows the result. */
  async function apply(action: RotateAction) {
    if (!doc) return;
    try {
      turns = await planRotation(doc.path, turns, action);
    } catch (e) {
      report(e);
    }
  }

  const turn = (degrees: number) => apply({ kind: 'turn', target: target(), degrees });

  async function saveAs() {
    if (!doc || saving) return;
    const output = await save({
      title: 'Save rotated PDF',
      defaultPath: `${folderOf(doc.path)}${stemOf(doc.path)} (rotated).pdf`,
      filters: [{ name: 'PDF', extensions: ['pdf'] }]
    });
    if (!output) return;
    saving = true;
    try {
      const ops: PageOp[] = turns.map((rotate, i) => ({ kind: 'page', page: i + 1, rotate }));
      announceSaved(await savePages(doc.path, output, ops));
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
    } else if (key === 's' && doc) {
      e.preventDefault();
      void saveAs();
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
  title="Rotate"
  subtitle={doc ? doc.name : 'Turn all or some pages, losslessly.'}
  empty={!doc}
  emptyHint="Your original is never changed. The rotated copy is saved as a new file."
  onopen={browseForPdf}
>
  {#snippet actions()}
    <button class="btn" title="Open (Ctrl+O)" onclick={browseForPdf}>
      <FolderOpen size={16} strokeWidth={1.75} /> Open
    </button>
    {#if doc}
      <button
        class="btn primary"
        title="Save (Ctrl+S)"
        disabled={!dirty || saving}
        onclick={saveAs}
      >
        <Save size={16} strokeWidth={1.75} />
        {saving ? 'Saving…' : 'Save as…'}
      </button>
    {/if}
  {/snippet}

  {#snippet toolbar()}
    <label>
      Pages
      <select bind:value={targetKind}>
        <option value="all">All pages</option>
        <option value="odd">Odd pages</option>
        <option value="even">Even pages</option>
        <option value="pages">Pages…</option>
      </select>
    </label>
    {#if targetKind === 'pages'}
      <input
        type="text"
        size="18"
        placeholder="2, 5-7"
        aria-label="Pages to rotate"
        bind:value={pagesText}
      />
    {/if}
    <button class="btn" onclick={() => turn(90)}>
      <RotateCw size={16} strokeWidth={1.75} /> 90° right
    </button>
    <button class="btn" onclick={() => turn(-90)}>
      <RotateCcw size={16} strokeWidth={1.75} /> 90° left
    </button>
    <button class="btn" onclick={() => turn(180)}>180°</button>
    <span class="sep"></span>
    <button class="btn" onclick={() => apply({ kind: 'orient', orientation: 'portrait' })}>
      Make all portrait
    </button>
    <button class="btn" onclick={() => apply({ kind: 'orient', orientation: 'landscape' })}>
      Make all landscape
    </button>
    <button class="btn" disabled={!dirty} onclick={() => (turns = turns.map(() => 0))}>
      Reset
    </button>
  {/snippet}

  <div class="page-grid" role="list" aria-label="Pages">
    {#each items as item (item.id)}
      <PageThumb
        {item}
        position={item.src}
        pdf={doc!.pdf}
        docKey={doc!.key}
        readonly
        tint={item.rotate ? 'a' : null}
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
    <span>{plural(turns.filter((t) => t !== 0).length, 'page')} turned</span>
    {#if doc}<span>Source {fmtSize(doc.size)}</span>{/if}
    {#if dirty}<span class="unsaved">Unsaved changes</span>{/if}
  {/snippet}
</ToolFrame>

<style>
  .sep {
    width: 1px;
    height: 20px;
    background: var(--border);
  }
  .unsaved {
    color: var(--warning);
    font-weight: 500;
  }
</style>
