import { describe, expect, it } from 'vitest';
import { CANVAS_HEIGHT, CANVAS_WIDTH, clampRect, inversePointer, compatibleMetrics, createWidgetId, isLatestRequest, resizeRect } from './model';

describe('designer model', () => {
  it('keeps the logical canvas fixed and clamps pointer geometry', () => {
    expect([CANVAS_WIDTH, CANVAS_HEIGHT]).toEqual([960, 376]);
    expect(inversePointer({ left: 10, top: 20, width: 480, height: 188 }, 100, 50)).toEqual({ x: 180, y: 60 });
    expect(clampRect({ x: 900, y: 350, width: 100, height: 100 })).toEqual({ x: 860, y: 276, width: 100, height: 100 });
    expect(inversePointer({ left: 10, top: 20, width: 960, height: 376 }, 970, 396)).toEqual({ x: 960, y: 376 });
    expect(resizeRect({ x: 900, y: 340, width: 40, height: 36 }, { x: 20, y: 0 })).toEqual({ x: 900, y: 340, width: 60, height: 36 });
    expect(resizeRect({ x: 10, y: 10, width: 40, height: 36 }, { x: -100, y: -100 })).toEqual({ x: 10, y: 10, width: 1, height: 1 });
  });

  it('filters metrics by widget compatibility', () => {
    expect(compatibleMetrics([{ id: 'a', label: 'a', provider_name: 'p', category: 'c', unit: '', recommended_widgets: ['gauge'] }, { id: 'b', label: 'b', provider_name: 'p', category: 'c', unit: '', recommended_widgets: ['text'] }], 'gauge')).toHaveLength(1);
  });

  it('accepts only the latest page request', () => {
    expect(isLatestRequest(1, 2)).toBe(false);
    expect(isLatestRequest(2, 2)).toBe(true);
  });

  it('creates distinct fallback widget ids in the same millisecond', () => {
    expect(createWidgetId(null, () => 123)).not.toBe(createWidgetId(null, () => 123));
  });
});
