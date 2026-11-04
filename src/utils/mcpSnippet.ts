/**
 * MCP 客户端配置片段生成。
 *
 * 单独抽出来是因为这段字符串是**用户唯一会复制走的东西** ——
 * 少一个 `Bearer `、少一层 `mcpServers` 包裹，现象都是"客户端连上了但工具全没有"，
 * 排查成本很高。所以这里给纯函数 + 单测，而不是写死在组件 JSX 里。
 */

export type McpClientKind = 'claudeCode' | 'claudeDesktop' | 'cursor' | 'generic';

export const MCP_CLIENT_KINDS: McpClientKind[] = [
  'claudeCode',
  'claudeDesktop',
  'cursor',
  'generic',
];

/**
 * 生成片段。
 *
 * - `claudeCode`：命令行 `claude mcp add`，Windows/Git Bash 各终端通用，不用改文件
 * - 其余：Streamable HTTP 的 JSON 结构（Claude Desktop / Cursor 一致）
 *
 * token 为空（服务未启动）时返回空串，由调用方给占位文案。
 */
export function buildMcpSnippet(
  client: McpClientKind,
  url: string,
  token: string
): string {
  if (!token) {
    return '';
  }

  if (client === 'claudeCode') {
    // 续行用反斜杠：Git Bash / PowerShell 7 / zsh 都认；
    // 单行更保险但太长，用户容易截断粘贴。
    return `claude mcp add --transport http combridge ${url} \\\n  --header "Authorization: Bearer ${token}"`;
  }

  return JSON.stringify(
    {
      mcpServers: {
        combridge: {
          type: 'http',
          url,
          headers: {
            Authorization: `Bearer ${token}`,
          },
        },
      },
    },
    null,
    2
  );
}
