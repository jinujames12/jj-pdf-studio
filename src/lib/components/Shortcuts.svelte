<script lang="ts">
  import { shortcutGroups } from '../shortcuts';
  import { dialogs } from '../stores/dialogs.svelte';
  import Modal from './Modal.svelte';
</script>

<Modal title="Keyboard shortcuts" onclose={() => (dialogs.open = null)}>
  {#each shortcutGroups as group (group.title)}
    <h3>{group.title}</h3>
    <dl>
      {#each group.items as item (item.keys)}
        <dt><kbd>{item.keys}</kbd></dt>
        <dd>{item.action}</dd>
      {/each}
    </dl>
  {/each}
</Modal>

<style>
  h3 {
    margin: var(--space-3) 0 var(--space-2);
    font-size: var(--text-caption);
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-muted);
  }
  h3:first-child {
    margin-top: 0;
  }
  dl {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: var(--space-2) var(--space-4);
    margin: 0;
    align-items: center;
  }
  dd {
    margin: 0;
  }
  kbd {
    display: inline-block;
    padding: 1px var(--space-2);
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--surface-2);
    font: inherit;
    font-size: var(--text-caption);
    white-space: nowrap;
  }
</style>
