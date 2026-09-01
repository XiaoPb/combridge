const SVG_DATA_URL_PREFIX = 'data:image/svg+xml';
const PNG_MIME_TYPE = 'image/png';
const SVG_MIME_TYPE = 'image/svg+xml';

export const CHART_PNG_GAP = 16;

export interface ChartPngRenderingContext {
  fillStyle: string | CanvasGradient | CanvasPattern;
  fillRect(x: number, y: number, width: number, height: number): void;
  drawImage(image: CanvasImageSource, dx: number, dy: number): void;
}

export interface ChartPngCanvas {
  width: number;
  height: number;
  getContext(contextId: '2d'): ChartPngRenderingContext | null;
  toBlob(callback: BlobCallback, type?: string): void;
}

export interface ChartPngImage {
  width: number;
  height: number;
  draw(context: ChartPngRenderingContext, x: number, y: number): void;
}

export interface ChartPngAdapters {
  gap?: number;
  pixelRatio?: number;
  loadImage?: (dataUrl: string) => Promise<ChartPngImage>;
  createCanvas?: (width: number, height: number) => ChartPngCanvas;
}

export interface ComposedChartPng {
  blob: Blob;
  width: number;
  height: number;
}

export function dataUrlToBlob(value: string): Blob {
  const trimmed = value.trim();
  const separator = trimmed.indexOf(',');
  if (!trimmed.startsWith('data:') || separator < 0)
    throw new Error('Expected a PNG or SVG data URL');

  const metadata = trimmed.slice(5, separator);
  const [rawMimeType, ...parameters] = metadata.split(';');
  const mimeType = rawMimeType.toLowerCase();
  if (mimeType !== PNG_MIME_TYPE && mimeType !== SVG_MIME_TYPE)
    throw new Error('Expected a PNG or SVG data URL');

  const payload = trimmed.slice(separator + 1);
  const isBase64 = parameters.some(
    (parameter) => parameter.trim().toLowerCase() === 'base64',
  );

  try {
    if (isBase64) {
      const binary = atob(decodeURIComponent(payload));
      const bytes = Uint8Array.from(binary, (character) => character.charCodeAt(0));
      return new Blob([bytes], { type: mimeType });
    }

    return new Blob([decodeURIComponent(payload)], { type: mimeType });
  } catch {
    throw new Error('Invalid image data URL payload');
  }
}

export function downloadBlob(blob: Blob, filename: string): void {
  const objectUrl = URL.createObjectURL(blob);
  let link: HTMLAnchorElement | null = null;
  try {
    link = document.createElement('a');
    link.href = objectUrl;
    link.download = filename;
    document.body.appendChild(link);
    link.click();
  } finally {
    link?.remove();
    URL.revokeObjectURL(objectUrl);
  }
}

function loadImage(dataUrl: string): Promise<ChartPngImage> {
  return new Promise((resolve, reject) => {
    if (typeof Image === 'undefined') {
      reject(new Error('Image is unavailable'));
      return;
    }

    const image = new Image();
    image.onload = () => {
      const width = image.naturalWidth || image.width;
      const height = image.naturalHeight || image.height;
      resolve({
        width,
        height,
        draw: (context, x, y) => context.drawImage(image, x, y),
      });
    };
    image.onerror = () => reject(new Error('Image failed to load'));
    image.src = dataUrl;
  });
}

function createCanvas(width: number, height: number): ChartPngCanvas {
  const canvas = document.createElement('canvas');
  canvas.width = width;
  canvas.height = height;
  return canvas;
}

