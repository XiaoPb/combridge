import { invoke } from './tauri';
import type {
  Gh3220CaptureConfig,
  Gh3220CaptureStatus,
  Gh3220DebugStatus,
  Gh3220DeviceInfo,
  Gh3220LinkConfig,
  Gh3220Preferences,
  Gh3220Stats,
  Gh3220Status,
} from '../types/gh3220';

export const gh3220Api = {
  connect(config: Gh3220LinkConfig): Promise<Gh3220Status> {
    return invoke<Gh3220Status>('gh3220_connect', { config });
  },

  disconnect(): Promise<void> {
    return invoke<void>('gh3220_disconnect');
  },

  getStatus(): Promise<Gh3220Status> {
    return invoke<Gh3220Status>('gh3220_get_status');
  },

  getDeviceInfo(): Promise<Gh3220DeviceInfo> {
    return invoke<Gh3220DeviceInfo>('gh3220_get_device_info');
  },

  setWorkMode(mode: number, funcMask: number): Promise<number> {
    return invoke<number>('gh3220_set_work_mode', { mode, funcMask });
  },

  start(funcMask: number, slotEn = 0): Promise<number> {
    return invoke<number>('gh3220_start', { funcMask, slotEn });
  },

  stop(funcMask: number): Promise<number> {
    return invoke<number>('gh3220_stop', { funcMask });
  },

  switchSlotFunc(slot: number, extra: number, funcMask: number, on: boolean): Promise<number> {
    return invoke<number>('gh3220_switch_slot_func', { slot, extra, funcMask, on });
  },

  regRead(addr: number, count: number): Promise<number[]> {
    return invoke<number[]>('gh3220_reg_read', { addr, count });
  },

  regWrite(addr: number, values: number[]): Promise<void> {
    return invoke<void>('gh3220_reg_write', { addr, values });
  },

  loadRegList(filePath: string): Promise<number> {
    return invoke<number>('gh3220_load_reg_list', { filePath });
  },

  chipCtrl(action: string): Promise<number> {
    return invoke<number>('gh3220_chip_ctrl', { action });
  },

  setFifoThr(value: number): Promise<number> {
    return invoke<number>('gh3220_set_fifo_thr', { value });
  },

  setReportEvent(mask: number): Promise<number> {
    return invoke<number>('gh3220_set_report_event', { mask });
  },

  debugStatus(kind: string): Promise<Gh3220DebugStatus> {
    return invoke<Gh3220DebugStatus>('gh3220_debug_status', { kind });
  },

  packageTest(hex: string): Promise<number[]> {
    return invoke<number[]>('gh3220_package_test', { hex });
  },

  sendRaw(hex: string): Promise<void> {
    return invoke<void>('gh3220_send_raw', { hex });
  },

  captureStart(config: Gh3220CaptureConfig): Promise<void> {
    return invoke<void>('gh3220_capture_start', { config });
  },

  captureStop(): Promise<void> {
    return invoke<void>('gh3220_capture_stop');
  },

  getCaptureStatus(): Promise<Gh3220CaptureStatus> {
    return invoke<Gh3220CaptureStatus>('gh3220_get_capture_status');
  },

  getStats(): Promise<Gh3220Stats> {
    return invoke<Gh3220Stats>('gh3220_get_stats');
  },

  clearStats(): Promise<void> {
    return invoke<void>('gh3220_clear_stats');
  },

  getPreferences(): Promise<Gh3220Preferences> {
    return invoke<Gh3220Preferences>('gh3220_get_preferences');
  },

  updatePreferences(prefs: Gh3220Preferences): Promise<void> {
    return invoke<void>('gh3220_update_preferences', { prefs });
  },
};
