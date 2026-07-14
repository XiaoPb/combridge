import type { FactoryTestStatus } from '../api/types';

export const hasFactoryTestResult = (status: FactoryTestStatus): boolean =>
  status === 'completed' || status === 'failed';
