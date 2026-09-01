import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  composeChartPng,
  dataUrlToBlob,
  downloadBlob,
  ensureWhiteSvgBackground,
  normalizeSvgExportDataUrl,
  resolveCssVariablesInValue,
} from './multiLineChartExport';

const PNG_ONE = 'data:image/png;base64,AAECAwQ=';
const PNG_TWO = 'data:image/png;base64,BQYHCAk=';

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe('dataUrlToBlob', () => {
  it('decodes a base64 PNG data URL into a PNG Blob', async () => {
    const blob = dataUrlToBlob(PNG_ONE);

    expect(blob).toBeInstanceOf(Blob);
    expect(blob.type).toBe('image/png');
    expect([...new Uint8Array(await blob.arrayBuffer())]).toEqual([0, 1, 2, 3, 4]);
  });

  it('decodes an encoded SVG data URL into an SVG Blob', async () => {
    const svg = '<svg xmlns="http://www.w3.org/2000/svg" />';
    const blob = dataUrlToBlob(
      `data:image/svg+xml;charset=UTF-8,${encodeURIComponent(svg)}`,
    );

    expect(blob.type).toBe('image/svg+xml');
    expect(await blob.text()).toBe(svg);
  });

  it('rejects data URLs with an unsupported MIME type', () => {
    expect(() => dataUrlToBlob('data:image/jpeg;base64,AAAA')).toThrow(
      'Expected a PNG or SVG data URL',
    );
  });
});

describe('downloadBlob', () => {
  it('downloads through an object URL and always cleans up the link', () => {
    const link = {
      href: '',
      download: '',
      click: vi.fn(),
      remove: vi.fn(),
    };
    const appendChild = vi.fn();
    vi.stubGlobal('document', {
      createElement: vi.fn(() => link),
      body: { appendChild },
    });
    vi.stubGlobal('URL', {
      createObjectURL: vi.fn(() => 'blob:chart'),
      revokeObjectURL: vi.fn(),
    });

    downloadBlob(new Blob(['chart'], { type: 'image/png' }), 'charts.png');

    expect(URL.createObjectURL).toHaveBeenCalledOnce();
    expect(link.href).toBe('blob:chart');
    expect(link.download).toBe('charts.png');
    expect(appendChild).toHaveBeenCalledWith(link);
    expect(link.click).toHaveBeenCalledOnce();
    expect(link.remove).toHaveBeenCalledOnce();
    expect(URL.revokeObjectURL).toHaveBeenCalledWith('blob:chart');
  });

  it('cleans up when clicking the download link fails', () => {
    const link = {
      href: '',
      download: '',
      click: vi.fn(() => {
        throw new Error('click failed');
      }),
      remove: vi.fn(),
    };
    vi.stubGlobal('document', {
      createElement: vi.fn(() => link),
      body: { appendChild: vi.fn() },
    });
    vi.stubGlobal('URL', {
      createObjectURL: vi.fn(() => 'blob:chart'),
      revokeObjectURL: vi.fn(),
    });

    expect(() => downloadBlob(new Blob(['chart']), 'charts.png')).toThrow(
      'click failed',
    );
    expect(link.remove).toHaveBeenCalledOnce();
    expect(URL.revokeObjectURL).toHaveBeenCalledWith('blob:chart');
  });
});

describe('composeChartPng', () => {
  function createAdapters(options: {
    images?: Array<{ width: number; height: number; label: string }>;
    toBlob?: (callback: BlobCallback, type?: string) => void;
  }) {
    const drawImage = vi.fn();
    const fillRect = vi.fn();
    const canvas = {
      width: 0,
      height: 0,
      getContext: vi.fn(() => ({
        fillStyle: '',
        fillRect,
        drawImage,
      })),
      toBlob: options.toBlob ?? ((callback) => callback(new Blob(['png'], { type: 'image/png' }))),
    };
    const createCanvas = vi.fn(() => canvas);
    const loadImage = vi.fn(async (dataUrl: string) => {
      const image = options.images?.[dataUrl === PNG_ONE ? 0 : 1];
      if (!image) throw new Error('image load failed');
      return {
        width: image.width,
        height: image.height,
        draw: (context: CanvasRenderingContext2D, x: number, y: number) =>
          context.drawImage(image.label as unknown as CanvasImageSource, x, y),
      };
    });
    return { loadImage, createCanvas, canvas, fillRect, drawImage };
  }

  it('composes PNGs vertically on a white canvas with a fixed gap', async () => {
    const adapters = createAdapters({
      images: [
        { width: 100, height: 20, label: 'first' },
        { width: 80, height: 30, label: 'second' },
      ],
    });

    const result = await composeChartPng([PNG_ONE, PNG_TWO], adapters);

    expect(result.width).toBe(100);
    expect(result.height).toBe(66);
    expect(adapters.createCanvas).toHaveBeenCalledWith(100, 66);
    expect(adapters.fillRect).toHaveBeenCalledWith(0, 0, 100, 66);
    expect(adapters.drawImage).toHaveBeenNthCalledWith(1, 'first', 0, 0);
    expect(adapters.drawImage).toHaveBeenNthCalledWith(2, 'second', 0, 36);
    expect(result.blob.type).toBe('image/png');
  });

  it('rejects an empty list of chart images', async () => {
    await expect(composeChartPng([], createAdapters({}))).rejects.toThrow(
      'Cannot compose chart PNG: no images provided',
    );
  });

  it('rejects a non-PNG chart data URL', async () => {
    await expect(
      composeChartPng(['data:image/svg+xml,%3Csvg%2F%3E'], createAdapters({})),
    ).rejects.toThrow('composeChartPng expects PNG data URLs');
  });

  it('reports image loading failures clearly', async () => {
    const adapters = createAdapters({ images: [] });

    await expect(composeChartPng([PNG_ONE], adapters)).rejects.toThrow(
      'Failed to load chart image',
    );
  });

  it('rejects when canvas.toBlob returns null', async () => {
    const adapters = createAdapters({
      images: [{ width: 10, height: 10, label: 'only' }],
      toBlob: (callback) => callback(null),
    });

    await expect(composeChartPng([PNG_ONE], adapters)).rejects.toThrow(
      'Canvas toBlob returned null',
    );
  });
});

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
