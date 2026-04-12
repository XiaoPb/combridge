import { invoke } from '@tauri-apps/api/core';
import type {
  ParserScriptInfo,
  JsonStructureInfo,
  FieldDefinition,
  DashboardJsonConfig,
} from '../types/dashboard';

export const dashboardApi = {
  async getParserScripts(): Promise<ParserScriptInfo[]> {
    return invoke('get_parser_scripts');
  },

  async saveParserScript(name: string, content: string): Promise<void> {
    return invoke('save_parser_script', { name, content });
  },

  async deleteParserScript(name: string): Promise<void> {
    return invoke('delete_parser_script', { name });
  },

  async executeParserScript(name: string, data: string): Promise<Record<string, number>> {
    return invoke('execute_parser_script', { name, data });
  },

  async initDefaultParserScripts(): Promise<void> {
    return invoke('init_default_parser_scripts');
  },

  async generateParserFromJson(
    jsonContent: string,
    name: string,
    selectedFields: string[]
  ): Promise<string> {
    return invoke('generate_parser_from_json', { jsonContent, name, selectedFields });
  },

  async mergeJsonToParser(
    jsonContent: string,
    scriptName: string,
    selectedFields: string[]
  ): Promise<string> {
    return invoke('merge_json_to_parser', { jsonContent, scriptName, selectedFields });
  },

  async analyzeJsonStructure(jsonContent: string): Promise<JsonStructureInfo> {
    return invoke('analyze_json_structure', { jsonContent });
  },

  async getParserDefinedFields(scriptName: string): Promise<FieldDefinition[]> {
    return invoke('get_parser_defined_fields', { scriptName });
  },

  async getJsonFiles(): Promise<string[]> {
    return invoke('get_json_files');
  },

  async saveJsonFile(fileName: string, config: DashboardJsonConfig): Promise<void> {
    return invoke('save_json_file', { file_name: fileName, config });
  },

  async deleteJsonFile(fileName: string): Promise<void> {
    return invoke('delete_json_file', { file_name: fileName });
  },

  async loadJsonFile(fileName: string): Promise<DashboardJsonConfig> {
    return invoke('load_json_file', { file_name: fileName });
  },
};
