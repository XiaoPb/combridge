import { describe, expect, it } from 'vitest';
import {
  ensureWhiteSvgBackground,
  normalizeSvgExportDataUrl,
  resolveCssVariablesInValue,
} from './multiLineChartExport';

describe('resolveCssVariablesInValue', () => {
  it('resolves CSS variables in nested objects and arrays without mutating input', () => {
    const formatter = () => 'preserved';
    const input = {
      axis: {
        lineStyle: { color: 'var(--border-color)' },
        label: ['var(--text-primary)', 3, formatter, null],
      },
      unchanged: 'plain text',
    };
    const resolved = resolveCssVariablesInValue(input, (name) => ({
      '--border-color': '#d9d9d9',
      '--text-primary': '#141414',
    })[name]);

    expect(resolved).toEqual({
      axis: {
        lineStyle: { color: '#d9d9d9' },
        label: ['#141414', 3, formatter, null],
      },
      unchanged: 'plain text',
    });
    expect(resolved).not.toBe(input);
    expect(resolved.axis).not.toBe(input.axis);
    expect(input.axis.lineStyle.color).toBe('var(--border-color)');
  });

  it('resolves multiple variables and uses a fallback when a variable is unavailable', () => {
    const resolved = resolveCssVariablesInValue(
      'border: var(--border-color); text: var(--missing, #fff);',
      (name) => (name === '--border-color' ? '#d9d9d9' : undefined),
    );

    expect(resolved).toBe('border: #d9d9d9; text: #fff;');
  });

  it('keeps unresolved variables without fallbacks and non-var strings unchanged', () => {
    const value = 'var(--missing) and url(var(--also-missing))';

    expect(resolveCssVariablesInValue(value, () => undefined)).toBe(value);
    expect(resolveCssVariablesInValue('not a var()', () => '#fff')).toBe(
      'not a var()',
    );
  });

  it('preserves repeated and cyclic object references while cloning plain objects', () => {
    const shared = { color: 'var(--border-color)' };
    const input: { first: typeof shared; second: typeof shared; self?: unknown } = {
      first: shared,
      second: shared,
    };
    input.self = input;

    const resolved = resolveCssVariablesInValue(input, () => '#d9d9d9');

    expect(resolved.first).toBe(resolved.second);
    expect(resolved.self).toBe(resolved);
    expect(resolved.first.color).toBe('#d9d9d9');
    expect(input.first.color).toBe('var(--border-color)');
  });
});

describe('normalizeSvgExportDataUrl', () => {
  it('keeps a true SVG data URL unchanged', () => {
    const url = 'data:image/svg+xml;charset=UTF-8,%3Csvg%3E%3C%2Fsvg%3E';

    expect(normalizeSvgExportDataUrl(url)).toBe(url);
  });

  it('converts an SVG string into a data URL', () => {
    expect(normalizeSvgExportDataUrl('<svg viewBox="0 0 1 1"></svg>')).toBe(
      'data:image/svg+xml;charset=UTF-8,%3Csvg%20viewBox%3D%220%200%201%201%22%3E%3C%2Fsvg%3E',
    );
  });

  it('rejects non-SVG export data', () => {
    expect(() => normalizeSvgExportDataUrl('data:image/png;base64,AAAA')).toThrow(
      'ECharts returned a non-SVG export',
    );
  });

  it('rejects XML without an SVG root element', () => {
    expect(() => normalizeSvgExportDataUrl('<?xml version="1.0"?><document />')).toThrow(
      'ECharts returned a non-SVG export',
    );
  });
});

describe('ensureWhiteSvgBackground', () => {
  it('adds a white background element when an SVG has no serialized background', () => {
    const svg = ensureWhiteSvgBackground('<svg viewBox="0 0 10 20"></svg>', 10, 20);

    expect(decodeURIComponent(svg.split(',')[1])).toContain(
      '<rect x="0" y="0" width="10" height="20" fill="#fff"/>',
    );
  });

  it('does not duplicate an existing white background element', () => {
    const input = '<svg><rect x="0" y="0" width="10" height="20" fill="#fff"/></svg>';

    expect(ensureWhiteSvgBackground(input, 10, 20)).toBe(
      normalizeSvgExportDataUrl(input),
    );
  });

  it('decodes base64 SVG data URLs before adding a white background', () => {
    const input = `data:image/svg+xml;base64,${btoa('<svg viewBox="0 0 10 20"></svg>')}`;

    const output = ensureWhiteSvgBackground(input, 10, 20);

    expect(output).toMatch(/^data:image\/svg\+xml;charset=UTF-8,/);
    expect(decodeURIComponent(output.split(',')[1])).toContain(
      '<rect x="0" y="0" width="10" height="20" fill="#fff"/>',
    );
  });

  it('expands a self-closing SVG root before inserting the background', () => {
    const output = ensureWhiteSvgBackground(
      '<svg viewBox="0 0 10 20"/>',
      10,
      20,
    );
    const decoded = decodeURIComponent(output.split(',')[1]);

    expect(decoded).toBe(
      '<svg viewBox="0 0 10 20"><rect x="0" y="0" width="10" height="20" fill="#fff"/></svg>',
    );
    expect(decoded).toMatch(/^<svg\b[^>]*>.*<\/svg>$/);
  });
});
