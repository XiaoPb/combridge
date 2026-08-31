import { describe, expect, it } from 'vitest';
import { normalizeSvgExportDataUrl } from './multiLineChartExport';

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
