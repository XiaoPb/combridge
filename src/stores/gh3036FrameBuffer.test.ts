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
});
