import { describe, expect, it } from 'vitest';
import { buildMcpSnippet } from './mcpSnippet';

const URL = 'http://127.0.0.1:47821/mcp';
const TOKEN = 'abc123def456';

describe('buildMcpSnippet', () => {
  it('returns empty when service is not running', () => {
    expect(buildMcpSnippet('claudeCode', URL, '')).toBe('');
    expect(buildMcpSnippet('generic', URL, '')).toBe('');
  });

  it('builds a CLI command with the exact Bearer header', () => {
    const snippet = buildMcpSnippet('claudeCode', URL, TOKEN);
    expect(snippet).toContain('claude mcp add --transport http combridge');
    expect(snippet).toContain(URL);
    // 少一个 Bearer 前缀就会 401，而且客户端只报"连接失败"
    expect(snippet).toContain(`--header "Authorization: Bearer ${TOKEN}"`);
  });

  it.each(['claudeDesktop', 'cursor', 'generic'] as const)(
    'builds parseable JSON for %s',
    (client) => {
      const snippet = buildMcpSnippet(client, URL, TOKEN);
      const parsed = JSON.parse(snippet);
      expect(parsed.mcpServers.combridge).toEqual({
        type: 'http',
        url: URL,
        headers: { Authorization: `Bearer ${TOKEN}` },
      });
    }
  );

  it('keeps token intact (no truncation or escaping)', () => {
    const longToken = 'f'.repeat(64);
    const parsed = JSON.parse(buildMcpSnippet('generic', URL, longToken));
    expect(parsed.mcpServers.combridge.headers.Authorization).toBe(`Bearer ${longToken}`);
  });
});
