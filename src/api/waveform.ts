import { invoke } from '@tauri-apps/api/core';

export type ParserType = 'delimiter' | 'regex';

export interface ParserConfig {
  parser_type: ParserType;
  delimiter: string | null;
  pattern: string | null;
  column_names: string[];
  trim_whitespace: boolean;
}

export interface WaveformBufferConfig {
  capacity: number;
  column_names: string[];
}

export interface WaveformData {
  columns: string[];
  rows: number[][];
  timestamp: number;
}

export interface WaveformStatus {
  buffer_id: string;
  row_count: number;
  column_count: number;
  column_names: string[];
  capacity: number;
  parser_type: ParserType | null;
}

export const waveformApi = {
  createBuffer: async (bufferId: string, config: WaveformBufferConfig): Promise<void> => {
    await invoke('waveform_create_buffer', { bufferId, config });
  },

  removeBuffer: async (bufferId: string): Promise<void> => {
    await invoke('waveform_remove_buffer', { bufferId });
  },

  configureParser: async (bufferId: string, config: ParserConfig): Promise<void> => {
    await invoke('waveform_configure_parser', { bufferId, config });
  },

  parseAndStore: async (bufferId: string, data: string): Promise<void> => {
    await invoke('waveform_parse_and_store', { bufferId, data });
  },

  readData: async (bufferId: string, rows: number): Promise<WaveformData> => {
    return invoke('waveform_read_data', { bufferId, rows });
  },

  getStatus: async (bufferId: string): Promise<WaveformStatus> => {
    return invoke('waveform_get_status', { bufferId });
  },

  clearBuffer: async (bufferId: string): Promise<void> => {
    await invoke('waveform_clear_buffer', { bufferId });
  },

  listBuffers: async (): Promise<string[]> => {
    return invoke('waveform_list_buffers');
  },
};
