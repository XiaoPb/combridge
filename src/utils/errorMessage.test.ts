import { describe, expect, it } from 'vitest';
import { formatErrorMessage, getErrorDetail } from './errorMessage';

describe('getErrorDetail', () => {
  it('reads JavaScript errors and string errors', () => {
    expect(getErrorDetail(new Error('连接超时'))).toBe('连接超时');
    expect(getErrorDetail('设备忙')).toBe('设备忙');
  });

  it('reads Tauri structured errors with their error code', () => {
    expect(getErrorDetail({ code: 3000, error_code: 'E3000', message: '解包失败' }))
      .toBe('[E3000] 解包失败');
  });

  it('supports legacy error fields', () => {
    expect(getErrorDetail({ error: '写入失败' })).toBe('写入失败');
  });

  it('does not expose unknown objects or empty errors', () => {
    expect(getErrorDetail({ code: 1 })).toBeNull();
    expect(getErrorDetail(null)).toBeNull();
    expect(getErrorDetail('  ')).toBeNull();
  });
});

describe('formatErrorMessage', () => {
  it('combines an operation message with backend details', () => {
    expect(formatErrorMessage(
      { error_code: 'E3000', message: '解包失败' },
      '执行 RPC 指令失败',
    )).toBe('执行 RPC 指令失败: [E3000] 解包失败');
  });

  it('uses only the operation message when no detail is available', () => {
    expect(formatErrorMessage({}, '执行 RPC 指令失败')).toBe('执行 RPC 指令失败');
  });

  it('does not repeat an identical operation message', () => {
    expect(formatErrorMessage('执行 RPC 指令失败', '执行 RPC 指令失败'))
      .toBe('执行 RPC 指令失败');
  });
});
