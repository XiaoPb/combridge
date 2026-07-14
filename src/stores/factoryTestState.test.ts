import { describe, expect, it } from 'vitest';
import { hasFactoryTestResult } from './factoryTestState';

describe('hasFactoryTestResult', () => {
  it.each([
    ['completed', true],
    ['failed', true],
    ['running', false],
    ['stopped', false],
  ] as const)('returns %s for %s', (status, expected) => {
    expect(hasFactoryTestResult(status)).toBe(expected);
  });
});
