import type {
  ChartGroupConfig,
  IdentifiedChartGroupConfig,
} from '../Waveform/chartGroup';

const ID_PREFIX = 'gh3036-data-';
let nextGeneratedId = 0;

function reserveExistingIds(ids: Iterable<string>): void {
  for (const id of ids) {
    const match = id.match(/^gh3036-data-(\d+)$/);
    if (!match) continue;
    const index = Number(match[1]);
    if (Number.isSafeInteger(index)) {
      nextGeneratedId = Math.max(nextGeneratedId, index + 1);
    }
  }
}

function allocateId(usedIds: Set<string>): string {
  let id = `${ID_PREFIX}${nextGeneratedId}`;
  while (usedIds.has(id)) {
    nextGeneratedId += 1;
    id = `${ID_PREFIX}${nextGeneratedId}`;
  }
  nextGeneratedId += 1;
  usedIds.add(id);
  return id;
}

export function normalizeGh3036ChartGroups(
  groups: readonly ChartGroupConfig[],
): IdentifiedChartGroupConfig[] {
  const usedIds = new Set(
    groups.flatMap((group) => (group.id ? [group.id] : [])),
  );
  reserveExistingIds(usedIds);
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
