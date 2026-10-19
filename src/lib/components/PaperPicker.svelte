<script lang="ts">
  import { MM_PER_INCH, PRESET_LABELS, validCustom, type PaperChoice } from '../paper';

  interface Props {
    choice: PaperChoice;
    /** Adds "Fit to image" (Images to PDF only). */
    allowFit?: boolean;
    label?: string;
  }

  let { choice = $bindable(), allowFit = false, label = 'Page size' }: Props = $props();

  // Switching units converts what is typed, so 210 mm becomes 8.27 in.
  function setUnit(unit: 'mm' | 'in') {
    if (unit === choice.unit) return;
    const k = unit === 'in' ? 1 / MM_PER_INCH : MM_PER_INCH;
    const round = (v: number) => Math.round(v * k * 100) / 100;
    choice = { ...choice, unit, width: round(choice.width), height: round(choice.height) };
  }

  const invalid = $derived(choice.kind === 'custom' && !validCustom(choice));
</script>

<label>
  {label}
  <select bind:value={choice.kind}>
    {#if allowFit}<option value="fit">Fit to image</option>{/if}
    {#each Object.entries(PRESET_LABELS) as [kind, text] (kind)}
      <option value={kind}>{text}</option>
    {/each}
    <option value="custom">Custom…</option>
  </select>
</label>
{#if choice.kind === 'custom'}
  <input
    type="number"
    min="1"
    step="any"
    class:invalid
    aria-label="Page width"
    bind:value={choice.width}
  />
  ×
  <input
    type="number"
    min="1"
    step="any"
    class:invalid
    aria-label="Page height"
    bind:value={choice.height}
  />
  <select
    aria-label="Unit"
    value={choice.unit}
    onchange={(e) => setUnit(e.currentTarget.value as 'mm' | 'in')}
  >
    <option value="mm">mm</option>
    <option value="in">in</option>
  </select>
{/if}
