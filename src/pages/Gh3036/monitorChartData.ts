import type { Gh3036FramesPayload } from '../../api/events';

export type IpdRawDataType = 'ipd' | 'rawdata';

export interface ChartTableData {
  columns: string[];
  rows: number[][];
}

export function buildIpdPaChartData(
  currentFrames: Gh3036FramesPayload | null,
  sampleRate: number,
  ipdRawDataType: IpdRawDataType,
  displayDurationSeconds: number
): ChartTableData {
  if (!currentFrames || currentFrames.channel_count === 0) {
    return { columns: [], rows: [] };
  }

  const columns = Array.from(
    { length: currentFrames.channel_count },
    (_, index) => `CH${index}`
  );
  const source = ipdRawDataType === 'ipd' ? currentFrames.ipd_pa : currentFrames.rawdata;

  // 验证数据有效性
  if (!source || source.length === 0) {
    console.warn('[buildIpdPaChartData] 无有效数据源');
    return { columns, rows: [] };
  }

  const maxPoints = Math.max(1, Math.floor(displayDurationSeconds * sampleRate));
  const availablePoints = Math.min(
    currentFrames.frame_count,
    ...source.slice(0, currentFrames.channel_count).map((channel) => channel?.length ?? 0)
  );

  if (availablePoints === 0) {
    console.warn('[buildIpdPaChartData] 可用数据点为0');
    return { columns, rows: [] };
  }

  const startIndex = Math.max(0, availablePoints - maxPoints);

  const rows: number[][] = [];
  for (let frameIdx = startIndex; frameIdx < availablePoints; frameIdx++) {
    const row: number[] = [];
    for (let chIdx = 0; chIdx < currentFrames.channel_count; chIdx++) {
      const value = source[chIdx]?.[frameIdx];
      row.push(value !== undefined ? value : 0);
    }
    rows.push(row);
  }

  return { columns, rows };
}
