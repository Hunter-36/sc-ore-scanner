import { useEffect } from 'react';
import { useOreStore, ScanResult } from '../store/useOreStore';

/**
 * v2: the Rust core runs the scan loop in-process and emits "scan-result" events.
 * This replaces the old WebSocket — there's no separate backend to connect to.
 * `connected` flips true on the first event (after the ~15-20s OCR model load).
 */
export function useScanEvents() {
  const { updateFromScan, setConnected } = useOreStore();

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let active = true;

    (async () => {
      try {
        const { listen } = await import('@tauri-apps/api/event');
        const stop = await listen<ScanResult>('scan-result', (event) => {
          setConnected(true);
          updateFromScan(event.payload);
        });
        if (active) {
          unlisten = stop;
        } else {
          stop();
        }
      } catch (err) {
        // Not running inside Tauri (e.g. a plain browser during dev/tests).
        console.warn('Tauri events unavailable:', err);
      }
    })();

    return () => {
      active = false;
      if (unlisten) unlisten();
    };
  }, []);

  // Dev/test only: inject a scan result without the Rust core via
  //   window.dispatchEvent(new CustomEvent('mock-scan', { detail: <ScanResult> }))
  // This block is dead-code-eliminated from production builds (DEV === false),
  // so it never ships. Used by the Playwright e2e suite and handy for UI dev.
  useEffect(() => {
    if (!import.meta.env.DEV) return;
    const onMock = (e: Event) => {
      const detail = (e as CustomEvent<ScanResult>).detail;
      if (!detail) return;
      setConnected(true);
      updateFromScan(detail);
    };
    window.addEventListener('mock-scan', onMock);
    return () => window.removeEventListener('mock-scan', onMock);
  }, []);

  return { connected: useOreStore((s) => s.connected) };
}
