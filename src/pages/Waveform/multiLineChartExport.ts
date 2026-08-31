const SVG_DATA_URL_PREFIX = 'data:image/svg+xml';

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
  const separator = normalized.indexOf(',');
  if (separator < 0) return normalized;

  const payload = normalized.slice(separator + 1);
  let svg: string;
  try {
    svg = decodeURIComponent(payload);
  } catch {
    return normalized;
  }

  if (
    /<rect\b[^>]*\bfill\s*=\s*["']#(?:fff|ffffff)["']/i.test(svg) ||
    /background(?:-color)?\s*:\s*#(?:fff|ffffff)\b/i.test(svg)
  )
    return normalized;

  const background = `<rect x="0" y="0" width="${width}" height="${height}" fill="#fff"/>`;
  const withBackground = svg.replace(
    /(<svg\b[^>]*>)/i,
    `$1${background}`,
  );
  return withBackground === svg
    ? normalized
    : `${SVG_DATA_URL_PREFIX};charset=UTF-8,${encodeURIComponent(withBackground)}`;
}
