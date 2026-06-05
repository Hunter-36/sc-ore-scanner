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

  return { connected: useOreStore((s) => s.connected) };
}
