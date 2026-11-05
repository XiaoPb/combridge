import { describe, expect, it } from 'vitest';
import type { Gh3036FramesPayload } from '../api/events';
import { mergeGh3036Frames } from './gh3036FrameBuffer';

function makeFrames(start: number, count: number): Gh3036FramesPayload {
  const values = Array.from({ length: count }, (_, index) => start + index);

  return {
    function_id: 1,
    function_name: 'HR',
    frame_count: count,
    channel_count: 2,
    frame_cnts: values,
    timestamps: values,
    frame_ids: values,
    ipd_pa: [
      values.map((value) => value + 1000),
      values.map((value) => value + 2000),
    ],
    rawdata: [
      values.map((value) => value + 3000),
      values.map((value) => value + 4000),
    ],
    flags: [values.map(() => 1), values.map(() => 2)],
    agc_info: [values.map(() => 3), values.map(() => 4)],
    acc_x: values.map((value) => value + 10),
    acc_y: values.map((value) => value + 20),
    acc_z: values.map((value) => value + 30),
    gyro_x: values.map(() => 0),
    gyro_y: values.map(() => 0),
    gyro_z: values.map(() => 0),
    algo_results: values.map((value) => [value]),
    led_drv_fs: values.map(() => [1, 2]),
    ref_data: values.map((value) => [value + 5000]),
  };
}

describe('mergeGh3036Frames', () => {
  it('keeps frame_count aligned with the retained cache window', () => {
    const maxFramesCount = 2;
    const first = mergeGh3036Frames(undefined, makeFrames(0, 15), maxFramesCount);
    const merged = mergeGh3036Frames(first, makeFrames(15, 15), maxFramesCount);

    expect(merged.frame_count).toBe(20);
    expect(merged.frame_cnts).toHaveLength(20);
    expect(merged.ipd_pa[0]).toHaveLength(20);
    expect(merged.rawdata[0]).toHaveLength(20);
    expect(merged.frame_cnts[0]).toBe(10);
    expect(merged.ipd_pa[0][0]).toBe(1010);
    expect(merged.rawdata[1][19]).toBe(4029);
  });

  it('drops incoming frames that overlap with the cache tail', () => {
    const maxFramesCount = 100;
    // 缓存已有 0..14，重传批次为 10..19：前 5 帧重叠，后 5 帧为新帧
    const first = mergeGh3036Frames(undefined, makeFrames(0, 15), maxFramesCount);
    const merged = mergeGh3036Frames(first, makeFrames(10, 10), maxFramesCount);

    expect(merged.frame_cnts).toEqual([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19]);
    expect(merged.frame_count).toBe(20);
    expect(merged.ipd_pa[0]).toHaveLength(20);
    expect(merged.ipd_pa[0][19]).toBe(1019);
    expect(merged.acc_x).toHaveLength(20);
    expect(merged.algo_results).toHaveLength(20);
  });

  it('returns the same cache when the whole batch is duplicated', () => {
    const maxFramesCount = 100;
    const first = mergeGh3036Frames(undefined, makeFrames(0, 10), maxFramesCount);
    const merged = mergeGh3036Frames(first, makeFrames(0, 10), maxFramesCount);

    expect(merged).toBe(first);
  });

  it('drops duplicate frames inside a single batch', () => {
    const maxFramesCount = 100;
    const duplicated = makeFrames(0, 5);
    duplicated.frame_cnts = [0, 1, 1, 2, 3];
    duplicated.timestamps = [0, 1, 1, 2, 3];

    const merged = mergeGh3036Frames(undefined, duplicated, maxFramesCount);

    expect(merged.frame_cnts).toEqual([0, 1, 2, 3]);
    expect(merged.frame_count).toBe(4);
    expect(merged.ipd_pa[0]).toHaveLength(4);
  });

  it('keeps the whole batch when the device restarts frame_cnt', () => {
    const maxFramesCount = 100;
    const first = mergeGh3036Frames(undefined, makeFrames(50, 10), maxFramesCount);

    // 设备重启后 frame_cnt 回退到 0
    const merged = mergeGh3036Frames(first, makeFrames(0, 5), maxFramesCount);

    expect(merged.frame_cnts).toEqual([...Array.from({ length: 10 }, (_, i) => 50 + i), 0, 1, 2, 3, 4]);
    expect(merged.frame_count).toBe(15);
  });

  it('treats a restarted session as new even when the cache holds the same frame_cnt range', () => {
    const maxFramesCount = 1000;
    // 上一轮会话已缓存 0..199。用户停止后重新开始采样时，设备的 frame_cnt 与
    // timestamp 都会从 0 重新计数，与缓存开头完全重合。
    const first = mergeGh3036Frames(undefined, makeFrames(0, 200), maxFramesCount);

    // 旧实现按 (frame_cnt, timestamp) 逐帧比对，会把整批 50 帧判成重传全部丢弃，
    // 表现为波形冻结约一个缓存窗口后才恢复。
    const restarted = mergeGh3036Frames(first, makeFrames(0, 50), maxFramesCount);

    expect(restarted.frame_count).toBe(250);
    expect(restarted.frame_cnts.slice(200)).toEqual(
      Array.from({ length: 50 }, (_, i) => i)
    );
    expect(restarted.ipd_pa[0]).toHaveLength(250);
  });

  it('still drops resent frames inside the same session', () => {
    const maxFramesCount = 1000;
    const first = mergeGh3036Frames(undefined, makeFrames(0, 200), maxFramesCount);
    // 同一会话内的重传：批次首帧不从 0 开始，必须继续按重复帧丢弃
    const merged = mergeGh3036Frames(first, makeFrames(195, 10), maxFramesCount);

    expect(merged.frame_count).toBe(205);
    expect(merged.frame_cnts.slice(-5)).toEqual([200, 201, 202, 203, 204]);
  });
});
