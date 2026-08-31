const SVG_DATA_URL_PREFIX = 'data:image/svg+xml';

export function normalizeSvgExportDataUrl(value: string): string {
  const trimmed = value.trim();
  if (trimmed.startsWith(SVG_DATA_URL_PREFIX)) return trimmed;

  if (/<svg(?:\s|>)/i.test(trimmed)) {
    return `${SVG_DATA_URL_PREFIX};charset=UTF-8,${encodeURIComponent(trimmed)}`;
  }

  throw new Error('ECharts returned a non-SVG export');
}
