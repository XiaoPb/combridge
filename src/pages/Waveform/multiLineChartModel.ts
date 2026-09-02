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
): ChartSeriesData[] {
  const selectedColumns = groupColumns
    .slice(0, normalizeMaxLines(maxLines))
    .filter((column, index, selected) => selected.indexOf(column) === index);

  return selectedColumns.map((name, lineIndex) => {
    const columnIndex = columns.indexOf(name);
    return {
      name,
      data: rows.map((row) => columnIndex >= 0 && columnIndex < row.length ? row[columnIndex] ?? 0 : 0),
      color: LINE_COLORS[lineIndex % LINE_COLORS.length],
    };
  });
}
