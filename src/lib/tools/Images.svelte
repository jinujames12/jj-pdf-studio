<script lang="ts">
  import { onMount } from 'svelte';
  import { confirm, open as openDialog, save } from '@tauri-apps/plugin-dialog';
  import ArrowDownAZ from 'lucide-svelte/icons/arrow-down-a-z';
  import CalendarClock from 'lucide-svelte/icons/calendar-clock';
  import FileOutput from 'lucide-svelte/icons/file-output';
  import ImagePlus from 'lucide-svelte/icons/image-plus';
  import ImageCard from '../components/ImageCard.svelte';
  import PaperPicker from '../components/PaperPicker.svelte';
  import ToolFrame from '../components/ToolFrame.svelte';
  import { imagesToFolder, imagesToPdf, type ImageOptions } from '../api';
  import { fmtSize, plural } from '../format';
  import { folderOf, IMAGE_EXTENSIONS } from '../paths';
  import { defaultPaper, toPaper, validCustom, type PaperChoice } from '../paper';
  import { sortable } from '../actions/sortable';
  import { announceFiles, announceSaved, report } from '../saved';
  import { images } from '../stores/images.svelte';

  const MANY_FILES = 100;

  let paper = $state<PaperChoice>(defaultPaper('a4'));
  let orientation = $state<ImageOptions['orientation']>('auto');
  let marginKind = $state<'none' | 'small' | 'large' | 'custom'>('none');
  let customMargin = $state(5);
  let fit = $state<ImageOptions['fit']>('inside');
  let quality = $state<ImageOptions['quality']>('keep');
  let output = $state<'one' | 'each'>('one');
  let saving = $state(false);
  let listVersion = $state(0);

  const items = $derived(images.items);
  const fitToImage = $derived(paper.kind === 'fit');
  const marginMm = $derived(
    marginKind === 'none'
      ? 0
      : marginKind === 'small'
        ? 10
        : marginKind === 'large'
          ? 20
          : customMargin
  );
  const marginOk = $derived(Number.isFinite(marginMm) && marginMm >= 0 && marginMm <= 100);
  const paperOk = $derived(paper.kind !== 'custom' || validCustom(paper));
  const ready = $derived(items.length > 0 && marginOk && paperOk && !saving && !images.loading);

  function options(): ImageOptions {
    return { page: toPaper(paper), orientation, margin_mm: marginMm, fit, quality };
  }

  async function addFiles() {
    const picked = await openDialog({
      multiple: true,
      title: 'Add images',
      filters: [{ name: 'Images', extensions: IMAGE_EXTENSIONS }]
    });
    if (picked?.length) await images.addPaths(picked);
  }

  async function create() {
    if (!ready) return;
    const first = items[0];
    saving = true;
    try {
      if (output === 'one') {
        const target = await save({
          title: 'Save PDF',
          defaultPath: `${folderOf(first.path)}${first.stem}${items.length > 1 ? ' (images)' : ''}.pdf`,
          filters: [{ name: 'PDF', extensions: ['pdf'] }]
        });
        if (!target) return;
        announceSaved(await imagesToPdf(images.toInputs(), target, options()));
      } else {
        if (items.length > MANY_FILES) {
          const go = await confirm(`This will create ${items.length} files. Continue?`, {
            title: 'Many files',
            kind: 'warning',
            okLabel: `Create ${items.length} files`,
            cancelLabel: 'Cancel'
          });
          if (!go) return;
        }
        const folder = await openDialog({
          directory: true,
          title: 'Choose a folder for the PDFs',
          defaultPath: folderOf(first.path)
        });
        if (!folder) return;
        announceFiles(await imagesToFolder(images.toInputs(), folder, options()));
      }
    } catch (e) {
      report(e);
    } finally {
      saving = false;
    }
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
      void create();
    }
  }

  onMount(() => {
    window.addEventListener('keydown', onkeydown);
    return () => window.removeEventListener('keydown', onkeydown);
  });
</script>

