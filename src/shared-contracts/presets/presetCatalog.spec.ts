import { describe, expect, it } from 'vitest';

import {
  defaultPresetId,
  getPresetCatalogEntryById,
  presetCatalog,
  presetCatalogSchema,
} from './presetCatalog.js';

describe('presetCatalog', () => {
  it('loads the approved bounded preset catalog from the checked-in asset', () => {
    expect(presetCatalogSchema.parse(presetCatalog)).toHaveLength(4);
    expect(presetCatalog.map((preset) => preset.id)).toEqual([
      'warm-tone',
      'cool-tone',
      'background-ivory',
      'background-pink',
    ]);
    expect(new Set(presetCatalog.map((preset) => preset.id)).size).toBe(presetCatalog.length);
    expect(presetCatalog.every((preset) => preset.previewRef.startsWith('preset-preview/'))).toBe(true);
    expect(defaultPresetId).toBe('warm-tone');
  });

  it('returns only approved preset ids from the typed catalog lookup', () => {
    expect(getPresetCatalogEntryById('background-pink')).toMatchObject({
      id: 'background-pink',
      name: '배경지 - 핑크',
      previewRef: 'preset-preview/background-pink',
    });
    expect(getPresetCatalogEntryById('not-approved')).toBeUndefined();
  });
});
