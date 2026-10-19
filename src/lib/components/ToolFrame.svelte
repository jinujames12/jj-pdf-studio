<script lang="ts">
  import type { Snippet } from 'svelte';
  import FolderOpen from 'lucide-svelte/icons/folder-open';
  import { documents } from '../stores/documents.svelte';

  interface Props {
    title: string;
    subtitle: string;
    /** True while there is nothing to work on: shows the drop zone instead. */
    empty: boolean;
    emptyTitle?: string;
    emptyHint?: string;
    onopen: () => void;
    /** Buttons on the right of the heading. */
    actions?: Snippet;
    /** Option controls between the heading and the grid. */
    toolbar?: Snippet;
    footer?: Snippet;
    /** The scrollable workspace. */
    children: Snippet;
  }

  let {
    title,
    subtitle,
    empty,
    emptyTitle = 'Drop a PDF here or click to browse',
    emptyHint = 'Your original is never changed. Results are saved to new files.',
    onopen,
    actions,
    toolbar,
    footer,
    children
  }: Props = $props();
</script>

<section class="tool">
  <header>
    <div class="heading">
      <h1>{title}</h1>
      <p>{subtitle}</p>
    </div>
    <div class="header-actions">{@render actions?.()}</div>
  </header>

  {#if empty}
    <button class="empty" class:hovering={documents.dragging} onclick={onopen}>
      <span class="badge"><FolderOpen size={28} strokeWidth={1.75} /></span>
      <span class="title">{documents.loading ? 'Opening…' : emptyTitle}</span>
      <span class="hint">{emptyHint}</span>
    </button>
  {:else}
    {#if toolbar}
      <div class="tool-toolbar" role="toolbar" aria-label="{title} options">
        {@render toolbar()}
      </div>
    {/if}
    <div class="scroll">{@render children()}</div>
    {#if footer}<footer>{@render footer()}</footer>{/if}
  {/if}
</section>

<style>
  .tool {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: var(--space-6);
    gap: var(--space-4);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
  }
  .heading {
    min-width: 0;
  }
  h1 {
    margin: 0;
    font-size: var(--text-title);
    font-weight: 600;
  }
  header p {
    margin: var(--space-1) 0 0;
    overflow: hidden;
    color: var(--text-muted);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .header-actions {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }
  .tool-toolbar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2) var(--space-3);
    padding: var(--space-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-card);
    background: var(--surface);
  }
  .scroll {
    flex: 1;
    min-height: 0;
    overflow: auto;
    border: 1px solid var(--border);
    border-radius: var(--radius-card);
    background: var(--surface);
  }
  footer {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-4);
    color: var(--text-muted);
    font-size: var(--text-caption);
  }
  .empty {
    display: flex;
    flex: 1;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    border: 2px dashed var(--border);
    border-radius: var(--radius-card);
    background: var(--surface);
    cursor: pointer;
    transition:
      border-color var(--duration-base) var(--ease),
      background var(--duration-base) var(--ease);
  }
  .empty:hover,
  .empty.hovering {
    border-color: var(--primary);
    background: var(--selected-bg);
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
  .title {
    font-size: var(--text-header);
    font-weight: 600;
  }
  .hint {
    color: var(--text-muted);
    font-size: var(--text-caption);
  }
</style>
