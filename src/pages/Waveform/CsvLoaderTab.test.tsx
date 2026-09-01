// @vitest-environment happy-dom

import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import i18n from '../../i18n';
import { useCsvChartStore } from '../../stores/csvChartStore';

const { exportAllPng } = vi.hoisted(() => ({
  exportAllPng: vi.fn(),
}));

vi.mock('./MultiLineChart', async () => {
  const ReactModule = await import('react');

  const MockMultiLineChart = ReactModule.forwardRef<
    { exportAllPng: () => Promise<void> },
    { onExportError?: (error: Error) => void }
  >(({ onExportError }, ref) => {
    ReactModule.useImperativeHandle(ref, () => ({ exportAllPng }), []);

    return ReactModule.createElement(
      'button',
      {
        type: 'button',
        'data-testid': 'trigger-chart-error',
        onClick: () => onExportError?.(new Error('chart callback failed')),
      },
      'Trigger chart error',
    );
  });

  return { default: MockMultiLineChart };
});

import CsvLoaderTab from './CsvLoaderTab';

function setCsvData(
  columns: string[] = ['heart_rate'],
  rows: number[][] = columns.length > 0 ? [[72]] : [],
) {
  useCsvChartStore.setState({
    csvData: { columns, rows },
  });
}

function createDeferred() {
  let resolve!: () => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<void>((promiseResolve, promiseReject) => {
    resolve = promiseResolve;
    reject = promiseReject;
  });
  return { promise, resolve, reject };
}

