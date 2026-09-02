import {
  LINE_COLORS,
  MAX_LINES_PER_CHART,
  normalizeMaxLines,
} from './multiLineChartModel';

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
    startIndex: Math.floor((startPercent * (rowCount - 1)) / 100),
    endIndex: Math.ceil((endPercent * (rowCount - 1)) / 100),
  };
}

export function calculateVisibleLineStats(
  columns: string[],
  rows: number[][],
  groupColumns: string[],
  zoom: DataZoomPercent,
  maxLines = MAX_LINES_PER_CHART,
): LineStatistics[] {
  const range = getVisibleRowRange(rows.length, zoom);
  if (!range) return [];

  const lineLimit = normalizeMaxLines(maxLines);
  const selectedColumns = groupColumns
    .slice(0, lineLimit)
    .filter((column, index, selected) => selected.indexOf(column) === index);

  return selectedColumns.map((name, lineIndex) => {
    const columnIndex = columns.indexOf(name);
    const color = LINE_COLORS[lineIndex % LINE_COLORS.length];
    let max: number | null = null;
    let min: number | null = null;
    const values: number[] = [];

    for (let rowIndex = range.startIndex; rowIndex <= range.endIndex; rowIndex += 1) {
      const value = rows[rowIndex]?.[columnIndex];
      if (!Number.isFinite(value)) continue;

      values.push(value);
      if (max === null || value > max) max = value;
      if (min === null || value < min) min = value;
    }

    if (values.length === 0 || max === null || min === null) {
      return { name, color, max: null, min: null, avg: null, diff: null };
    }

    const sortedValues = [...values].sort((a, b) => Math.abs(b) - Math.abs(a));
    const scale = Math.abs(sortedValues[0]);
    let mean = 0;
    if (scale !== 0) {
      sortedValues.forEach((value, index) => {
        const count = index + 1;
        const normalizedValue = value / scale;
        mean = mean + (normalizedValue - mean) / count;
      });
    }

    return { name, color, max, min, avg: mean * scale, diff: max - min };
  });
}
