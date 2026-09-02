// @vitest-environment happy-dom

import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import i18n from '../../i18n';
import { useCsvChartStore } from '../../stores/csvChartStore';

const { exportAllPng, openFileDialog, chartProps } = vi.hoisted(() => ({
  exportAllPng: vi.fn(),
  openFileDialog: vi.fn(),
  chartProps: { showLineStatistics: undefined as boolean | undefined },
}));

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: openFileDialog,
}));

vi.mock('./MultiLineChart', async () => {
  const ReactModule = await import('react');

  const MockMultiLineChart = ReactModule.forwardRef<
    { exportAllPng: () => Promise<void> },
    {
      onExportError?: (error: Error) => void;
      showLineStatistics?: boolean;
    }
  >(({ onExportError, showLineStatistics }, ref) => {
    chartProps.showLineStatistics = showLineStatistics;
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
    openFileDialog.mockReset();
    chartProps.showLineStatistics = undefined;
  });

  it('renders and toggles the line statistics switch after the no-header option', async () => {
    setCsvData();

    render(<CsvLoaderTab />);

    expect(screen.getByText('无表头')).toBeTruthy();
    expect(screen.getByText('显示线统计')).toBeTruthy();
    const switches = screen.getAllByRole('switch');
    const statisticsSwitch = switches[switches.length - 1]!;
    expect(statisticsSwitch.getAttribute('aria-checked')).toBe('false');
    expect(chartProps.showLineStatistics).toBe(false);

    fireEvent.click(statisticsSwitch);

    await waitFor(() => {
      expect(useCsvChartStore.getState().showLineStatistics).toBe(true);
      expect(chartProps.showLineStatistics).toBe(true);
    });
  });

  it('provides localized line statistics labels', async () => {
    await i18n.changeLanguage('zh-CN');
    expect(i18n.t('waveform:csvLoader.showLineStatistics')).toBe('显示线统计');
    expect(i18n.t('waveform:chart.max')).toBe('最大值');
    expect(i18n.t('waveform:chart.min')).toBe('最小值');
    expect(i18n.t('waveform:chart.avg')).toBe('平均值');
    expect(i18n.t('waveform:chart.diff')).toBe('差值');

    await i18n.changeLanguage('en-US');
    expect(i18n.t('waveform:csvLoader.showLineStatistics')).toBe('Show line statistics');
    expect(i18n.t('waveform:chart.max')).toBe('Max');
    expect(i18n.t('waveform:chart.min')).toBe('Min');
    expect(i18n.t('waveform:chart.avg')).toBe('Avg');
    expect(i18n.t('waveform:chart.diff')).toBe('Diff');
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

  it('disables export while CSV data is loading', async () => {
    setCsvData();
    useCsvChartStore.setState({ isLoading: true });

    render(<CsvLoaderTab />);
    const button = screen.getByRole('button', { name: /导出全部 PNG/ });

    expect(button).toHaveProperty('disabled', true);
    fireEvent.click(button);
    expect(exportAllPng).not.toHaveBeenCalled();

    await act(async () => {
      useCsvChartStore.setState({ isLoading: false });
    });
    await waitFor(() => expect(button).toHaveProperty('disabled', false));
  });

  it('does not start file selection or reload while CSV data is loading', async () => {
    setCsvData();
    const loadCsvFile = vi.fn<
      (filePath: string, options?: { resetZoom?: boolean }) => Promise<void>
    >(async () => undefined);
    openFileDialog.mockResolvedValue('new-data.csv');
    useCsvChartStore.setState({
      filePath: 'old-data.csv',
      isLoading: true,
      loadCsvFile,
    });

    render(<CsvLoaderTab />);
    const selectButton = screen.getByRole('button', { name: /选择文件/ });
    const reloadButton = screen.getByRole('button', { name: /重新加载/ });

    expect(selectButton).toHaveProperty('disabled', true);
    expect(reloadButton).toHaveProperty('disabled', true);

    // Simulate a programmatic event bypassing the DOM disabled property.
    selectButton.removeAttribute('disabled');
    reloadButton.removeAttribute('disabled');
    fireEvent.click(selectButton);
    fireEvent.click(reloadButton);
    await act(async () => {
      await Promise.resolve();
    });

    expect(openFileDialog).not.toHaveBeenCalled();
    expect(loadCsvFile).not.toHaveBeenCalled();
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
