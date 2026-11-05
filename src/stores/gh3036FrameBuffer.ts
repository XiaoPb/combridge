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

/** 新会话首帧的 frame_cnt 上限：设备每次重新开始采样都从 0 计数 */
const NEW_SESSION_HEAD_MAX_CNT = 10;
/** 判定跨会话所需的最小 frame_cnt 回退量，避免把正常重传误判为新会话 */
const NEW_SESSION_MIN_REGRESSION = 100;

/**
 * 筛选需要保留的入站帧下标：
 * 1. 新会话（frame_cnt 从 0 重新计数，且与缓存末尾相差足够大）整批保留，
 *    不再逐帧比对 (frame_cnt, timestamp)——两者都会随会话归零，逐帧比对会把
 *    新会话的数据误判成上一会话的重传帧整段丢弃（波形冻结约一个缓存窗口）；
 * 2. (frame_cnt, timestamp) 组合与既有缓存相同的帧视为重传重复帧，直接丢弃，
 *    避免其在波形上形成"不变的值"平坦段；
 * 3. 同一批次内 (frame_cnt, timestamp) 完全相同的连续重复帧同样剔除；
 * 4. frame_cnt 回退且 (frame_cnt, timestamp) 未在缓存中出现过的帧，
 *    视为设备重启后的新一轮采集，从该帧起整批保留。
 */
function pickIndices(
  existing: Gh3036FramesPayload | undefined,
  incoming: Gh3036FramesPayload
): number[] {
  const lastCnt =
    existing && existing.frame_cnts.length > 0
      ? existing.frame_cnts[existing.frame_cnts.length - 1]
      : Number.NEGATIVE_INFINITY;

  const firstCnt = incoming.frame_cnts[0];
  const isNewSession =
    existing !== undefined &&
    existing.frame_cnts.length > 0 &&
    Number.isFinite(firstCnt) &&
    firstCnt <= NEW_SESSION_HEAD_MAX_CNT &&
    lastCnt - firstCnt >= NEW_SESSION_MIN_REGRESSION;

  let cacheKeys: Set<string> | null = null;
  let resetLatched = isNewSession;
  const batchKeys = new Set<string>();
  const indices: number[] = [];

  for (let i = 0; i < incoming.frame_cnts.length; i++) {
    const cnt = incoming.frame_cnts[i];
    const ts = incoming.timestamps[i];

    if (!resetLatched && cnt <= lastCnt) {
      // 可能是重传帧，也可能是设备重启后的新会话，需要比对 (cnt, ts)
      cacheKeys ??= new Set(
        existing
          ? existing.frame_cnts.map((cachedCnt, index) =>
              `${cachedCnt}:${existing.timestamps[index]}`
            )
          : []
      );
      if (cacheKeys.has(`${cnt}:${ts}`)) {
        continue;
      }
      // cnt 回退但组合不存在于缓存 → 新一轮采集开始
      resetLatched = true;
    }

    const key = `${cnt}:${ts}`;
    if (batchKeys.has(key)) {
      continue;
    }
    batchKeys.add(key);
    indices.push(i);
  }

  return indices;
}

function gatherScalars(
  values: number[] | undefined,
  indices: number[]
): number[] {
  return indices.map((index) => values?.[index] ?? 0);
}

function gatherMatrix(
  rows: number[][] | undefined,
  indices: number[]
): number[][] {
  // 通道矩阵（ipd_pa/rawdata/flags/agc_info）：每行是一个通道的时间序列，
  // 按帧下标在每行内抽取
  return (rows ?? []).map((row) => indices.map((index) => row?.[index] ?? 0));
}

function gatherFrameMatrix(
  rows: number[][] | undefined,
  indices: number[]
): number[][] {
  // 帧矩阵（algo_results/ref_data）：每行是一帧的数据，按帧下标整行抽取
  return indices.map((index) => rows?.[index] ?? []);
}

function gatherPairs(
  values: [number, number][] | undefined,
  indices: number[]
): [number, number][] {
  return indices.map((index) => values?.[index] ?? [0, 0]);
}

