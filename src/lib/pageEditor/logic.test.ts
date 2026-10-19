import { describe, expect, it } from 'vitest';
import { formatRanges } from '../format';
import {
  editedName,
  markerPositions,
  numberLabels,
  rangesToPages,
  splitFileCount,
  splitGroupOf,
  splitPartLabel,
  splitReady,
  splitTint,
  type SplitView
} from './logic';

const view = (over: Partial<SplitView>): SplitView => ({
  kind: 'every',
  perFile: 1,
  ranges: [],
  cuts: new Set(),
  ...over
});

describe('split: every N pages', () => {
  const v = view({ kind: 'every', perFile: 3 });

  it('defaults to one page per file', () => {
    expect(splitFileCount(view({}), 5)).toBe(5);
  });

  it('counts a short last file', () => {
    expect(splitFileCount(v, 7)).toBe(3);
  });

  it('groups pages and alternates the tints', () => {
    expect([1, 2, 3, 4, 7].map((p) => splitGroupOf(v, p))).toEqual([0, 0, 0, 1, 2]);
    expect([1, 4, 7].map((p) => splitTint(v, p))).toEqual(['a', 'b', 'a']);
  });

  it('labels the first page of each part', () => {
    expect([1, 2, 3, 4, 5].map((p) => splitPartLabel(v, p))).toEqual([
      'Part 1',
      null,
      null,
      'Part 2',
      null
    ]);
  });
});

describe('split: by ranges', () => {
  const v = view({
    kind: 'ranges',
    ranges: [
      [1, 3],
      [5, 6]
    ]
  });

  it('highlights only the pages in a range', () => {
    expect([1, 3, 4, 5, 7].map((p) => splitGroupOf(v, p))).toEqual([0, 0, -1, 1, -1]);
    expect(splitTint(v, 4)).toBeNull();
    expect(splitPartLabel(v, 4)).toBeNull();
  });

  it('is ready only with at least one valid range', () => {
    expect(splitReady(v, 8)).toBe(true);
    expect(splitReady(view({ kind: 'ranges' }), 8)).toBe(false);
    expect(splitFileCount(v, 8)).toBe(2);
  });
});

describe('split: at markers', () => {
  const v = view({ kind: 'at', cuts: new Set([2, 5]) });

  it('cuts after the marked pages', () => {
    expect([1, 2, 3, 5, 6].map((p) => splitGroupOf(v, p))).toEqual([0, 0, 1, 1, 2]);
    expect(splitFileCount(v, 6)).toBe(3);
  });

  it('needs a marker before it can run', () => {
    expect(splitReady(v, 6)).toBe(true);
    expect(splitReady(view({ kind: 'at' }), 6)).toBe(false);
  });

  it('keeps markers on pages while they move, and ignores the last page', () => {
    // Markers are page ids ("after this page"); positions follow the current order.
    expect(markerPositions(new Set([30, 10]), [10, 20, 30, 40])).toEqual([1, 3]);
    expect(markerPositions(new Set([40]), [10, 20, 30, 40])).toEqual([]);
    expect(markerPositions(new Set([30, 10]), [30, 40, 10, 20])).toEqual([1, 3]);
  });
});

describe('extract: grid and text stay in step', () => {
  it('formats the selection as ranges', () => {
    expect(formatRanges([7, 2, 3, 5, 8, 4])).toBe('2-5, 7-8');
  });

  it('expands typed ranges to pages', () => {
    expect(
      rangesToPages([
        [2, 2],
        [5, 7]
      ])
    ).toEqual([2, 5, 6, 7]);
    expect(formatRanges(rangesToPages([[1, 3]]))).toBe('1-3');
  });
});

describe('page number labels', () => {
  const all = (format: string, start = 1, count = 4) => [
    ...numberLabels({ format, start }, count, null).values()
  ];

  it.each([
    ['{n}', ['1', '2', '3', '4']],
    ['Page {n}', ['Page 1', 'Page 2', 'Page 3', 'Page 4']],
    ['Page {n} of {total}', ['Page 1 of 4', 'Page 2 of 4', 'Page 3 of 4', 'Page 4 of 4']],
    ['{n} / {total}', ['1 / 4', '2 / 4', '3 / 4', '4 / 4']],
    ['- {n} -', ['- 1 -', '- 2 -', '- 3 -', '- 4 -']]
  ])('formats %s', (format, expected) => {
    expect(all(format)).toEqual(expected);
  });

  it('starts at the start number and counts total from there', () => {
    expect(all('{n} of {total}', 5, 3)).toEqual(['5 of 7', '6 of 7', '7 of 7']);
  });

  it('skips the cover: the first numbered page gets the start number', () => {
    const labels = numberLabels({ format: 'Page {n} of {total}', start: 1 }, 5, [2, 3, 4, 5]);
    expect(labels.has(1)).toBe(false);
    expect(labels.get(2)).toBe('Page 1 of 4');
    expect(labels.get(5)).toBe('Page 4 of 4');
  });
});

describe('save name', () => {
  it('is prefilled next to the source and never the source name', () => {
    expect(editedName('report')).toBe('report (edited).pdf');
  });
});
