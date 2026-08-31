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

export function getChartGroupKey(
  group: ChartGroupConfig,
  index: number,
): string {
  return group.id || `legacy:${index}:${group.name}`;
}

export function getChartLegendKey(
  scope: string,
  group: ChartGroupConfig,
  index: number,
  column: string,
): string {
  return `${scope}:${getChartGroupKey(group, index)}:${column}`;
}

export function resolveChartLegendSelection(
  scope: string,
  selected: Record<string, boolean> | undefined,
  group: ChartGroupConfig,
  index: number,
  column: string,
): boolean | undefined {
  if (!selected) return undefined;
  const scopedKey = getChartLegendKey(scope, group, index, column);
  if (Object.prototype.hasOwnProperty.call(selected, scopedKey))
    return selected[scopedKey];
  return undefined;
}

export function migrateLegacyChartLegendSelections(
  scope: string,
  selected: Record<string, boolean>,
  chartGroups: readonly ChartGroupConfig[],
): Record<string, boolean> {
  let migrated: Record<string, boolean> | undefined;

  chartGroups.forEach((group, index) => {
    group.columns.forEach((column) => {
      const scopedKey = getChartLegendKey(scope, group, index, column);
      const legacyKey = `${group.name}_${column}`;
      if (
        Object.prototype.hasOwnProperty.call(selected, scopedKey) ||
        !Object.prototype.hasOwnProperty.call(selected, legacyKey)
      ) {
        return;
      }

      migrated ??= { ...selected };
      migrated[scopedKey] = selected[legacyKey];
    });
  });

  return migrated ?? selected;
}

export function getNextChartGroupName(
  groups: readonly ChartGroupConfig[],
): string {
  const existing = new Set(groups.map((group) => group.name));
  let index = 1;
  while (existing.has(`图表${index}`)) {
    index += 1;
  }
  return `图表${index}`;
}