/** 按下标集合抽取帧，保持所有与帧对齐的数组同步截取 */
function pickFrames(
  frames: Gh3036FramesPayload,
  indices: number[]
): Gh3036FramesPayload {
  if (indices.length === frames.frame_cnts.length) {
    return frames;
  }
  return {
    ...frames,
    frame_count: indices.length,
    frame_cnts: gatherScalars(frames.frame_cnts, indices),
    timestamps: gatherScalars(frames.timestamps, indices),
    frame_ids: gatherScalars(frames.frame_ids, indices),
    ipd_pa: gatherMatrix(frames.ipd_pa, indices),
    rawdata: gatherMatrix(frames.rawdata, indices),
    flags: gatherMatrix(frames.flags, indices),
    agc_info: gatherMatrix(frames.agc_info, indices),
    acc_x: gatherScalars(frames.acc_x, indices),
    acc_y: gatherScalars(frames.acc_y, indices),
    acc_z: gatherScalars(frames.acc_z, indices),
    gyro_x: gatherScalars(frames.gyro_x, indices),
    gyro_y: gatherScalars(frames.gyro_y, indices),
    gyro_z: gatherScalars(frames.gyro_z, indices),
    algo_results: gatherFrameMatrix(frames.algo_results, indices),
    led_drv_fs: gatherPairs(frames.led_drv_fs, indices),
    ref_data: gatherFrameMatrix(frames.ref_data, indices),
  };
}

/**
 * 对入站批次去重。返回 null 表示整批都是重复帧（调用方应保留原缓存，
 * 避免无意义的 store 更新与图表重绘）。
 */
function dedupeIncoming(
  existing: Gh3036FramesPayload | undefined,
  incoming: Gh3036FramesPayload
): Gh3036FramesPayload | null {
  const indices = pickIndices(existing, incoming);
  if (indices.length === 0) {
    return null;
  }
  return pickFrames(incoming, indices);
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
  const deduped = dedupeIncoming(existing, incoming);

  // 整批重复：原样保留缓存，避免触发多余渲染
  if (!deduped) {
    return existing ?? normalizeGh3036Frames(incoming, maxFramesCount);
  }

  if (!existing) {
    return normalizeGh3036Frames(deduped, maxFramesCount);
  }

  // 通道数变化意味着缓存按旧通道数组织，按帧对齐会错位，直接以新批次重建
  if (existing.channel_count !== deduped.channel_count) {
    return normalizeGh3036Frames(deduped, maxFramesCount);
  }

  const maxLength = maxFramesCount * FRAME_CACHE_MULTIPLIER;
  const channelCount = deduped.channel_count;
  const frameCnts = trimArray(
    [...existing.frame_cnts, ...deduped.frame_cnts],
    maxLength
  );

  return {
    function_id: deduped.function_id,
    function_name: deduped.function_name,
    frame_count: frameCnts.length,
    channel_count: channelCount,
    frame_cnts: frameCnts,
    timestamps: trimArray(
      [...existing.timestamps, ...deduped.timestamps],
      maxLength
    ),
    frame_ids: trimArray(
      [...existing.frame_ids, ...deduped.frame_ids],
      maxLength
    ),
    ipd_pa: mergeMatrix(existing.ipd_pa, deduped.ipd_pa, channelCount, maxLength),
    rawdata: mergeMatrix(existing.rawdata, deduped.rawdata, channelCount, maxLength),
    flags: mergeMatrix(existing.flags, deduped.flags, channelCount, maxLength),
    agc_info: mergeMatrix(existing.agc_info, deduped.agc_info, channelCount, maxLength),
    acc_x: trimArray([...existing.acc_x, ...deduped.acc_x], maxLength),
    acc_y: trimArray([...existing.acc_y, ...deduped.acc_y], maxLength),
    acc_z: trimArray([...existing.acc_z, ...deduped.acc_z], maxLength),
    gyro_x: trimArray([...existing.gyro_x, ...deduped.gyro_x], maxLength),
    gyro_y: trimArray([...existing.gyro_y, ...deduped.gyro_y], maxLength),
    gyro_z: trimArray([...existing.gyro_z, ...deduped.gyro_z], maxLength),
    algo_results: trimArray(
      [...existing.algo_results, ...deduped.algo_results],
      maxLength
    ),
    led_drv_fs: trimArray(
      [...existing.led_drv_fs, ...deduped.led_drv_fs],
      maxLength
    ),
    ref_data: trimArray(
      [...(existing.ref_data ?? []), ...(deduped.ref_data ?? [])],
      maxLength
    ),
  };
}
