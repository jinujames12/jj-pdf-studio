<script lang="ts">
  import { onMount } from 'svelte';
  import LockKeyhole from 'lucide-svelte/icons/lock-keyhole';
  import { passwordPrompt } from '../stores/password.svelte';

  const request = $derived(passwordPrompt.request);

  let input = $state<HTMLInputElement>();
  let dialog = $state<HTMLElement>();
  let password = $state('');
  let error = $state('');
  let checking = $state(false);

  // Take focus so typing and Escape work, and give it back when the dialog closes.
  onMount(() => {
    const before = document.activeElement as HTMLElement | null;
    (input ?? dialog)?.focus();
    return () => before?.focus?.();
  });

  async function submit(e: Event) {
    e.preventDefault();
    if (!request || checking || !password) return;
    checking = true;
    error = '';
    try {
      const problem = await request.verify(password);
      if (problem) {
        error = problem;
        password = '';
        input?.focus();
      } else {
        request.done(true);
      }
    } finally {
      checking = false;
    }
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && !checking) {
      e.stopPropagation();
      request?.done(false);
    }
  }
</script>

{#if request}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="backdrop" onclick={() => !checking && request.done(false)}>
    <div
      class="dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="password-title"
      tabindex="-1"
      bind:this={dialog}
      {onkeydown}
      onclick={(e) => e.stopPropagation()}
    >
      <form onsubmit={submit}>
        <span class="badge"><LockKeyhole size={28} strokeWidth={1.75} /></span>
        <h2 id="password-title">This PDF is password protected</h2>
        <p class="note" title={request.name}>
          Enter the password to open <strong>{request.name}</strong>.
        </p>
        <input
          type="password"
          bind:this={input}
          bind:value={password}
          class:invalid={error}
          autocomplete="off"
          spellcheck="false"
          aria-label="Password"
          aria-invalid={error ? 'true' : undefined}
          aria-describedby={error ? 'password-error' : undefined}
          disabled={checking}
        />
        {#if error}<p class="field-error" id="password-error" role="alert">{error}</p>{/if}
        <div class="buttons">
          <button class="btn primary" type="submit" disabled={!password || checking}>
            {checking ? 'Checking…' : 'Open'}
          </button>
          <button class="btn" type="button" disabled={checking} onclick={() => request.done(false)}>
            Cancel
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 60;
    display: grid;
    place-items: center;
    background: rgba(10, 12, 24, 0.6);
  }
  .dialog {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-2);
    width: min(92vw, 400px);
    padding: var(--space-6);
    border: 1px solid var(--border);
    border-radius: var(--radius-card);
    background: var(--surface);
    box-shadow: var(--shadow-dialog);
    outline: none;
    text-align: center;
  }
  form {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
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
  h2 {
    margin: var(--space-2) 0 0;
    font-size: var(--text-header);
    font-weight: 600;
  }
  .note {
    margin: 0;
    max-width: 100%;
    color: var(--text-muted);
    overflow-wrap: anywhere;
  }
  input {
    width: 100%;
    height: 34px;
    margin-top: var(--space-2);
    padding: 0 var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-control);
    background: var(--surface-2);
    color: var(--text);
    font: inherit;
  }
  input.invalid {
    border-color: var(--danger);
  }
  .buttons {
    display: flex;
    gap: var(--space-2);
    margin-top: var(--space-3);
  }
</style>
