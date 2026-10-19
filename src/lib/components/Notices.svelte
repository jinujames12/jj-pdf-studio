<script lang="ts">
  import { onMount } from 'svelte';
  import { dialogs } from '../stores/dialogs.svelte';
  import Modal from './Modal.svelte';

  let text = $state('');
  let failed = $state(false);

  // The notices are large; load them only when someone asks to read them.
  onMount(async () => {
    try {
      text = (await import('../../../THIRD_PARTY_NOTICES.md?raw')).default;
    } catch {
      failed = true;
    }
  });
</script>

<Modal title="Third-party licenses" wide onclose={() => (dialogs.open = null)}>
  {#if failed}
    <p>The license list could not be loaded. It is also in THIRD_PARTY_NOTICES.md.</p>
  {:else if !text}
    <p>Loading…</p>
  {:else}
    <!-- Scrollable text needs to be reachable by keyboard. -->
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <pre tabindex="0" aria-label="Third-party licenses">{text}</pre>
  {/if}
</Modal>

<style>
  pre {
    margin: 0;
    font-family: var(--font-mono, ui-monospace, Consolas, monospace);
    font-size: var(--text-caption);
    white-space: pre-wrap;
    word-break: break-word;
    user-select: text;
  }
</style>
