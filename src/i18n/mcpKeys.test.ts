import { describe, expect, it } from 'vitest';
import i18n from './index';

/** 展平成 `a.b.c` 形式的 key 列表（只看叶子字符串） */
const flatten = (node: unknown, prefix = ''): string[] => {
  if (typeof node === 'string') return [prefix];
  if (!node || typeof node !== 'object') return [];
  return Object.entries(node as Record<string, unknown>).flatMap(([key, value]) =>
    flatten(value, prefix ? `${prefix}.${key}` : key)
  );
};

describe('MCP integration panel translations', () => {
  const zh = i18n.getResourceBundle('zh-CN', 'system') as Record<string, unknown>;
  const en = i18n.getResourceBundle('en-US', 'system') as Record<string, unknown>;

  it('exposes the integration tab label in both languages', () => {
    expect((zh.tab as Record<string, string>).integration).toBeTruthy();
    expect((en.tab as Record<string, string>).integration).toBeTruthy();
  });

  it('keeps zh and en key sets identical under system.mcp', () => {
    const zhKeys = flatten(zh.mcp, 'mcp').sort();
    const enKeys = flatten(en.mcp, 'mcp').sort();
    // 少一条 key 的表现是界面直接显示 "mcp.client.cursor" 这种原文
    expect(zhKeys).toEqual(enKeys);
    expect(zhKeys.length).toBeGreaterThan(20);
  });

  it('covers every client kind offered by the snippet builder', async () => {
    const { MCP_CLIENT_KINDS } = await import('../utils/mcpSnippet');
    for (const kind of MCP_CLIENT_KINDS) {
      expect((zh.mcp as any).client[kind]).toBeTruthy();
      expect((zh.mcp as any).clientPath[kind]).toBeTruthy();
      expect((en.mcp as any).client[kind]).toBeTruthy();
      expect((en.mcp as any).clientPath[kind]).toBeTruthy();
    }
  });

  it('uses an interpolation placeholder that i18next can fill', async () => {
    await i18n.changeLanguage('zh-CN');
    const text = i18n.t('system:mcp.security.auditRecords', { count: 7 });
    expect(text).toContain('7');
    expect(text).not.toContain('{{');
  });
});
