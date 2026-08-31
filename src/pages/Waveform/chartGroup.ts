export interface ChartGroupConfig {
  id?: string;
  name: string;
  columns: string[];
  height?: number;
}

export type IdentifiedChartGroupConfig = ChartGroupConfig & { id: string };

let fallbackIdCounter = 0;

function createChartGroupId(): string {
  const randomUUID = globalThis.crypto?.randomUUID;
  if (typeof randomUUID === 'function') {
    return randomUUID.call(globalThis.crypto);
  }

  fallbackIdCounter += 1;
  return `chart-${Date.now()}-${fallbackIdCounter}`;
}

export function createChartGroup(
  name: string,
  columns: string[] = [],
  height = 300,
): IdentifiedChartGroupConfig {
  return {
    id: createChartGroupId(),
    name,
    columns: [...columns],
    height,
  };
}

export function getChartGroupKey(group: ChartGroupConfig, index: number): string {
  return group.id || `legacy:${index}:${group.name}`;
}

export function getNextChartGroupName(groups: readonly ChartGroupConfig[]): string {
  const existing = new Set(groups.map((group) => group.name));
  let index = 1;
  while (existing.has(`图表${index}`)) {
    index += 1;
  }
  return `图表${index}`;
}
