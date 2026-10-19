<script lang="ts">
  import CircleCheck from 'lucide-svelte/icons/circle-check';
  import CircleAlert from 'lucide-svelte/icons/circle-alert';
  import Info from 'lucide-svelte/icons/info';
  import X from 'lucide-svelte/icons/x';
  import { dismiss, toasts } from '../stores/toast.svelte';
</script>

<div class="stack" aria-live="polite">
  {#each toasts.list as t (t.id)}
    <div class="toast {t.kind}" role={t.kind === 'error' ? 'alert' : 'status'}>
      <span class="icon">
        {#if t.kind === 'success'}<CircleCheck size={18} strokeWidth={1.75} />
        {:else if t.kind === 'error'}<CircleAlert size={18} strokeWidth={1.75} />
        {:else}<Info size={18} strokeWidth={1.75} />{/if}
      </span>
      <div class="body">
        <p>{t.message}</p>
        {#if t.actions.length}
          <div class="row">
            {#each t.actions as a (a.label)}
              <button class="link" onclick={a.run}>{a.label}</button>
            {/each}
          </div>
        {/if}
      </div>
      <button class="close" aria-label="Dismiss" onclick={() => dismiss(t.id)}>
        <X size={14} strokeWidth={1.75} />
      </button>
    </div>
  {/each}
</div>

<style>
  .stack {
    position: fixed;
    right: var(--space-4);
    bottom: var(--space-4);
    z-index: 50;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    width: min(380px, calc(100vw - 32px));
  }
  .toast {
    display: flex;
    gap: var(--space-3);
    align-items: flex-start;
    padding: var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-card);
    background: var(--surface);
    box-shadow: var(--shadow-dialog);
    animation: toast-in var(--duration-base) var(--ease);
  }
  .success .icon {
    color: var(--success);
  }
  .error .icon {
    color: var(--danger);
  }
  .info .icon {
    color: var(--primary);
  }
  .body {
    flex: 1;
    min-width: 0;
  }
  p {
    margin: 0;
    overflow-wrap: anywhere;
    user-select: text;
  }
  .row {
    display: flex;
    gap: var(--space-4);
    margin-top: var(--space-1);
  }
  .link {
    padding: 0;
    border: 0;
    background: none;
    color: var(--primary);
    font-weight: 600;
    cursor: pointer;
  }
  .link:hover {
    text-decoration: underline;
  }
  .close {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border: 0;
    border-radius: var(--radius-control);
    background: none;
    color: var(--text-muted);
    cursor: pointer;
  }
  .close:hover {
    background: var(--hover-bg);
  }
  @keyframes toast-in {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
  }
</style>
