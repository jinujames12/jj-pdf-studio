import { beforeEach, describe, expect, it } from 'vitest';
import type { PageNumberOptions } from '../api';
import { pageEdit as edit } from './pageEdit.svelte';

// A portrait page, a landscape page, a page that is rotated 90° in the file, a portrait page.
const doc = {
  pages: [
    { width: 600, height: 800, rotate: 0 },
    { width: 800, height: 600, rotate: 0 },
    { width: 600, height: 800, rotate: 90 },
    { width: 600, height: 800, rotate: 0 }
  ]
};

const order = () => edit.items.map((i) => (i.kind === 'page' ? i.src : 0));
const idOf = (position: number) => edit.items[position - 1].id;
const pick = (...positions: number[]) => new Set(positions.map(idOf));

const numbering: PageNumberOptions = {
  position: 'bottom-center',
  margin_mm: 10,
  font_size: 11,
  color: '#000000',
  format: 'Page {n} of {total}',
  start: 1,
  pages: null
};

beforeEach(() => edit.reset(doc));

describe('reorder', () => {
  it('moves a single page', () => {
    edit.move(idOf(1), 2); // drop page 1 at index 2
    expect(order()).toEqual([2, 3, 1, 4]);
  });

  it('moves a multi-selection together, keeping its order', () => {
    edit.select(idOf(1));
    edit.select(idOf(2), 'toggle');
    edit.move(idOf(1), 3);
    expect(order()).toEqual([3, 4, 1, 2]);
  });

  it('ignores a drop where the page already is', () => {
    edit.move(idOf(2), 1);
    expect(edit.dirty).toBe(false);
  });
});

describe('rotate', () => {
  it('turns a selection in 90° steps and wraps around', () => {
    edit.select(idOf(1));
    edit.select(idOf(2), 'toggle');
    edit.rotate(90);
    edit.rotate(90);
    edit.rotate(90);
    edit.rotate(90);
    expect(edit.turns).toEqual([0, 0, 0, 0]);
    edit.rotate(-90);
    expect(edit.turns).toEqual([270, 270, 0, 0]);
  });

  it('sends the extra turn on top of the page, not the page total (lossless /Rotate)', () => {
    edit.rotate(90, pick(3));
    expect(edit.toOps()[2]).toEqual({ kind: 'page', page: 3, rotate: 90 });
  });

  it('does nothing without a selection', () => {
    edit.rotate(90);
    expect(edit.dirty).toBe(false);
  });

  it('applies planned turns as one undo step', () => {
    edit.setTurns([90, 0, 180, 270]);
    expect(edit.turns).toEqual([90, 0, 180, 270]);
    edit.undo();
    expect(edit.turns).toEqual([0, 0, 0, 0]);
  });

  it('describes the current list for orientation planning', () => {
    edit.insertBlankAt(1, 'A4');
    const geometry = edit.pageGeometry();
    expect(geometry).toHaveLength(5);
    expect(geometry[1]).toEqual({ width: 595, height: 842, rotate: 0 });
    expect(geometry[3]).toEqual({ width: 600, height: 800, rotate: 90 });
  });
});

describe('delete', () => {
  it('removes the selection', () => {
    expect(edit.remove(pick(2, 3))).toBe(true);
    expect(order()).toEqual([1, 4]);
  });

  it('refuses to delete the last page', () => {
    expect(edit.remove(pick(1, 2, 3))).toBe(true);
    expect(edit.remove(pick(1))).toBe(false);
    expect(order()).toEqual([4]);
  });

  it('refuses to delete every page at once', () => {
    expect(edit.remove(pick(1, 2, 3, 4))).toBe(false);
    expect(order()).toEqual([1, 2, 3, 4]);
    expect(edit.dirty).toBe(false);
  });
});

describe('insert blank', () => {
  it('uses the size of the previous page by default and shows rotation', () => {
    edit.insertBlankAt(2, 'same'); // after the landscape page
    expect(edit.items[2]).toMatchObject({ kind: 'blank', width: 800, height: 600 });
  });

  it('takes the on-screen size of a page rotated in the file', () => {
    edit.insertBlankAt(3, 'same'); // after page 3, which the file rotates by 90°
    expect(edit.items[3]).toMatchObject({ kind: 'blank', width: 800, height: 600 });
  });

  it.each([
    ['A4', 595, 842],
    ['A3', 842, 1191],
    ['A5', 420, 595],
    ['Letter', 612, 792],
    ['Legal', 612, 1008]
  ] as const)('creates a %s page', (size, width, height) => {
    edit.insertBlankAt(0, size);
    expect(edit.items[0]).toMatchObject({ kind: 'blank', width, height });
  });

  it('inserts after the last selected page from the toolbar', () => {
    edit.select(idOf(2));
    edit.insertBlank('A4');
    expect(edit.items[2].kind).toBe('blank');
    expect(edit.selected.has(edit.items[2].id)).toBe(true);
  });

  it('inserts between thumbnails, before or after a page', () => {
    edit.insertBlankNextTo(idOf(2), 'after', 'same');
    expect(edit.items.map((i) => i.kind)).toEqual(['page', 'page', 'blank', 'page', 'page']);
    edit.insertBlankNextTo(idOf(1), 'before', 'Letter');
    expect(edit.items[0]).toMatchObject({ kind: 'blank', width: 612, height: 792 });
  });

  it('is written as a blank operation', () => {
    edit.insertBlankAt(4, 'A4');
    expect(edit.toOps()[4]).toEqual({ kind: 'blank', width: 595, height: 842, rotate: 0 });
  });
});