<ToolFrame
  title="Images to PDF"
  subtitle={items.length
    ? `${plural(items.length, 'image')} · ${fmtSize(images.totalSize)}`
    : 'Turn JPG and PNG images into a PDF.'}
  empty={items.length === 0}
  emptyTitle={images.loading ? 'Opening…' : 'Drop images here or click to browse'}
  emptyHint="JPG, PNG, BMP, GIF, TIFF and WebP. JPEGs are embedded without losing quality."
  onopen={addFiles}
>
  {#snippet actions()}
    <button class="btn" title="Add images (Ctrl+O)" onclick={addFiles}>
      <ImagePlus size={16} strokeWidth={1.75} />
      {images.loading ? 'Adding…' : 'Add images'}
    </button>
    {#if items.length}
      <button class="btn primary" title="Save (Ctrl+S)" disabled={!ready} onclick={create}>
        <FileOutput size={16} strokeWidth={1.75} />
        {saving ? 'Creating…' : output === 'one' ? 'Create PDF' : 'Create PDFs'}
      </button>
    {/if}
  {/snippet}

  {#snippet toolbar()}
    <PaperPicker bind:choice={paper} allowFit />
    <label>
      Orientation
      <select bind:value={orientation} disabled={fitToImage}>
        <option value="auto">Auto (per image)</option>
        <option value="portrait">Portrait</option>
        <option value="landscape">Landscape</option>
      </select>
    </label>
    <label>
      Margin
      <select bind:value={marginKind}>
        <option value="none">None</option>
        <option value="small">Small (10 mm)</option>
        <option value="large">Large (20 mm)</option>
        <option value="custom">Custom…</option>
      </select>
    </label>
    {#if marginKind === 'custom'}
      <input
        type="number"
        min="0"
        max="100"
        step="any"
        class:invalid={!marginOk}
        aria-label="Margin in millimetres"
        bind:value={customMargin}
      />
      mm
    {/if}
    <label>
      Fit
      <select bind:value={fit} disabled={fitToImage}>
        <option value="inside">Fit inside</option>
        <option value="fill">Fill page (crop)</option>
        <option value="original">Original size</option>
      </select>
    </label>
    <label>
      Quality
      <select bind:value={quality}>
        <option value="keep">Keep original</option>
        <option value="reduce">Reduce size</option>
      </select>
    </label>
    <label>
      Output
      <select bind:value={output}>
        <option value="one">One PDF</option>
        <option value="each">One PDF per image</option>
      </select>
    </label>
    <span class="sep"></span>
    <button class="btn" title="Sort by file name" onclick={() => images.sortBy('name')}>
      <ArrowDownAZ size={16} strokeWidth={1.75} /> Name
    </button>
    <button class="btn" title="Sort by date modified" onclick={() => images.sortBy('date')}>
      <CalendarClock size={16} strokeWidth={1.75} /> Date
    </button>
    <button class="btn" onclick={() => images.clear()}>Clear all</button>
  {/snippet}

  <div
    class="page-grid"
    role="list"
    aria-label="Images to convert"
    use:sortable={{
      draggable: '.image-card',
      onmove: (id, to) => images.move(id, to),
      onend: () => listVersion++
    }}
  >
    {#key listVersion}
      {#each items as item, i (item.id)}
        <ImageCard
          {item}
          position={i + 1}
          onrotate={(deg) => images.rotate(item.id, deg)}
          onremove={() => images.remove(item.id)}
        />
      {/each}
    {/key}
  </div>

  {#snippet footer()}
    <span>{plural(items.length, 'image')}</span>
    <span>Sources {fmtSize(images.totalSize)}</span>
    <span>Drag images to reorder. Hover one to turn or remove it.</span>
    {#if fitToImage}
      <span>Fit to image: each page takes its image's size (96 DPI if the image has none).</span>
    {/if}
  {/snippet}
</ToolFrame>

<style>
  .sep {
    width: 1px;
    height: 20px;
    background: var(--border);
  }
</style>
