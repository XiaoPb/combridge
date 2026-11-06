import type { Gh3220FrameBuffer, Gh3220DataMode } from '../../stores/gh3220Store';

export interface ChartTableData {
  columns: string[];
  rows: number[][];
}

/**
 * 从波形缓冲构建 MultiLineChart 数据：
 * 主数据源为 RAW 或 AGC 通道，附加 ACC 三轴（若存在），
 * 多源按最短行数对齐，不补 0，避免绘制虚假恒定平线。
 */
export function buildWaveformChartData(
  buffer: Gh3220FrameBuffer | null,
  mode: Gh3220DataMode
): ChartTableData {
  if (!buffer) {
    return { columns: [], rows: [] };
  }

  const mainRows = mode === 'agc' ? buffer.agcRows : buffer.rawRows;
  const sources: ChartTableData[] = [];

  if (mainRows.length > 0 && buffer.channelCount > 0) {
    const prefix = mode === 'agc' ? 'AGC' : 'RAW';
    sources.push({
      columns: Array.from({ length: buffer.channelCount }, (_, i) => `${prefix}${i}`),
      rows: mainRows,
    });
  }

  if (buffer.accRows.length > 0) {
    sources.push({
      columns: ['ACC_X', 'ACC_Y', 'ACC_Z'],
      rows: buffer.accRows,
    });
  }

  if (sources.length === 0) {
    return { columns: [], rows: [] };
  }

  const columns: string[] = [];
  for (const source of sources) {
    columns.push(...source.columns);
  }

  const alignedRows = Math.min(...sources.map((source) => source.rows.length));
  const rows: number[][] = [];
  for (let i = 0; i < alignedRows; i++) {
    const row: number[] = [];
    for (const source of sources) {
      row.push(...source.rows[i]);
    }
    rows.push(row);
  }

  return { columns, rows };
}

/** 图表分组：主通道一组、ACC 一组（供 MultiLineChart chartGroups 使用） */
export function buildChartGroups(
  buffer: Gh3220FrameBuffer | null,
  mode: Gh3220DataMode,
  labels: { mainGroup: string; accGroup: string }
): Array<{ id: string; name: string; columns: string[]; height: number }> {
  if (!buffer) return [];
  const groups: Array<{ id: string; name: string; columns: string[]; height: number }> = [];

  const mainRows = mode === 'agc' ? buffer.agcRows : buffer.rawRows;
  if (mainRows.length > 0 && buffer.channelCount > 0) {
    const prefix = mode === 'agc' ? 'AGC' : 'RAW';
    groups.push({
      id: `gh3220-${mode}`,
      name: labels.mainGroup,
      columns: Array.from({ length: buffer.channelCount }, (_, i) => `${prefix}${i}`),
      height: 250,
    });
  }
  if (buffer.accRows.length > 0) {
    groups.push({
      id: 'gh3220-acc',
      name: labels.accGroup,
      columns: ['ACC_X', 'ACC_Y', 'ACC_Z'],
      height: 200,
    });
  }
  return groups;
}
