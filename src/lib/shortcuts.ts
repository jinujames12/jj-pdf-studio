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
    title: 'Page Editor',
    items: [
      { keys: 'Ctrl+Z / Ctrl+Y', action: 'Undo / redo' },
      { keys: 'Ctrl+A', action: 'Select all pages' },
      { keys: 'Delete', action: 'Remove the selected pages' },
      { keys: 'R / Shift+R', action: 'Rotate right / left' },
      { keys: 'Ctrl+D', action: 'Duplicate the selected pages' },
      { keys: 'Ctrl+Shift+S', action: 'Open the Split panel' },
      { keys: 'Ctrl+E', action: 'Open the Extract panel' },
      { keys: 'Ctrl+Shift+N', action: 'Open the Page numbers panel' },
      { keys: 'Esc', action: 'Close the options panel, then clear the selection' },
      { keys: 'Right-click', action: 'Page menu: rotate, duplicate, move, insert blank, split' },
      { keys: 'Arrow keys', action: 'Move the selection (along the toolbar when it has focus)' },
      { keys: 'Shift+F10 or Menu key', action: 'Open the page menu for the current page' },
      { keys: 'Enter or Space', action: 'Preview the last selected page' }
    ]
  }
];
