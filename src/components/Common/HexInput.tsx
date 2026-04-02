import React, { useState, useCallback, useMemo } from 'react';
import { Input, Space, Button, Typography, Tooltip } from 'antd';
import { ClearOutlined, CopyOutlined, SwapOutlined } from '@ant-design/icons';

const { Text } = Typography;

interface HexInputProps {
  value?: string;
  onChange?: (value: string, bytes: number[]) => void;
  placeholder?: string;
  disabled?: boolean;
  maxLength?: number;
  showByteCount?: boolean;
  showActions?: boolean;
  allowText?: boolean;
}

const isValidHex = (str: string): boolean => {
  return /^[0-9A-Fa-f\s]*$/.test(str);
};

const normalizeHex = (str: string): string => {
  return str.replace(/\s+/g, '').toUpperCase();
};

const formatHex = (str: string): string => {
  const normalized = normalizeHex(str);
  return normalized.match(/.{1,2}/g)?.join(' ') || '';
};

const hexToBytes = (str: string): number[] => {
  const normalized = normalizeHex(str);
  const bytes: number[] = [];
  for (let i = 0; i < normalized.length; i += 2) {
    bytes.push(parseInt(normalized.substr(i, 2), 16));
  }
  return bytes;
};

const bytesToHex = (bytes: number[]): string => {
  return bytes.map((b) => b.toString(16).padStart(2, '0').toUpperCase()).join(' ');
};

const textToHex = (text: string): string => {
  const encoder = new TextEncoder();
  return bytesToHex(Array.from(encoder.encode(text)));
};

const hexToText = (hex: string): string => {
  const bytes = hexToBytes(hex);
  return new TextDecoder().decode(new Uint8Array(bytes));
};

const HexInput: React.FC<HexInputProps> = ({
  value = '',
  onChange,
  placeholder = '输入十六进制数据，如: 48 65 6C 6C 6F',
  disabled = false,
  maxLength,
  showByteCount = true,
  showActions = true,
  allowText = true,
}) => {
  const [inputValue, setInputValue] = useState(value);
  const [mode, setMode] = useState<'hex' | 'text'>('hex');
  const [error, setError] = useState<string | null>(null);

  const bytes = useMemo(() => {
    if (mode === 'hex') {
      return hexToBytes(inputValue);
    }
    return Array.from(new TextEncoder().encode(inputValue));
  }, [inputValue, mode]);

  const handleChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) => {
      const newValue = e.target.value;

      if (mode === 'hex' && !isValidHex(newValue)) {
        setError('请输入有效的十六进制字符 (0-9, A-F)');
        return;
      }

      setError(null);
      setInputValue(newValue);

      if (onChange) {
        const newBytes = mode === 'hex' ? hexToBytes(newValue) : Array.from(new TextEncoder().encode(newValue));
        onChange(newValue, newBytes);
      }
    },
    [mode, onChange]
  );

  const handleClear = useCallback(() => {
    setInputValue('');
    setError(null);
    if (onChange) {
      onChange('', []);
    }
  }, [onChange]);

  const handleCopy = useCallback(() => {
    navigator.clipboard.writeText(inputValue);
  }, [inputValue]);

  const handleToggleMode = useCallback(() => {
    if (mode === 'hex') {
      const text = hexToText(inputValue);
      setMode('text');
      setInputValue(text);
      if (onChange) {
        onChange(text, bytes);
      }
    } else {
      const hex = textToHex(inputValue);
      setMode('hex');
      setInputValue(hex);
      if (onChange) {
        onChange(hex, bytes);
      }
    }
  }, [mode, inputValue, bytes, onChange]);

  const handleFormat = useCallback(() => {
    if (mode === 'hex') {
      const formatted = formatHex(inputValue);
      setInputValue(formatted);
      if (onChange) {
        onChange(formatted, bytes);
      }
    }
  }, [mode, inputValue, bytes, onChange]);

  return (
    <div>
      <Space.Compact style={{ width: '100%' }}>
        <Input
          value={inputValue}
          onChange={handleChange}
          placeholder={placeholder}
          disabled={disabled}
          maxLength={maxLength}
          status={error ? 'error' : undefined}
          style={{ fontFamily: mode === 'hex' ? 'monospace' : undefined }}
        />
        {showActions && (
          <>
            <Tooltip title="格式化">
              <Button onClick={handleFormat} disabled={disabled || mode !== 'hex'}>
                格式化
              </Button>
            </Tooltip>
            {allowText && (
              <Tooltip title={mode === 'hex' ? '转为文本' : '转为十六进制'}>
                <Button onClick={handleToggleMode} disabled={disabled}>
                  <SwapOutlined />
                </Button>
              </Tooltip>
            )}
            <Tooltip title="复制">
              <Button onClick={handleCopy} disabled={disabled || !inputValue}>
                <CopyOutlined />
              </Button>
            </Tooltip>
            <Tooltip title="清空">
              <Button onClick={handleClear} disabled={disabled || !inputValue}>
                <ClearOutlined />
              </Button>
            </Tooltip>
          </>
        )}
      </Space.Compact>

      {error && (
        <Text type="danger" style={{ fontSize: 12 }}>
          {error}
        </Text>
      )}

      {showByteCount && !error && (
        <Text type="secondary" style={{ fontSize: 12 }}>
          {bytes.length} 字节
        </Text>
      )}
    </div>
  );
};

export default HexInput;
