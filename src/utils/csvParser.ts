export interface CsvParseConfig {
  skipInfoRows: number;
  noHeader: boolean;
  splitColumn: boolean;
  splitColumnIndex: number;
}

export interface CsvParseResult {
  columns: string[];
  rows: number[][];
}

const DEFAULT_CONFIG: CsvParseConfig = {
  skipInfoRows: 0,
  noHeader: false,
  splitColumn: false,
  splitColumnIndex: 0,
};

export function parseCsv(csvContent: string, config: Partial<CsvParseConfig> = {}): CsvParseResult {
  const cfg: CsvParseConfig = { ...DEFAULT_CONFIG, ...config };
  const lines = csvContent.split(/\r?\n/).filter(line => line.trim().length > 0);

  if (lines.length === 0) {
    return { columns: [], rows: [] };
  }

  const skipRows = Math.max(0, cfg.skipInfoRows);
  const dataStartIndex = skipRows;
  
  if (dataStartIndex >= lines.length) {
    return { columns: [], rows: [] };
  }

  let rawColumns: string[];
  let dataLines: string[];

  if (cfg.noHeader) {
    const firstDataLine = lines[dataStartIndex] || '';
    const columnCount = firstDataLine.split(',').length;
    rawColumns = Array.from({ length: columnCount }, (_, i) => `CH${i}`);
    dataLines = lines.slice(dataStartIndex);
  } else {
    const headerLine = lines[dataStartIndex] || '';
    rawColumns = headerLine.split(',').map(col => col.trim());
    dataLines = lines.slice(dataStartIndex + 1);
  }

  const rawData: number[][] = [];

  for (const line of dataLines) {
    const values = line.split(',').map(cell => {
      const trimmed = cell.trim();
      const num = parseFloat(trimmed);
      return isNaN(num) ? 0 : num;
    });
    if (values.length > 0) {
      rawData.push(values);
    }
  }

  if (!cfg.splitColumn) {
    return {
      columns: rawColumns,
      rows: rawData,
    };
  }

  const splitIndex = Math.max(0, Math.min(cfg.splitColumnIndex, rawColumns.length > 0 ? rawColumns.length - 1 : 0));

  const splitColumnName = rawColumns[splitIndex] || `Column${splitIndex}`;
  const columns: string[] = [
    ...rawColumns.slice(0, splitIndex),
    `${splitColumnName}_Odd`,
    `${splitColumnName}_Even`,
    ...rawColumns.slice(splitIndex + 1),
  ];

  const rows: number[][] = [];
  const halfLength = Math.floor(rawData.length / 2);

  for (let i = 0; i < halfLength; i++) {
    const oddRow = rawData[i * 2];
    const evenRow = rawData[i * 2 + 1];

    if (oddRow && evenRow) {
      const mergedRow: number[] = [
        ...oddRow.slice(0, splitIndex),
        oddRow[splitIndex] ?? 0,
        evenRow[splitIndex] ?? 0,
        ...oddRow.slice(splitIndex + 1),
      ];
      rows.push(mergedRow);
    }
  }

  return { columns, rows };
}

export async function readCsvFile(filePath: string, config: Partial<CsvParseConfig> = {}): Promise<CsvParseResult> {
  const { readTextFile } = await import('@tauri-apps/plugin-fs');
  const content = await readTextFile(filePath);
  return parseCsv(content, config);
}
