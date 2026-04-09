export interface CsvParseConfig {
  skipInfoRows: number;
  noHeader: boolean;
}

export interface CsvParseResult {
  columns: string[];
  rows: number[][];
}

const DEFAULT_CONFIG: CsvParseConfig = {
  skipInfoRows: 1,
  noHeader: false,
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

  return {
    columns: rawColumns,
    rows: rawData,
  };
}

export async function readCsvFile(filePath: string, config: Partial<CsvParseConfig> = {}): Promise<CsvParseResult> {
  const { readTextFile } = await import('@tauri-apps/plugin-fs');
  const content = await readTextFile(filePath);
  return parseCsv(content, config);
}
