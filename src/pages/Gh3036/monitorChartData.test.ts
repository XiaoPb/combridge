import { describe, expect, it } from 'vitest';
import type { Gh3036FramesPayload } from '../../api/events';
import { buildIpdPaChartData } from './monitorChartData';

function makeFrames(frameCount: number, channelValues: number[][]): Gh3036FramesPayload {
  const frameIndexes = Array.from({ length: frameCount }, (_, index) => index);

  return {
    function_id: 1,
    function_name: 'HR',
    frame_count: frameCount,
    channel_count: channelValues.length,
    frame_cnts: frameIndexes,
    timestamps: frameIndexes,
    frame_ids: frameIndexes,
    ipd_pa: channelValues,
    rawdata: channelValues.map((channel) => channel.map((value) => value * 10)),
    flags: channelValues.map(() => frameIndexes.map(() => 0)),
    agc_info: channelValues.map(() => frameIndexes.map(() => 0)),
    acc_x: frameIndexes,
    acc_y: frameIndexes,
    acc_z: frameIndexes,
    gyro_x: frameIndexes.map(() => 0),
    gyro_y: frameIndexes.map(() => 0),
    gyro_z: frameIndexes.map(() => 0),
    algo_results: frameIndexes.map(() => []),
    led_drv_fs: frameIndexes.map(() => [0, 0]),
    ref_data: frameIndexes.map(() => []),
  };
}

describe('buildIpdPaChartData', () => {
  it('uses retained channel length instead of a drifted historical frame_count', () => {
    const frames = makeFrames(1000, [
      [101, 102, 103, 104],
      [201, 202, 203, 204],
    ]);

    const data = buildIpdPaChartData(frames, 'ipd');

    expect(data.columns).toEqual(['CH0', 'CH1']);
    expect(data.rows).toEqual([
      [101, 201],
      [102, 202],
      [103, 203],
      [104, 204],
    ]);
  });

  it('returns live data after a clear-style reset and new append', () => {
    const frames = makeFrames(2, [
      [11, 12],
      [21, 22],
    ]);

    const data = buildIpdPaChartData(frames, 'rawdata');

    expect(data.rows).toEqual([
      [110, 210],
      [120, 220],
    ]);
  });
});
