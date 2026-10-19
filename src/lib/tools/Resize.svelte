<script lang="ts">
  import { onMount } from 'svelte';
  import { save } from '@tauri-apps/plugin-dialog';
  import FolderOpen from 'lucide-svelte/icons/folder-open';
  import Save from 'lucide-svelte/icons/save';
  import PageThumb from '../components/PageThumb.svelte';
  import PaperPicker from '../components/PaperPicker.svelte';
  import Preview from '../components/Preview.svelte';
  import ToolFrame from '../components/ToolFrame.svelte';
  import {
    errorMessage,
    planResize,
    resizePdf,
    type ResizeOptions,
    type RotateTarget
  } from '../api';
  import { plural, fmtSize } from '../format';
  import { folderOf, stemOf } from '../paths';
  import { defaultPaper, shortSize, toPaper, validCustom, type PaperChoice } from '../paper';
  import { announceSaved, report } from '../saved';
  import { browseForPdf, documents } from '../stores/documents.svelte';
  import type { PageItem } from '../stores/pageEdit.svelte';
  import { preview } from '../stores/preview.svelte';

  type TargetKind = 'all' | 'odd' | 'even' | 'pages';

  const doc = $derived(documents.current);
  const count = $derived(doc?.info.pages.length ?? 0);

  let paper = $state<PaperChoice>(defaultPaper('a4'));
  let orientation = $state<ResizeOptions['orientation']>('auto');
  let mode = $state<ResizeOptions['mode']>('fit');
  let anchor = $state<ResizeOptions['anchor']>('center');
  let margin = $state(0);
  let targetKind = $state<TargetKind>('all');
  let pagesText = $state('');
  let saving = $state(false);

  /** New size per page in points, `null` for pages left alone; `undefined` while unknown. */
  let plan = $state.raw<Array<[number, number] | null> | undefined>(undefined);
  let planError = $state('');

  const marginOk = $derived(Number.isFinite(margin) && margin >= 0 && margin <= 20);
  const paperOk = $derived(paper.kind !== 'custom' || validCustom(paper));
  const ready = $derived(!!plan && !planError && marginOk && paperOk && !saving);
  const changed = $derived(plan?.filter((p) => p !== null).length ?? 0);

  function target(): RotateTarget {
    return targetKind === 'pages' ? { kind: 'pages', text: pagesText } : { kind: targetKind };
  }

  function options(): ResizeOptions {
    return {
      paper: toPaper(paper)!,
      orientation,
      mode,
      anchor,
      margin_mm: margin,
      target: target()
    };
  }

  // Rust validates the options and says which pages change and to what size.
  // Only the newest answer is used, so typing quickly never shows a stale plan.
  let latest = 0;
  $effect(() => {
    const source = doc?.path;
    if (!source || !marginOk || !paperOk) {
      plan = undefined;
      planError = '';
      return;
    }
    const opts = options();
    const ticket = ++latest;
    const timer = setTimeout(() => {
      planResize(source, opts).then(
        (result) => {
          if (ticket !== latest) return;
          plan = result;
          planError = '';
        },
        (e) => {
          if (ticket !== latest) return;
          plan = undefined;
          planError = errorMessage(e);
        }
      );
    }, 150);
    return () => clearTimeout(timer);
  });

  const items = $derived<PageItem[]>(
    Array.from({ length: count }, (_, i) => ({ id: i + 1, kind: 'page', src: i + 1, rotate: 0 }))
  );

  /** "A4 → Letter" style caption under each page. */
  function caption(i: number): string {
    const page = doc?.info.pages[i];
    if (!page) return `${i + 1}`;
    const sideways = page.rotate % 180 !== 0;
    const from = shortSize(
      sideways ? page.height : page.width,
      sideways ? page.width : page.height
    );
    const to = plan?.[i];
    return to ? `${i + 1}: ${from} → ${shortSize(to[0], to[1])}` : `${i + 1}: ${from}`;
  }

  async function saveAs() {
    if (!doc || !ready) return;
    const output = await save({
      title: 'Save resized PDF',
      defaultPath: `${folderOf(doc.path)}${stemOf(doc.path)} (resized).pdf`,
      filters: [{ name: 'PDF', extensions: ['pdf'] }]
    });
    if (!output) return;
    saving = true;
    try {
      announceSaved(await resizePdf(doc.path, output, options()));
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
  title="Resize"
  subtitle={doc ? doc.name : 'Change page size to A4, Letter and more.'}
  empty={!doc}
  emptyHint="Your original is never changed. The resized copy is saved as a new file."
  onopen={browseForPdf}
>
  {#snippet actions()}
    <button class="btn" title="Open (Ctrl+O)" onclick={browseForPdf}>
      <FolderOpen size={16} strokeWidth={1.75} /> Open
    </button>
    {#if doc}
      <button class="btn primary" title="Save (Ctrl+S)" disabled={!ready} onclick={saveAs}>
        <Save size={16} strokeWidth={1.75} />
        {saving ? 'Saving…' : 'Save as…'}
      </button>
    {/if}
  {/snippet}

  {#snippet toolbar()}
    <PaperPicker bind:choice={paper} label="Resize to" />
    <label>
      Orientation
      <select bind:value={orientation}>
        <option value="auto">Auto (match each page)</option>
        <option value="portrait">Portrait</option>
        <option value="landscape">Landscape</option>
      </select>
    </label>
    <label>
      Mode
      <select bind:value={mode}>
        <option value="fit">Fit (scale to fit inside)</option>
        <option value="fill">Fill (scale to cover, crop)</option>
        <option value="size">Change page size only</option>
      </select>
    </label>
    {#if mode === 'size'}
      <label>
        Content
        <select bind:value={anchor}>
          <option value="center">Centred</option>
          <option value="topleft">Top left</option>
        </select>
      </label>
    {/if}
    <label>
      Margin
      <input
        type="number"
        min="0"
        max="20"
        step="1"
        class:invalid={!marginOk}
        bind:value={margin}
      />
      mm
    </label>
    <label>
      Apply to
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
        class:invalid={!!planError}
        aria-label="Pages to resize"
        bind:value={pagesText}
      />
    {/if}
    {#if planError}<span class="field-error">{planError}</span>{/if}
  {/snippet}

  <div class="page-grid" role="list" aria-label="Pages">
    {#each items as item, i (item.id)}
      <PageThumb
        {item}
        position={item.src}
        pdf={doc!.pdf}
        docKey={doc!.key}
        readonly
        label={caption(i)}
        tint={plan?.[i] ? 'a' : null}
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
    <span>{plan ? `${plural(changed, 'page')} will be resized` : ''}</span>
    {#if doc}<span>Source {fmtSize(doc.size)}</span>{/if}
    {#if mode === 'fit' || mode === 'fill'}
      <span>Links and comments move with the page. Form fields may misalign after scaling.</span>
    {/if}
  {/snippet}
</ToolFrame>
