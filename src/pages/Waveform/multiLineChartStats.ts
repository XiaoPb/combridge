import { LINE_COLORS } from './multiLineChartModel';

const MAX_LINES_PER_CHART = 4;

export interface DataZoomPercent {
  start: number;
  end: number;
}

export interface VisibleRowRange {
  startIndex: number;
  endIndex: number;
}

export interface LineStatistics {
  name: string;
  color: string;
  max: number | null;
  min: number | null;
  avg: number | null;
  diff: number | null;
}

export function getVisibleRowRange(
  rowCount: number,
  zoom: DataZoomPercent,
): VisibleRowRange | null {
  if (!Number.isInteger(rowCount) || rowCount <= 0) return null;
  if (!Number.isFinite(zoom.start) || !Number.isFinite(zoom.end)) {
    return { startIndex: 0, endIndex: rowCount - 1 };
  }

  const startPercent = Math.max(0, Math.min(100, Math.min(zoom.start, zoom.end)));
  const endPercent = Math.max(0, Math.min(100, Math.max(zoom.start, zoom.end)));
  return {
    startIndex: Math.round((startPercent * (rowCount - 1)) / 100),
    endIndex: Math.round((endPercent * (rowCount - 1)) / 100),
  };
}

export function calculateVisibleLineStats(
  columns: string[],
  rows: number[][],
  groupColumns: string[],
  zoom: DataZoomPercent,
  maxLines = 4,
): LineStatistics[] {
  const range = getVisibleRowRange(rows.length, zoom);
  if (!range) return [];

  const lineLimit = Math.max(0, Math.min(maxLines, MAX_LINES_PER_CHART));
  const selectedColumns = [...new Set(groupColumns)].slice(0, lineLimit);

  return selectedColumns.map((name, lineIndex) => {
    const columnIndex = columns.indexOf(name);
    const values = rows
      .slice(range.startIndex, range.endIndex + 1)
      .map((row) => row[columnIndex])
      .filter((value): value is number => Number.isFinite(value));
    const color = LINE_COLORS[lineIndex % LINE_COLORS.length];

    if (values.length === 0) {
      return { name, color, max: null, min: null, avg: null, diff: null };
    }

    const max = Math.max(...values);
    const min = Math.min(...values);
    const avg = values.reduce((sum, value) => sum + value, 0) / values.length;
    return { name, color, max, min, avg, diff: max - min };
  });
}
