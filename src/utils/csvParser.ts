export interface CsvParseConfig {
  skipFirstRow: boolean;
  useSecondRowAsHeader: boolean;
  splitColumn: boolean;
  splitColumnIndex: number;
}

export interface CsvParseResult {
  columns: string[];
  rows: number[][];
}

const DEFAULT_CONFIG: CsvParseConfig = {
  skipFirstRow: false,
  useSecondRowAsHeader: false,
  splitColumn: false,
  splitColumnIndex: 0,
};

export function parseCsv(csvContent: string, config: Partial<CsvParseConfig> = {}): CsvParseResult {
  const cfg: CsvParseConfig = { ...DEFAULT_CONFIG, ...config };
  const lines = csvContent.split(/\r?\n/).filter(line => line.trim().length > 0);

  if (lines.length === 0) {
    return { columns: [], rows: [] };
  }

  let headerLineIndex = 0;
  let dataStartIndex = 0;

  if (cfg.skipFirstRow) {
    dataStartIndex = 1;
    if (cfg.useSecondRowAsHeader) {
      headerLineIndex = 1;
      dataStartIndex = 2;
    }
  } else if (cfg.useSecondRowAsHeader) {
    headerLineIndex = 1;
    dataStartIndex = 2;
  }

  const headerLine = lines[headerLineIndex] || '';
  const rawColumns = headerLine.split(',').map(col => col.trim());

  const dataLines = lines.slice(dataStartIndex);
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
