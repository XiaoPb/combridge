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

export interface ChartSeriesData {
  name: string;
  data: number[];
  color: string;
}

export function buildChartSeries(
  columns: string[],
  rows: number[][],
  groupColumns: string[],
  maxLines = 4,
): ChartSeriesData[] {
  const selectedColumns = groupColumns.slice(0, maxLines).filter((column, index, selected) =>
    selected.indexOf(column) === index
  );

  return selectedColumns.map((name, lineIndex) => {
    const columnIndex = columns.indexOf(name);
    return {
      name,
      data: rows.map((row) => columnIndex >= 0 && columnIndex < row.length ? row[columnIndex] : 0),
      color: LINE_COLORS[lineIndex % LINE_COLORS.length],
    };
  });
}
