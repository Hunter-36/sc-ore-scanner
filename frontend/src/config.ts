// Central backend connection config. Keep the host/port in one place so the
// WebSocket and HTTP calls can't drift apart if the port ever changes.
export const BACKEND_HOST = '127.0.0.1';
export const BACKEND_PORT = 8765;

export const BACKEND_HTTP = `http://${BACKEND_HOST}:${BACKEND_PORT}`;
export const BACKEND_WS = `ws://${BACKEND_HOST}:${BACKEND_PORT}/ws`;
