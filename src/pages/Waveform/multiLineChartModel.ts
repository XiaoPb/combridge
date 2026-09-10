export const LINE_COLORS = [
  '#165DFF',
  '#F53F3F',
  '#00B42A',
  '#FF7D00',
  '#722ED1',
  '#14C9C9',
  '#EB0AA4',
  '#FFC107',
  '#40A9FF',
];

export const MAX_LINES_PER_CHART = 4;
export const MAX_LINE_OFFSET_POINTS = 1_000_000;

export function normalizeLineOffset(offset: number): number {
  if (!Number.isFinite(offset)) return 0;
  return Math.max(-MAX_LINE_OFFSET_POINTS, Math.min(MAX_LINE_OFFSET_POINTS, Math.round(offset)));
}

export function applyLineOffset(data: readonly number[], offset: number): number[] {
  const normalized = normalizeLineOffset(offset);
  const length = data.length;
  if (length === 0 || Math.abs(normalized) >= length) return Array(length).fill(0);
  if (normalized > 0) return Array(normalized).fill(0).concat(data.slice(0, length - normalized));
  if (normalized < 0) return data.slice(-normalized).concat(Array(-normalized).fill(0));
  return [...data];
}

export function normalizeMaxLines(maxLines: number): number {
  if (!Number.isFinite(maxLines)) return MAX_LINES_PER_CHART;
  return Math.max(0, Math.min(Math.floor(maxLines), MAX_LINES_PER_CHART));
}

export interface ChartSeriesData {
  name: string;
  data: number[];
  color: string;
}

export function buildChartSeries(
  columns: string[],
  rows: number[][],
  groupColumns: string[],
  maxLines = MAX_LINES_PER_CHART,
  lineOffsets?: Readonly<Record<string, number>>,
): ChartSeriesData[] {
  const selectedColumns = groupColumns
    .slice(0, normalizeMaxLines(maxLines))
    .filter((column, index, selected) => selected.indexOf(column) === index);

  return selectedColumns.map((name, lineIndex) => {
    const columnIndex = columns.indexOf(name);
    return {
      name,
      data: applyLineOffset(rows.map((row) => {
        return columnIndex >= 0 && columnIndex < row.length
          ? (row[columnIndex] ?? 0)
          : 0;
      }), lineOffsets?.[name] ?? 0),
      color: LINE_COLORS[lineIndex % LINE_COLORS.length],
    };
  });
}
