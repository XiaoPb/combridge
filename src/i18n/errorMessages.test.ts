import { describe, expect, it } from 'vitest';
import i18n from './index';

const stringKeys = [
  'common:common.error',
  'common:message.dispatchFailed',
  'serial:message.scanFailed',
  'serial:message.openFailed',
  'ble:message.configureFailed',
  'ble:message.atSendFailed',
  'protocol:message.listFailed',
  'protocol:gh3036.configDownloadFailed',
  'waveform:errors.createBuffer',
  'waveform:errors.loadCsv',
  'gh3036:errors.executeRpc',
  'gh3036:errors.getVersion',
  'system:message.timezoneUpdateFailed',
] as const;

describe('user-visible error translations', () => {
  it.each(['zh-CN', 'en-US'] as const)('resolves all audited keys as strings in %s', async (language) => {
    await i18n.changeLanguage(language);

    for (const key of stringKeys) {
      const value = i18n.t(key);
      expect(typeof value, key).toBe('string');
      expect(value, key).not.toBe(key);
    }
  });
});
