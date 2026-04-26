import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
import { getAllWebviewWindows } from '@tauri-apps/api/webviewWindow';

const SPO2_REF_WINDOW_LABEL = 'spo2-ref-window';

export async function openSpo2RefWindow(initialValue?: number): Promise<WebviewWindow | null> {
  try {
    const existingWindow = await getSpo2RefWindow();
    if (existingWindow) {
      await existingWindow.setFocus();
      return existingWindow;
    }

    const url = initialValue !== undefined 
      ? `/spo2-ref?value=${encodeURIComponent(initialValue)}`
      : '/spo2-ref';

    const webview = new WebviewWindow(SPO2_REF_WINDOW_LABEL, {
      url,
      title: '血氧金标配置',
      width: 320,
      height: 300,
      resizable: false,
      decorations: true,
      alwaysOnTop: true,
      skipTaskbar: true,
      center: true,
    });

    webview.once('tauri://created', () => {
      console.debug('[Spo2RefWindow] 窗口创建成功');
    });

    webview.once('tauri://error', (e) => {
      console.error('[Spo2RefWindow] 窗口创建失败:', e);
    });

    return webview;
  } catch (err) {
    console.error('[Spo2RefWindow] 打开窗口失败:', err);
    return null;
  }
}

export async function closeSpo2RefWindow(): Promise<void> {
  try {
    const window = await getSpo2RefWindow();
    if (window) {
      await window.close();
    }
  } catch (err) {
    console.error('[Spo2RefWindow] 关闭窗口失败:', err);
  }
}

export async function getSpo2RefWindow(): Promise<WebviewWindow | null> {
  try {
    const windows = await getAllWebviewWindows();
    return windows.find((w) => w.label === SPO2_REF_WINDOW_LABEL) || null;
  } catch (err) {
    console.error('[Spo2RefWindow] 获取窗口失败:', err);
    return null;
  }
}

export async function isSpo2RefWindowOpen(): Promise<boolean> {
  const window = await getSpo2RefWindow();
  return window !== null;
}
