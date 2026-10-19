<script lang="ts">
  import RotateCcw from 'lucide-svelte/icons/rotate-ccw';
  import RotateCw from 'lucide-svelte/icons/rotate-cw';
  import { planRotation, type RotateAction } from '../api';
  import { report } from '../saved';
  import { documents } from '../stores/documents.svelte';
  import { pageEdit } from '../stores/pageEdit.svelte';
  import { rotateSettings as s, rotateTarget } from '../stores/panels.svelte';

  const doc = $derived(documents.current);
  const turned = $derived(pageEdit.items.filter((i) => i.rotate !== 0).length);

  /**
   * Rust decides which pages an action touches (on the pages as listed in the
   * editor); the result is one undoable edit.
   */
  async function apply(action: RotateAction) {
    if (!doc) return;
    try {
      pageEdit.setTurns(
        await planRotation(doc.path, pageEdit.turns, action, pageEdit.pageGeometry())
      );
    } catch (e) {
      report(e);
    }
  }

  const turn = (degrees: number) => apply({ kind: 'turn', target: rotateTarget(), degrees });
</script>

<form class="opts" onsubmit={(e) => e.preventDefault()}>
  <label class="field">
    <span>Pages</span>
    <select bind:value={s.target}>
      <option value="all">All pages</option>
      <option value="odd">Odd pages</option>
      <option value="even">Even pages</option>
      <option value="pages">Pages…</option>
    </select>
  </label>
  {#if s.target === 'pages'}
    <input type="text" placeholder="2, 5-7" aria-label="Pages to rotate" bind:value={s.pagesText} />
  {/if}

  <div class="row">
    <button type="button" class="btn" onclick={() => turn(90)}>
      <RotateCw size={16} strokeWidth={1.75} /> 90° right
    </button>
    <button type="button" class="btn" onclick={() => turn(-90)}>
      <RotateCcw size={16} strokeWidth={1.75} /> 90° left
    </button>
    <button type="button" class="btn" onclick={() => turn(180)}>180°</button>
  </div>

  <div class="row">
    <button
      type="button"
      class="btn"
      onclick={() => apply({ kind: 'orient', orientation: 'portrait' })}
    >
      Make all portrait
    </button>
    <button
      type="button"
      class="btn"
      onclick={() => apply({ kind: 'orient', orientation: 'landscape' })}
    >
      Make all landscape
    </button>
  </div>

  <button type="button" class="btn" disabled={!turned} onclick={() => pageEdit.setTurns([])}>
    Reset rotation
  </button>

  <p class="note">
    Page numbers here are the positions on the grid. Rotation is lossless (it only sets the page's
    /Rotate) and is written when you save. Undo reverses each step.
  </p>
</form>
