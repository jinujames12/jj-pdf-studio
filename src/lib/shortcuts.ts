// The keyboard shortcuts listed in the Shortcuts dialog and in the README. The handlers
// live in the tools themselves; keep this list in step with them.
export interface ShortcutGroup {
  title: string;
  items: Array<{ keys: string; action: string }>;
}

export const shortcutGroups: ShortcutGroup[] = [
  {
    title: 'Everywhere',
    items: [
      { keys: 'Ctrl+O', action: 'Open a PDF' },
      { keys: 'Ctrl+S', action: 'Save the result' },
      { keys: 'Esc', action: 'Cancel a running job, or close a dialog' },
      { keys: 'F1', action: 'Show this list' }
    ]
  },
  {
    title: 'Pages',
    items: [
      { keys: 'Ctrl+Z / Ctrl+Y', action: 'Undo / redo' },
      { keys: 'Ctrl+A', action: 'Select all pages' },
      { keys: 'Delete', action: 'Remove the selected pages' },
      { keys: 'R / Shift+R', action: 'Rotate right / left' },
      { keys: 'Arrow keys', action: 'Move the selected pages' },
      { keys: 'Enter or Space', action: 'Preview the last selected page' }
    ]
  }
];
