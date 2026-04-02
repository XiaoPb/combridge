import React from 'react';
import { Space, Select, Input, Typography } from 'antd';
import { FilterOutlined } from '@ant-design/icons';

const { Search } = Input;
const { Text } = Typography;

interface LogFilterProps {
  directionFilter: 'all' | 'send' | 'receive';
  formatFilter: 'hex' | 'text' | 'binary';
  searchText: string;
  onDirectionChange: (value: 'all' | 'send' | 'receive') => void;
  onFormatChange: (value: 'hex' | 'text' | 'binary') => void;
  onSearchChange: (value: string) => void;
}

const LogFilter: React.FC<LogFilterProps> = ({
  directionFilter,
  formatFilter,
  searchText,
  onDirectionChange,
  onFormatChange,
  onSearchChange,
}) => {
  return (
    <Space style={{ marginBottom: 12 }} wrap>
      <Text type="secondary">
        <FilterOutlined style={{ marginRight: 4 }} />
        筛选:
      </Text>

      <Select
        value={directionFilter}
        onChange={onDirectionChange}
        style={{ width: 100 }}
        size="small"
        options={[
          { value: 'all', label: '全部' },
          { value: 'send', label: '发送' },
          { value: 'receive', label: '接收' },
        ]}
      />

      <Select
        value={formatFilter}
        onChange={onFormatChange}
        style={{ width: 100 }}
        size="small"
        options={[
          { value: 'hex', label: '十六进制' },
          { value: 'text', label: '文本' },
          { value: 'binary', label: '二进制' },
        ]}
      />

      <Search
        placeholder="搜索数据..."
        value={searchText}
        onChange={(e) => onSearchChange(e.target.value)}
        style={{ width: 150 }}
        size="small"
        allowClear
      />
    </Space>
  );
};

export default LogFilter;
