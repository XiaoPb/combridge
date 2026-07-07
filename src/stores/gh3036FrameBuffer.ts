import type { Gh3036FramesPayload } from '../api/events';

const FRAME_CACHE_MULTIPLIER = 10;

function trimArray<T>(items: T[], maxLength: number): T[] {
  return items.slice(-maxLength);
}

function mergeMatrix(
  existing: number[][],
  incoming: number[][],
  channelCount: number,
  maxLength: number
): number[][] {
  return Array.from({ length: channelCount }, (_, index) => (
    trimArray([...(existing[index] ?? []), ...(incoming[index] ?? [])], maxLength)
  ));
}

export function normalizeGh3036Frames(
  frames: Gh3036FramesPayload,
  maxFramesCount: number
): Gh3036FramesPayload {
  const maxLength = maxFramesCount * FRAME_CACHE_MULTIPLIER;
  const channelCount = frames.channel_count;
  const frameCnts = trimArray(frames.frame_cnts, maxLength);

  return {
    ...frames,
    frame_count: frameCnts.length,
    channel_count: channelCount,
    frame_cnts: frameCnts,
    timestamps: trimArray(frames.timestamps, maxLength),
    frame_ids: trimArray(frames.frame_ids, maxLength),
    ipd_pa: mergeMatrix([], frames.ipd_pa, channelCount, maxLength),
    rawdata: mergeMatrix([], frames.rawdata, channelCount, maxLength),
    flags: mergeMatrix([], frames.flags, channelCount, maxLength),
    agc_info: mergeMatrix([], frames.agc_info, channelCount, maxLength),
    acc_x: trimArray(frames.acc_x, maxLength),
    acc_y: trimArray(frames.acc_y, maxLength),
    acc_z: trimArray(frames.acc_z, maxLength),
    gyro_x: trimArray(frames.gyro_x, maxLength),
    gyro_y: trimArray(frames.gyro_y, maxLength),
    gyro_z: trimArray(frames.gyro_z, maxLength),
    algo_results: trimArray(frames.algo_results, maxLength),
    led_drv_fs: trimArray(frames.led_drv_fs, maxLength),
    ref_data: trimArray(frames.ref_data ?? [], maxLength),
  };
}

export function mergeGh3036Frames(
  existing: Gh3036FramesPayload | undefined,
  incoming: Gh3036FramesPayload,
  maxFramesCount: number
): Gh3036FramesPayload {
  if (!existing) {
    return normalizeGh3036Frames(incoming, maxFramesCount);
  }

  const maxLength = maxFramesCount * FRAME_CACHE_MULTIPLIER;
  const channelCount = incoming.channel_count;
  const frameCnts = trimArray([...existing.frame_cnts, ...incoming.frame_cnts], maxLength);

  return {
    function_id: incoming.function_id,
    function_name: incoming.function_name,
    frame_count: frameCnts.length,
    channel_count: channelCount,
    frame_cnts: frameCnts,
    timestamps: trimArray([...existing.timestamps, ...incoming.timestamps], maxLength),
    frame_ids: trimArray([...existing.frame_ids, ...incoming.frame_ids], maxLength),
    ipd_pa: mergeMatrix(existing.ipd_pa, incoming.ipd_pa, channelCount, maxLength),
    rawdata: mergeMatrix(existing.rawdata, incoming.rawdata, channelCount, maxLength),
    flags: mergeMatrix(existing.flags, incoming.flags, channelCount, maxLength),
    agc_info: mergeMatrix(existing.agc_info, incoming.agc_info, channelCount, maxLength),
    acc_x: trimArray([...existing.acc_x, ...incoming.acc_x], maxLength),
    acc_y: trimArray([...existing.acc_y, ...incoming.acc_y], maxLength),
    acc_z: trimArray([...existing.acc_z, ...incoming.acc_z], maxLength),
    gyro_x: trimArray([...existing.gyro_x, ...incoming.gyro_x], maxLength),
    gyro_y: trimArray([...existing.gyro_y, ...incoming.gyro_y], maxLength),
    gyro_z: trimArray([...existing.gyro_z, ...incoming.gyro_z], maxLength),
    algo_results: trimArray([...existing.algo_results, ...incoming.algo_results], maxLength),
    led_drv_fs: trimArray([...existing.led_drv_fs, ...incoming.led_drv_fs], maxLength),
    ref_data: trimArray([...(existing.ref_data ?? []), ...(incoming.ref_data ?? [])], maxLength),
  };
}