export async function composeChartPng(
  dataUrls: readonly string[],
  adapters: ChartPngAdapters = {},
): Promise<ComposedChartPng> {
  if (dataUrls.length === 0)
    throw new Error('Cannot compose chart PNG: no images provided');

  const gap = adapters.gap ?? CHART_PNG_GAP;
  const pixelRatio = adapters.pixelRatio ?? 1;
  if (!Number.isFinite(gap) || gap < 0)
    throw new Error('Chart PNG gap must be a non-negative number');
  if (!Number.isFinite(pixelRatio) || pixelRatio <= 0)
    throw new Error('Chart PNG pixel ratio must be positive');

  const loader = adapters.loadImage ?? loadImage;
  const images = await Promise.all(
    dataUrls.map(async (dataUrl) => {
      const blob = dataUrlToBlob(dataUrl);
      if (blob.type !== PNG_MIME_TYPE)
        throw new Error('composeChartPng expects PNG data URLs');

      try {
        return await loader(dataUrl);
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        throw new Error(`Failed to load chart image: ${message}`);
      }
    }),
  );

  if (images.some((image) =>
    !Number.isFinite(image.width) ||
    !Number.isFinite(image.height) ||
    image.width <= 0 ||
    image.height <= 0
  ))
    throw new Error('Chart image dimensions must be positive');

  const width = Math.max(...images.map((image) => image.width));
  const height = images.reduce((total, image) => total + image.height, 0)
    + gap * (images.length - 1);
  const canvas = (adapters.createCanvas ?? createCanvas)(width, height);
  canvas.width = width;
  canvas.height = height;
  const context = canvas.getContext('2d');
  if (!context) throw new Error('Could not get 2D canvas context');

  context.fillStyle = '#fff';
  context.fillRect(0, 0, width, height);
  let y = 0;
  images.forEach((image) => {
    image.draw(context, 0, y);
    y += image.height + gap;
  });

  const blob = await new Promise<Blob>((resolve, reject) => {
    canvas.toBlob((result) => {
      if (result === null) reject(new Error('Canvas toBlob returned null'));
      else resolve(result);
    }, PNG_MIME_TYPE);
  });

  return { blob, width, height };
}

export type CssVariableResolver = (name: string) => string | undefined;

interface ParsedCssVariable {
  name: string;
  fallback?: string;
  end: number;
}

function isPlainObject(value: object): value is Record<string, unknown> {
  const prototype = Object.getPrototypeOf(value);
  return prototype === Object.prototype || prototype === null;
}

function findTopLevelComma(value: string): number {
  let depth = 0;
  for (let index = 0; index < value.length; index += 1) {
    const character = value[index];
    if (character === '(') depth += 1;
    else if (character === ')') depth = Math.max(0, depth - 1);
    else if (character === ',' && depth === 0) return index;
  }
  return -1;
}

function parseCssVariableAt(value: string, start: number): ParsedCssVariable | null {
  if (!value.startsWith('var(', start)) return null;

  let depth = 0;
  let end = -1;
  for (let index = start + 4; index < value.length; index += 1) {
    const character = value[index];
    if (character === '(') depth += 1;
    else if (character === ')') {
      if (depth === 0) {
        end = index;
        break;
      }
      depth -= 1;
    }
  }
  if (end < 0) return null;

  const contents = value.slice(start + 4, end);
  const comma = findTopLevelComma(contents);
  const name = (comma < 0 ? contents : contents.slice(0, comma)).trim();
  if (!/^--[\w-]+$/.test(name)) return null;

  return {
    name,
    fallback: comma < 0 ? undefined : contents.slice(comma + 1).trim(),
    end,
  };
}

function resolveCssVariablesInString(
  value: string,
  resolver: CssVariableResolver,
  resolving: ReadonlySet<string> = new Set(),
): string {
  let result = '';
  let cursor = 0;
  let changed = false;

  while (cursor < value.length) {
    const start = value.indexOf('var(', cursor);
    if (start < 0) {
      result += value.slice(cursor);
      break;
    }

    result += value.slice(cursor, start);
    const parsed = parseCssVariableAt(value, start);
    if (!parsed) {
      result += value.slice(start, start + 4);
      cursor = start + 4;
      continue;
    }

    const originalToken = value.slice(start, parsed.end + 1);
    const resolved = resolving.has(parsed.name)
      ? undefined
      : resolver(parsed.name);
    const replacement = resolved?.trim()
      ? resolved
      : parsed.fallback?.trim()
        ? parsed.fallback
        : undefined;

    if (replacement === undefined) result += originalToken;
    else {
      result += resolveCssVariablesInString(
        replacement,
        resolver,
        new Set([...resolving, parsed.name]),
      );
      changed = true;
    }
    cursor = parsed.end + 1;
  }

  return changed ? result : value;
}

