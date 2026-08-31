import type {
  ChartGroupConfig,
  IdentifiedChartGroupConfig,
} from '../Waveform/chartGroup';

const ID_PREFIX = 'gh3036-data-';

function allocateId(usedIds: Set<string>): string {
  let index = 0;
  let id = `${ID_PREFIX}${index}`;
  while (usedIds.has(id)) {
    index += 1;
    id = `${ID_PREFIX}${index}`;
  }
  usedIds.add(id);
  return id;
}

export function normalizeGh3036ChartGroups(
  groups: readonly ChartGroupConfig[],
): IdentifiedChartGroupConfig[] {
  const usedIds = new Set(
    groups.flatMap((group) => (group.id ? [group.id] : [])),
  );
  const assignedIds = new Set<string>();

  return groups.map((group) => {
    const id =
      group.id && !assignedIds.has(group.id) ? group.id : allocateId(usedIds);
    assignedIds.add(id);
    return { ...group, id, columns: [...group.columns] };
  });
}

export function appendGh3036ChartGroup(
  groups: readonly ChartGroupConfig[],
  name: string,
  columns: string[] = [],
): IdentifiedChartGroupConfig[] {
  const normalized = normalizeGh3036ChartGroups(groups);
  const usedIds = new Set(normalized.map((group) => group.id));
  const id = allocateId(usedIds);
  return [...normalized, { id, name, columns: [...columns] }];
}
