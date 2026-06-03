import { useEffect, useRef } from 'react';
import { useOreStore, ScanResult } from '../store/useOreStore';

const WS_URL = 'ws://127.0.0.1:8765/ws';
const RECONNECT_DELAY = 3000; // 3 seconds

export function useWebSocket() {
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimeoutRef = useRef<number | null>(null);
  const { setConnected, updateFromScan, clear } = useOreStore();

  const connect = () => {
    // Clean up existing connection
    if (wsRef.current) {
      wsRef.current.close();
    }

    console.log('Connecting to backend WebSocket...');
    const ws = new WebSocket(WS_URL);
    wsRef.current = ws;

    ws.onopen = () => {
      console.log('WebSocket connected');
      setConnected(true);

      // Clear any pending reconnect
      if (reconnectTimeoutRef.current) {
        clearTimeout(reconnectTimeoutRef.current);
        reconnectTimeoutRef.current = null;
      }
    };

    ws.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data) as ScanResult;

        // Update store with scan results
        updateFromScan(data);

        console.log('Scan result:', Object.keys(data.ores).length, 'ores detected');
      } catch (error) {
        console.error('Failed to parse WebSocket message:', error);
      }
    };

    ws.onerror = (error) => {
      console.error('WebSocket error:', error);
    };

    ws.onclose = () => {
      console.log('WebSocket disconnected');
      setConnected(false);
      clear();
      wsRef.current = null;

      // Attempt to reconnect
      if (!reconnectTimeoutRef.current) {
        console.log(`Reconnecting in ${RECONNECT_DELAY / 1000}s...`);
        reconnectTimeoutRef.current = window.setTimeout(() => {
          reconnectTimeoutRef.current = null;
          connect();
        }, RECONNECT_DELAY);
      }
    };
  };

  useEffect(() => {
    connect();

    // Cleanup on unmount
    return () => {
      if (reconnectTimeoutRef.current) {
        clearTimeout(reconnectTimeoutRef.current);
      }
      if (wsRef.current) {
        wsRef.current.close();
      }
    };
  }, []);

  return { connected: useOreStore((s) => s.connected) };
}
