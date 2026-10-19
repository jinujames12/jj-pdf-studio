// Which app-wide dialog is open (opened from About, the F1 key or the title bar).
export const dialogs = $state<{ open: 'shortcuts' | 'notices' | null }>({ open: null });
