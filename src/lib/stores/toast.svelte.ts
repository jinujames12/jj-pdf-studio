import { CANCELLED } from './jobs.svelte';

// Small toast queue. Errors stay until dismissed; other toasts fade after a while.

export interface ToastAction {
  label: string;
  run: () => void;
}

export interface Toast {
  id: number;
  kind: 'success' | 'error' | 'info';
  message: string;
  actions: ToastAction[];
}

export const toasts = $state<{ list: Toast[] }>({ list: [] });

let nextId = 1;

export function dismiss(id: number) {
  toasts.list = toasts.list.filter((t) => t.id !== id);
}

export function toast(kind: Toast['kind'], message: string, actions: ToastAction[] = []) {
  // Pressing Cancel on a job is not an error.
  if (kind === 'error' && message === CANCELLED) {
    kind = 'info';
    message = 'Cancelled. No file was saved.';
  }
  const id = nextId++;
  toasts.list = [...toasts.list, { id, kind, message, actions }];
  if (kind !== 'error') setTimeout(() => dismiss(id), actions.length ? 12000 : 6000);
}
