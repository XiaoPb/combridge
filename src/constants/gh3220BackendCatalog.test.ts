import { beforeEach, describe, expect, it, vi } from 'vitest';
import { gh3220Api } from '../api/gh3220';
import { GH3220_COMMAND_CODES, GH3220_VERSION_TYPES } from './gh3220Functions';
import { loadBackendCatalog } from './gh3220BackendCatalog';

vi.mock('../api/gh3220', () => ({
  gh3220Api: {
    getCommandCodes: vi.fn(),
    getVersionTypes: vi.fn(),
  },
}));

const codes = Object.entries(GH3220_COMMAND_CODES).map(([key, cmd]) => ({ key, cmd }));
const types = GH3220_VERSION_TYPES.map((item) => ({
  key: item.key,
  value: Number(item.value),
  nameZh: item.nameZh,
  nameEn: item.nameEn,
}));

describe('GH3220 后端目录（方案 B）', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('后端可用时采用后端目录且与前端常量一致', async () => {
    vi.mocked(gh3220Api.getCommandCodes).mockResolvedValue(codes);
    vi.mocked(gh3220Api.getVersionTypes).mockResolvedValue(types);
    const snapshot = await loadBackendCatalog();
    expect(snapshot.source).toBe('backend');
    expect(snapshot.commandCodes).toHaveLength(20);
    expect(snapshot.versionTypes).toHaveLength(9);
    expect(snapshot.mismatches).toEqual([]);
  });

  it('旧二进制没有目录命令时静默回退前端常量', async () => {
    vi.mocked(gh3220Api.getCommandCodes).mockRejectedValue(new Error('unknown command'));
    vi.mocked(gh3220Api.getVersionTypes).mockResolvedValue(types);
    const snapshot = await loadBackendCatalog();
    expect(snapshot.source).toBe('frontend');
    expect(snapshot.commandCodes).toHaveLength(20);
    expect(snapshot.versionTypes).toHaveLength(9);
    expect(snapshot.mismatches).toEqual([]);
  });

  it('后端目录缺项时报告漂移但不抛错', async () => {
    vi.mocked(gh3220Api.getCommandCodes).mockResolvedValue(codes.slice(0, -1));
    vi.mocked(gh3220Api.getVersionTypes).mockResolvedValue(types);
    const snapshot = await loadBackendCatalog();
    expect(snapshot.source).toBe('backend');
    expect(snapshot.mismatches.some((text) => text.includes('CMD 后端缺少'))).toBe(true);
  });
});
