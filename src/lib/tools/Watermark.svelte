<script lang="ts">
  import { onMount } from 'svelte';
  import { open as openDialog, save } from '@tauri-apps/plugin-dialog';
  import FolderOpen from 'lucide-svelte/icons/folder-open';
  import ImagePlus from 'lucide-svelte/icons/image-plus';
  import Stamp from 'lucide-svelte/icons/stamp';
  import ToolFrame from '../components/ToolFrame.svelte';
  import StampPreview from '../components/StampPreview.svelte';
  import { openImage, stampPdf, type Stamping, type WatermarkOptions } from '../api';
  import { fmtSize, plural } from '../format';
  import { fileName, folderOf, IMAGE_EXTENSIONS, stemOf } from '../paths';
  import { announceSaved, report } from '../saved';
  import { browseForPdf, documents } from '../stores/documents.svelte';

  type Place = WatermarkOptions['place'];

  const doc = $derived(documents.current);
  const count = $derived(doc?.info.pages.length ?? 0);

  let mode = $state<'text' | 'image'>('text');
  let text = $state('CONFIDENTIAL');
  let fontSize = $state(60);
  let color = $state('#808080');
  let imagePath = $state('');
  let imageInfo = $state('');
  let widthPercent = $state(30);
  let opacity = $state(30);
  let rotation = $state(45);
  let place = $state<Place>('center');
  let layer = $state<'over' | 'under'>('over');
  let margin = $state(10);
  let range = $state<'all' | 'pages'>('all');
  let pagesText = $state('');
  let saving = $state(false);
  let problem = $state('');

  const PLACES: Array<[Place, string]> = [
    ['center', 'Centre'],
    ['tiled', 'Tiled across the page'],
    ['top-left', 'Top left'],
    ['top-right', 'Top right'],
    ['bottom-left', 'Bottom left'],
    ['bottom-right', 'Bottom right']
  ];

  // Text and logos have different usual looks, so switching resets those.
  function chooseMode(next: 'text' | 'image') {
    if (next === mode) return;
    mode = next;
    rotation = next === 'text' ? 45 : 0;
    opacity = next === 'text' ? 30 : 60;
    place = next === 'text' ? 'center' : 'bottom-right';
  }

  async function pickImage() {
    const picked = await openDialog({
      multiple: false,
      title: 'Choose a logo or stamp',
      filters: [{ name: 'Image', extensions: IMAGE_EXTENSIONS }]
    });
    if (!picked) return;
    try {
      const info = await openImage(picked);
      imagePath = picked;
      imageInfo = `${fileName(picked)} · ${info.width} × ${info.height} px`;
    } catch (e) {
      report(e);
    }
  }

  const sizeOk = $derived(Number.isFinite(fontSize) && fontSize >= 1 && fontSize <= 500);
  const widthOk = $derived(
    Number.isFinite(widthPercent) && widthPercent >= 1 && widthPercent <= 100
  );
  const marginOk = $derived(Number.isFinite(margin) && margin >= 0 && margin <= 100);
  const angleOk = $derived(Number.isFinite(rotation));
  const complete = $derived(
    marginOk &&
      angleOk &&
      (range === 'all' || !!pagesText.trim()) &&
      (mode === 'text' ? sizeOk && text.trim() !== '' : widthOk && imagePath !== '')
  );

  const stamping = $derived<Stamping | null>(
    complete
      ? ({
          tool: 'watermark',
          ...(mode === 'text'
            ? { kind: 'text', text, font_size: fontSize, color }
            : { kind: 'image', path: imagePath, width_percent: widthPercent }),
          opacity: opacity / 100,
          rotation,
          place,
          layer,
          margin_mm: margin,
          pages: range === 'pages' ? pagesText : null
        } as Stamping)
      : null
  );
  const ready = $derived(!!doc && !!stamping && !problem && !saving);

  async function saveAs() {
    if (!doc || !stamping || !ready) return;
    const output = await save({
      title: 'Save watermarked PDF',
      defaultPath: `${folderOf(doc.path)}${stemOf(doc.path)} (watermarked).pdf`,
      filters: [{ name: 'PDF', extensions: ['pdf'] }]
    });
    if (!output) return;
    saving = true;
    try {
      announceSaved(await stampPdf(doc.path, output, stamping));
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
  title="Watermark"
  subtitle={doc ? doc.name : 'Stamp text or a logo on pages.'}
  empty={!doc}
  emptyHint="Your original is never changed. The watermarked copy is saved as a new file."
  onopen={browseForPdf}
>
  {#snippet actions()}
    <button class="btn" title="Open (Ctrl+O)" onclick={browseForPdf}>
      <FolderOpen size={16} strokeWidth={1.75} /> Open
    </button>
    {#if doc}
      <button class="btn primary" title="Save (Ctrl+S)" disabled={!ready} onclick={saveAs}>
        <Stamp size={16} strokeWidth={1.75} />
        {saving ? 'Saving…' : 'Save as…'}
      </button>
    {/if}
  {/snippet}

  {#if doc}
    <div class="opts-layout">
      <form class="opts" onsubmit={(e) => e.preventDefault()}>
        <div class="segmented" role="group" aria-label="Kind of watermark">
          <button type="button" aria-pressed={mode === 'text'} onclick={() => chooseMode('text')}>
            Text
          </button>
          <button type="button" aria-pressed={mode === 'image'} onclick={() => chooseMode('image')}>
            Image
          </button>
        </div>

        {#if mode === 'text'}
          <label class="field">
            <span>Text</span>
            <input type="text" bind:value={text} class:invalid={!text.trim()} />
          </label>
          <div class="row">
            <label class="field">
              <span>Font size (pt)</span>
              <input
                type="number"
                min="1"
                max="500"
                bind:value={fontSize}
                class:invalid={!sizeOk}
              />
            </label>
            <label class="field">
              <span>Colour</span>
              <input type="color" bind:value={color} />
            </label>
          </div>
        {:else}
          <div class="field">
            <span>Picture (PNG or JPG; transparency is kept)</span>
            <div class="row">
              <button type="button" class="btn" onclick={pickImage}>
                <ImagePlus size={16} strokeWidth={1.75} />
                {imagePath ? 'Change…' : 'Choose…'}
              </button>
              <span class="note">{imageInfo || 'No picture chosen yet.'}</span>
            </div>
          </div>
          <label class="field">
            <span>Width: {widthPercent}% of the page width</span>
            <input type="range" min="5" max="100" step="1" bind:value={widthPercent} />
          </label>
        {/if}

        <label class="field">
          <span>Opacity: {opacity}%</span>
          <input type="range" min="5" max="100" step="5" bind:value={opacity} />
        </label>

        <div class="row">
          <label class="field">
            <span>Rotation (degrees)</span>
            <input type="number" step="5" bind:value={rotation} class:invalid={!angleOk} />
          </label>
          <label class="field">
            <span>Position</span>
            <select bind:value={place}>
              {#each PLACES as [value, label] (value)}
                {#if mode === 'text' ? value === 'center' || value === 'tiled' : value !== 'tiled'}
                  <option {value}>{label}</option>
                {/if}
              {/each}
            </select>
          </label>
        </div>

        <div class="row">
          <label class="field">
            <span>Draw</span>
            <select bind:value={layer}>
              <option value="over">Over the page</option>
              <option value="under">Under the page content</option>
            </select>
          </label>
          {#if place !== 'center' && place !== 'tiled'}
            <label class="field">
              <span>Margin (mm)</span>
              <input
                type="number"
                min="0"
                max="100"
                bind:value={margin}
                class:invalid={!marginOk}
              />
            </label>
          {/if}
        </div>

        <div class="row">
          <label class="field">
            <span>Pages</span>
            <select bind:value={range}>
              <option value="all">All pages</option>
              <option value="pages">Only these pages…</option>
            </select>
          </label>
          {#if range === 'pages'}
            <label class="field">
              <span>Page range</span>
              <input
                type="text"
                placeholder="2-end"
                bind:value={pagesText}
                class:invalid={!pagesText.trim()}
              />
            </label>
          {/if}
        </div>

        <p class="note">
          A watermark added this way can be removed by someone with PDF editing tools. It is a
          visible label, not security. "Under the page content" is hidden by pages with a solid
          background, such as scans.
        </p>
      </form>

      <StampPreview
        source={doc.path}
        pageCount={count}
        rotations={doc.info.pages.map((p) => p.rotate)}
        {stamping}
        bind:error={problem}
      />
    </div>
  {/if}

  {#snippet footer()}
    <span>{plural(count, 'page')}</span>
    {#if doc}<span>Source {fmtSize(doc.size)}</span>{/if}
    <span>Text size is in points; image stamps scale with each page's width.</span>
  {/snippet}
</ToolFrame>
