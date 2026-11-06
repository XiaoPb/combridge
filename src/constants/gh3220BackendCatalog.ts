/**
 * §6.6 方案 B 的前端适配层：优先从 Rust 后端拉取「协议命令号 / 版本类型」目录，
 * 旧版本二进制（未注册 `gh3220_get_command_codes` / `gh3220_get_version_types`）
 * 或后端异常时静默回退到常量层目录，保证 UI 永远可用。
 *
 * 前端仍是方案 A 的 22 条命令元数据；后端目录只用于：
 * 1. 校验 CMD 号是否与前端常量漂移（mismatches）；
 * 2. 在命令控制台显示目录来源，便于跨端一致性排查。
 */

import { gh3220Api } from '../api/gh3220';
import type { Gh3220CommandCodeDto, Gh3220NamedValueDto } from '../types/gh3220';
import { GH3220_COMMAND_CODES, GH3220_VERSION_TYPES } from './gh3220Functions';

export interface Gh3220CatalogSnapshot {
  source: 'backend' | 'frontend';
  commandCodes: Gh3220CommandCodeDto[];
  versionTypes: Gh3220NamedValueDto[];
  /** 后端目录与前端常量不一致的 CMD / 版本值，空数组表示一致 */
  mismatches: string[];
}

const frontendCommandCodes: Gh3220CommandCodeDto[] = Object.entries(GH3220_COMMAND_CODES).map(
  ([key, cmd]) => ({ key, cmd })
);

const frontendVersionTypes: Gh3220NamedValueDto[] = GH3220_VERSION_TYPES.map((item) => ({
  key: item.key,
  value: Number(item.value),
  nameZh: item.nameZh,
  nameEn: item.nameEn,
}));

/** 只比对数值集合，避免前后端 key 命名风格不同造成误报 */
function diffValues(frontend: number[], backend: number[], label: string): string[] {
  const missing = frontend.filter((value) => !backend.includes(value));
  const extra = backend.filter((value) => !frontend.includes(value));
  const out: string[] = [];
  if (missing.length) out.push(`${label} 后端缺少 0x${missing.map((v) => v.toString(16)).join(',0x')}`);
  if (extra.length) out.push(`${label} 后端多出 0x${extra.map((v) => v.toString(16)).join(',0x')}`);
  return out;
}

/** 拉取一次后端目录；任一条失败即整体回退前端常量（不抛错、不阻塞 UI） */
export async function loadBackendCatalog(): Promise<Gh3220CatalogSnapshot> {
  const [codes, types] = await Promise.allSettled([
    gh3220Api.getCommandCodes(),
    gh3220Api.getVersionTypes(),
  ]);

  if (codes.status !== 'fulfilled' || types.status !== 'fulfilled') {
    return {
      source: 'frontend',
      commandCodes: frontendCommandCodes,
      versionTypes: frontendVersionTypes,
      mismatches: [],
    };
  }

  const commandCodes = codes.value;
  const versionTypes = types.value;
  return {
    source: 'backend',
    commandCodes,
    versionTypes,
    mismatches: [
      ...diffValues(
        frontendCommandCodes.map((item) => item.cmd),
        commandCodes.map((item) => item.cmd),
        'CMD'
      ),
      ...diffValues(
        frontendVersionTypes.map((item) => item.value),
        versionTypes.map((item) => item.value),
        'ver_type'
      ),
    ],
  };
}
