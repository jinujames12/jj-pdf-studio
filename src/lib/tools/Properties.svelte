<script lang="ts">
  import { onMount } from 'svelte';
  import { save } from '@tauri-apps/plugin-dialog';
  import Eraser from 'lucide-svelte/icons/eraser';
  import FileText from 'lucide-svelte/icons/file-text';
  import FolderOpen from 'lucide-svelte/icons/folder-open';
  import ToolFrame from '../components/ToolFrame.svelte';
  import { readProperties, saveProperties, type Properties } from '../api';
  import { fmtSize, plural } from '../format';
  import { shortSize } from '../paper';
  import { folderOf, stemOf } from '../paths';
  import { announceSaved, report } from '../saved';
  import { browseForPdf, documents } from '../stores/documents.svelte';

  const doc = $derived(documents.current);

  let props = $state.raw<Properties | null>(null);
  let title = $state('');
  let author = $state('');
  let subject = $state('');
  let keywords = $state('');
  let creator = $state('');
  let removeAll = $state(false);
  let saving = $state(false);

  // Read the file's own values whenever a document is opened.
  let latest = 0;
  $effect(() => {
    const path = doc?.path;
    void doc?.key;
    props = null;
    removeAll = false;
    if (!path) return;
    const ticket = ++latest;
    readProperties(path).then(
      (p) => {
        if (ticket !== latest) return;
        props = p;
        ({ title, author, subject, keywords, creator } = p);
      },
      (e) => ticket === latest && report(e)
    );
  });

  /** "12 pages: 10 × A4, 2 × Letter; 3 turned". */
  const pageSummary = $derived.by(() => {
    if (!props) return '';
    const sizes: Record<string, number> = {};
    let turned = 0;
    for (const p of props.pages) {
      const sideways = p.rotate % 180 !== 0;
      const label = shortSize(sideways ? p.height : p.width, sideways ? p.width : p.height);
      sizes[label] = (sizes[label] ?? 0) + 1;
      if (p.rotate) turned++;
    }
    const entries = Object.entries(sizes);
    const list = entries.map(([label, n]) => (entries.length > 1 ? `${n} × ${label}` : label));
    return `${plural(props.pages.length, 'page')}: ${list.join(', ')}${turned ? `; ${turned} turned` : ''}`;
  });

  const protectionText = $derived.by(() => {
    const info = props?.protection;
    if (!info || info.protection === 'none') return 'Not encrypted';
    if (info.protection === 'restrictions_only')
      return `Restrictions only (${info.algorithm ?? 'encrypted'}): ${info.blocked.join(', ') || 'none'}. A saved copy has no protection.`;
    return `Password protected${info.algorithm ? ` (${info.algorithm})` : ''}`;
  });

  const FIELDS = [
    ['Title', () => title, (v: string) => (title = v)],
    ['Author', () => author, (v: string) => (author = v)],
    ['Subject', () => subject, (v: string) => (subject = v)],
    ['Keywords', () => keywords, (v: string) => (keywords = v)],
    ['Creator', () => creator, (v: string) => (creator = v)]
  ] as const;

  async function saveAs() {
    if (!doc || !props || saving) return;
    const output = await save({
      title: removeAll ? 'Save PDF without metadata' : 'Save PDF with new properties',
      defaultPath: `${folderOf(doc.path)}${stemOf(doc.path)} (${removeAll ? 'clean' : 'edited'}).pdf`,
      filters: [{ name: 'PDF', extensions: ['pdf'] }]
    });
    if (!output) return;
    saving = true;
    try {
      announceSaved(
        await saveProperties(doc.path, output, {
          title,
          author,
          subject,
          keywords,
          creator,
          remove_all: removeAll
        })
      );
    } catch (e) {
      report(e);
    } finally {
      saving = false;
    }
  }

  function onkeydown(e: KeyboardEvent) {
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
    return () => window.removeEventListener('keydown', onkeydown);
  });
</script>

<ToolFrame
  title="Properties"
  subtitle={doc ? doc.name : 'Edit title and author, or remove metadata.'}
  empty={!doc}
  emptyHint="Your original is never changed. The edited copy is saved as a new file."
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
        disabled={!props || saving}
        onclick={saveAs}
      >
        <FileText size={16} strokeWidth={1.75} />
        {saving ? 'Saving…' : 'Save as…'}
      </button>
    {/if}
  {/snippet}

  {#if doc && props}
    <div class="opts-layout">
      <form class="opts" onsubmit={(e) => e.preventDefault()}>
        {#each FIELDS as [label, get, set] (label)}
          <label class="field">
            <span>{label}</span>
            <input
              type="text"
              disabled={removeAll}
              value={removeAll ? '' : get()}
              oninput={(e) => set(e.currentTarget.value)}
            />
          </label>
        {/each}

        <button
          type="button"
          class="btn"
          class:danger={removeAll}
          aria-pressed={removeAll}
          onclick={() => (removeAll = !removeAll)}
        >
          <Eraser size={16} strokeWidth={1.75} />
          {removeAll ? 'Keep metadata' : 'Remove all metadata'}
        </button>
        {#if removeAll}
          <p class="note">
            The saved copy will have no title, author, dates or software names, and no XMP metadata.
            The pages are not changed.
          </p>
        {:else}
          <p class="note">
            Saving also removes the file's XMP metadata, so old values cannot show through.
          </p>
        {/if}
      </form>

      <dl class="facts">
        <dt>Pages</dt>
        <dd>{pageSummary}</dd>
        <dt>PDF version</dt>
        <dd>{props.pdf_version}</dd>
        <dt>File size</dt>
        <dd>{fmtSize(props.size)}</dd>
        <dt>Protection</dt>
        <dd>{protectionText}</dd>
        <dt>Producer</dt>
        <dd>{props.producer || 'Not set'}</dd>
        <dt>Created</dt>
        <dd>{props.created || 'Not set'}</dd>
        <dt>Modified</dt>
        <dd>{props.modified || 'Not set'}</dd>
        <dt>XMP metadata</dt>
        <dd>{props.has_xmp ? 'Present' : 'None'}</dd>
      </dl>
    </div>
  {/if}

  {#snippet footer()}
    {#if props}<span>{plural(props.pages.length, 'page')}</span>{/if}
    {#if doc}<span>Source {fmtSize(doc.size)}</span>{/if}
  {/snippet}
</ToolFrame>

<style>
  .facts {
    display: grid;
    grid-template-columns: max-content minmax(0, 1fr);
    gap: var(--space-2) var(--space-4);
    margin: 0;
    padding: var(--space-4);
    border: 1px solid var(--border);
    border-radius: var(--radius-card);
    background: var(--surface-2);
  }
  dt {
    color: var(--text-muted);
  }
  dd {
    margin: 0;
    overflow-wrap: anywhere;
  }
  .btn.danger {
    border-color: var(--danger);
    color: var(--danger);
  }
</style>