describe('duplicate and move to the ends', () => {
  it('copies each selected page right after itself and selects the copies', () => {
    edit.duplicate(pick(1, 3));
    expect(order()).toEqual([1, 1, 2, 3, 3, 4]);
    expect(edit.selected.size).toBe(2);
    expect(new Set(edit.items.map((i) => i.id)).size).toBe(6);
  });

  it('keeps the edit on the copy', () => {
    edit.rotate(90, pick(2));
    edit.duplicate(pick(2));
    expect(edit.turns).toEqual([0, 90, 90, 0, 0]);
  });

  it('moves the selection to the start or the end in order', () => {
    edit.moveToEdge('start', pick(3, 4));
    expect(order()).toEqual([3, 4, 1, 2]);
    edit.moveToEdge('end', pick(1)); // position 1 is now original page 3
    expect(order()).toEqual([4, 1, 2, 3]);
  });
});

describe('selection', () => {
  it('selects with click, ctrl and shift', () => {
    edit.select(idOf(1));
    edit.select(idOf(3), 'range');
    expect(edit.selectedPositions).toEqual([1, 2, 3]);
    edit.select(idOf(2), 'toggle');
    expect(edit.selectedPositions).toEqual([1, 3]);
  });

  it('selects odd, even, inverse and by range', () => {
    edit.selectWhere((_, n) => n % 2 === 1);
    expect(edit.selectedPositions).toEqual([1, 3]);
    edit.invertSelection();
    expect(edit.selectedPositions).toEqual([2, 4]);
    edit.selectPositions([1, 4, 99]);
    expect(edit.selectedPositions).toEqual([1, 4]);
  });

  it('selects landscape and portrait pages as they appear, after rotation', () => {
    edit.selectOrientation('landscape');
    expect(edit.selectedPositions).toEqual([2, 3]); // page 3 is turned by its own /Rotate
    edit.rotate(90, pick(1));
    edit.selectOrientation('landscape');
    expect(edit.selectedPositions).toEqual([1, 2, 3]);
    edit.selectOrientation('portrait');
    expect(edit.selectedPositions).toEqual([4]);
  });

  it('drops deleted pages from the selection', () => {
    edit.selectAll();
    edit.remove(pick(1));
    expect(edit.selected.size).toBe(3);
  });
});

describe('undo and redo', () => {
  it('covers every kind of edit and writes nothing until asked', () => {
    const steps: Array<() => void> = [
      () => edit.rotate(90, pick(1)),
      () => edit.remove(pick(2)),
      () => edit.move(idOf(1), 2),
      () => edit.insertBlankAt(1, 'A4'),
      () => edit.duplicate(pick(1)),
      () => edit.moveToEdge('end', pick(1)),
      () => edit.setTurns([90, 90, 90, 90, 90, 90]),
      () => edit.setNumbering(numbering)
    ];
    const snapshots = [{ ops: edit.toOps(), numbering: edit.numbering }];
    for (const step of steps) {
      step();
      snapshots.push({ ops: edit.toOps(), numbering: edit.numbering });
    }
    expect(edit.dirty).toBe(true);
    for (let k = steps.length - 1; k >= 0; k--) {
      edit.undo();
      expect({ ops: edit.toOps(), numbering: edit.numbering }).toEqual(snapshots[k]);
    }
    expect(edit.canUndo).toBe(false);
    expect(edit.dirty).toBe(false);
    for (let k = 1; k <= steps.length; k++) {
      edit.redo();
      expect({ ops: edit.toOps(), numbering: edit.numbering }).toEqual(snapshots[k]);
    }
    expect(edit.canRedo).toBe(false);
  });

  it('clears the redo list after a new edit', () => {
    edit.rotate(90, pick(1));
    edit.undo();
    edit.rotate(180, pick(2));
    expect(edit.canRedo).toBe(false);
  });

  it('a new document starts clean', () => {
    edit.rotate(90, pick(1));
    edit.setNumbering(numbering);
    edit.reset({ pages: [{ width: 1, height: 1, rotate: 0 }] });
    expect(edit.items).toHaveLength(1);
    expect(edit.dirty).toBe(false);
    expect(edit.numbering).toBeNull();
  });
});

describe('page numbering', () => {
  it('is an undoable edit that does not touch the page list', () => {
    const before = edit.toOps();
    edit.setNumbering(numbering);
    expect(edit.numbering).toEqual(numbering);
    expect(edit.toOps()).toEqual(before);
    expect(edit.dirty).toBe(true);
    edit.undo();
    expect(edit.numbering).toBeNull();
  });

  it('does not add a history step when nothing changed', () => {
    edit.setNumbering(numbering);
    edit.setNumbering({ ...numbering });
    edit.undo();
    expect(edit.canUndo).toBe(false);
  });

  it('can be removed again', () => {
    edit.setNumbering(numbering);
    edit.setNumbering(null);
    expect(edit.numbering).toBeNull();
  });
});

describe('operations for export', () => {
  it('lists only the selected pages, in grid order, with the edits applied', () => {
    edit.move(idOf(4), 0);
    edit.rotate(90, pick(1));
    edit.insertBlankAt(2, 'A5');
    expect(edit.opsFor(new Set([edit.items[0].id, edit.items[2].id]))).toEqual([
      { kind: 'page', page: 4, rotate: 90 },
      { kind: 'blank', width: 420, height: 595, rotate: 0 }
    ]);
  });
});
