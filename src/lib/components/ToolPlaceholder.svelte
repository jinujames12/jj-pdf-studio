<script lang="ts">
  import type { ToolDef } from '../tools';

  let { tool }: { tool: ToolDef } = $props();
</script>

<section class="tool">
  <header>
    <h1>{tool.name}</h1>
    <p>{tool.description}</p>
  </header>

  <div class="empty">
    <span class="badge"><tool.icon size={28} strokeWidth={1.75} /></span>
    <h2>Coming soon</h2>
    <p>This tool is not built yet. It will arrive in an upcoming release.</p>
  </div>
</section>

<style>
  .tool {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: var(--space-6);
  }
  h1 {
    margin: 0;
    font-size: var(--text-title);
    font-weight: 600;
  }
  header p {
    margin: var(--space-1) 0 0;
    color: var(--text-muted);
  }
  .empty {
    display: flex;
    flex: 1;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    margin-top: var(--space-6);
    border: 1px dashed var(--border);
    border-radius: var(--radius-card);
    background: var(--surface);
    text-align: center;
  }
  .empty h2 {
    margin: 0;
    font-size: var(--text-header);
    font-weight: 600;
  }
  .empty p {
    margin: 0;
    color: var(--text-muted);
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
</style>