describe('CsvLoaderTab PNG export entry', () => {
  afterEach(() => {
    cleanup();
  });

  beforeEach(async () => {
    await i18n.changeLanguage('zh-CN');
    useCsvChartStore.getState().clearData();
    exportAllPng.mockReset();
    exportAllPng.mockResolvedValue(undefined);
  });

  it('hides the export button when CSV data or columns are unavailable', () => {
    const { rerender } = render(<CsvLoaderTab />);

    expect(screen.queryByRole('button', { name: /导出全部 PNG/ })).toBeNull();

    setCsvData([]);
    rerender(<CsvLoaderTab />);

    expect(screen.queryByRole('button', { name: /导出全部 PNG/ })).toBeNull();
  });

  it('shows the export button when CSV data has columns', () => {
    setCsvData();

    render(<CsvLoaderTab />);

    expect(screen.getByRole('button', { name: /导出全部 PNG/ })).toBeTruthy();
  });

  it('hides the export button for a header-only CSV', () => {
    setCsvData(['heart_rate'], []);

    render(<CsvLoaderTab />);

    expect(screen.queryByRole('button', { name: /导出全部 PNG/ })).toBeNull();
  });

  it('calls the chart ref export method and disables the button while exporting', async () => {
    const deferred = createDeferred();
    exportAllPng.mockReturnValueOnce(deferred.promise);
    setCsvData();

    render(<CsvLoaderTab />);
    const button = screen.getByRole('button', { name: /导出全部 PNG/ });

    fireEvent.click(button);

    expect(exportAllPng).toHaveBeenCalledTimes(1);
    expect(button).toHaveProperty('disabled', true);
    expect(button.className).toContain('ant-btn-loading');

    fireEvent.click(button);
    expect(exportAllPng).toHaveBeenCalledTimes(1);

    deferred.resolve();
    await waitFor(() => expect(button).toHaveProperty('disabled', false));
  });

  it('disables file selection and reload while exporting', async () => {
    const deferred = createDeferred();
    exportAllPng.mockReturnValueOnce(deferred.promise);
    setCsvData();
    const loadCsvFile = vi.fn<
      (filePath: string, options?: { resetZoom?: boolean }) => Promise<void>
    >(async () => undefined);
    useCsvChartStore.setState({ filePath: 'data.csv', loadCsvFile });

    render(<CsvLoaderTab />);
    fireEvent.click(screen.getByRole('button', { name: /导出全部 PNG/ }));

    expect(screen.getByRole('button', { name: /选择文件/ })).toHaveProperty('disabled', true);
    expect(screen.getByRole('button', { name: /重新加载/ })).toHaveProperty('disabled', true);

    deferred.resolve();
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /选择文件/ })).toHaveProperty('disabled', false);
      expect(screen.getByRole('button', { name: /重新加载/ })).toHaveProperty('disabled', false);
    });
  });

  it('shows a page error and restores the button when export rejects', async () => {
    exportAllPng.mockRejectedValueOnce(new Error('PNG export failed'));
    setCsvData();

    render(<CsvLoaderTab />);
    const button = screen.getByRole('button', { name: /导出全部 PNG/ });

    fireEvent.click(button);

    expect(await screen.findByText('PNG export failed')).toBeTruthy();
    await waitFor(() => expect(button).toHaveProperty('disabled', false));
  });

  it('shows errors reported by the chart through the existing page alert', async () => {
    setCsvData();

    render(<CsvLoaderTab />);
    fireEvent.click(screen.getByTestId('trigger-chart-error'));

    expect(await screen.findByText('chart callback failed')).toBeTruthy();
  });

  it('prefers a new export error over a stale store error', async () => {
    exportAllPng.mockRejectedValueOnce(new Error('PNG export failed'));
    setCsvData();

    render(<CsvLoaderTab />);
    fireEvent.click(screen.getByRole('button', { name: /导出全部 PNG/ }));
    expect(await screen.findByText('PNG export failed')).toBeTruthy();

    await act(async () => {
      useCsvChartStore.setState({ error: 'stale load error' });
    });

    await waitFor(() => {
      expect(document.body.textContent).toContain('PNG export failed');
      expect(document.body.textContent).not.toContain('stale load error');
    });
  });

  it('clears an export error when reloading and receiving new CSV data', async () => {
    exportAllPng.mockRejectedValueOnce(new Error('PNG export failed'));
    setCsvData();
    const loadCsvFile = vi.fn<
      (filePath: string, options?: { resetZoom?: boolean }) => Promise<void>
    >(async () => {
      useCsvChartStore.setState({
        csvData: { columns: ['temperature'], rows: [[25]] },
        filePath: 'new-data.csv',
      });
    });
    useCsvChartStore.setState({ filePath: 'old-data.csv', loadCsvFile });

    render(<CsvLoaderTab />);
    fireEvent.click(screen.getByRole('button', { name: /导出全部 PNG/ }));
    expect(await screen.findByText('PNG export failed')).toBeTruthy();

    fireEvent.click(screen.getByRole('button', { name: /重新加载/ }));

    await waitFor(() => {
      expect(loadCsvFile).toHaveBeenCalledWith('old-data.csv', { resetZoom: true });
      expect(document.body.textContent).not.toContain('PNG export failed');
    });
  });

  it('clears an export error when CSV data is cleared', async () => {
    exportAllPng.mockRejectedValueOnce(new Error('PNG export failed'));
    setCsvData();

    render(<CsvLoaderTab />);
    fireEvent.click(screen.getByRole('button', { name: /导出全部 PNG/ }));
    expect(await screen.findByText('PNG export failed')).toBeTruthy();

    useCsvChartStore.getState().clearData();

    await waitFor(() => expect(document.body.textContent).not.toContain('PNG export failed'));
  });

  it.each([
    ['zh-CN', '导出失败'],
    ['en-US', 'Export failed'],
  ] as const)('shows the localized export failure label and raw error detail in %s', async (language, label) => {
    await i18n.changeLanguage(language);
    exportAllPng.mockRejectedValueOnce(new Error('native export failed'));
    setCsvData();

    render(<CsvLoaderTab />);
    fireEvent.click(screen.getByRole('button', { name: /Export All PNG|导出全部 PNG/ }));

    expect(await screen.findByText(label)).toBeTruthy();
    expect(screen.getByText('native export failed')).toBeTruthy();
  });
});
