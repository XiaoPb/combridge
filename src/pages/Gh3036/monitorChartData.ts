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
  const maxPoints = Math.max(1, Math.floor(displayDurationSeconds * sampleRate));
  const availablePoints = Math.min(
    currentFrames.frame_count,
    ...source.slice(0, currentFrames.channel_count).map((channel) => channel?.length ?? 0)
  );
  const startIndex = Math.max(0, availablePoints - maxPoints);

  const rows: number[][] = [];
  for (let frameIdx = startIndex; frameIdx < availablePoints; frameIdx++) {
    const row: number[] = [];
    for (let chIdx = 0; chIdx < currentFrames.channel_count; chIdx++) {
      row.push(source[chIdx]?.[frameIdx] ?? 0);
    }
    rows.push(row);
  }

  return { columns, rows };
}