/**
 * Deep-clones arrays and plain objects while replacing CSS var() tokens in
 * string leaves. Missing variables without a fallback are intentionally kept
 * unchanged so callers can choose a safe fallback at the option root.
 */
export function resolveCssVariablesInValue<T>(
  value: T,
  resolver: CssVariableResolver,
): T {
  const seen = new WeakMap<object, unknown>();

  const clone = (current: unknown): unknown => {
    if (typeof current === 'string')
      return resolveCssVariablesInString(current, resolver);
    if (current === null || typeof current !== 'object') return current;
    if (!Array.isArray(current) && !isPlainObject(current)) return current;

    const previous = seen.get(current);
    if (previous !== undefined) return previous;

    if (Array.isArray(current)) {
      const cloned: unknown[] = [];
      seen.set(current, cloned);
      for (let index = 0; index < current.length; index += 1) {
        if (index in current) cloned[index] = clone(current[index]);
      }
      return cloned;
    }

    const cloned = Object.create(Object.getPrototypeOf(current)) as Record<
      string,
      unknown
    >;
    seen.set(current, cloned);
    Object.keys(current).forEach((key) => {
      cloned[key] = clone(current[key]);
    });
    return cloned;
  };

  return clone(value) as T;
}

export function normalizeSvgExportDataUrl(value: string): string {
  const trimmed = value.trim();
  if (trimmed.startsWith(SVG_DATA_URL_PREFIX)) return trimmed;

  if (/<svg(?:\s|>)/i.test(trimmed)) {
    return `${SVG_DATA_URL_PREFIX};charset=UTF-8,${encodeURIComponent(trimmed)}`;
  }

  throw new Error('ECharts returned a non-SVG export');
}

function decodeSvgDataUrlPayload(normalized: string): string | null {
  const separator = normalized.indexOf(',');
  if (separator < 0) return null;

  const metadata = normalized.slice(0, separator);
  const payload = normalized.slice(separator + 1);
  const isBase64 = metadata
    .slice(SVG_DATA_URL_PREFIX.length)
    .split(';')
    .some((token) => token.trim().toLowerCase() === 'base64');

  try {
    if (!isBase64) return decodeURIComponent(payload);

    const binary = atob(decodeURIComponent(payload));
    const bytes = Uint8Array.from(binary, (character) => character.charCodeAt(0));
    return new TextDecoder().decode(bytes);
  } catch {
    return null;
  }
}

/**
 * Ensures an SVG export has a serialized white background element. ECharts
 * normally emits one for an option-level backgroundColor, so this is only a
 * small fallback for renderer versions that omit it.
 */
export function ensureWhiteSvgBackground(
  value: string,
  width: number,
  height: number,
): string {
  const normalized = normalizeSvgExportDataUrl(value);
  const svg = decodeSvgDataUrlPayload(normalized);
  if (svg === null) return normalized;

  if (
    /<rect\b[^>]*\bfill\s*=\s*["']#(?:fff|ffffff)["']/i.test(svg) ||
    /background(?:-color)?\s*:\s*#(?:fff|ffffff)\b/i.test(svg)
  )
    return normalized;

  const background = `<rect x="0" y="0" width="${width}" height="${height}" fill="#fff"/>`;
  const expandedSelfClosingRoot = svg.replace(
    /<svg\b([^>]*)\/>/i,
    `<svg$1>${background}</svg>`,
  );
  if (expandedSelfClosingRoot !== svg)
    return `${SVG_DATA_URL_PREFIX};charset=UTF-8,${encodeURIComponent(expandedSelfClosingRoot)}`;

  const withBackground = svg.replace(
    /(<svg\b[^>]*>)/i,
    `$1${background}`,
  );
  return withBackground === svg
    ? normalized
    : `${SVG_DATA_URL_PREFIX};charset=UTF-8,${encodeURIComponent(withBackground)}`;
}
