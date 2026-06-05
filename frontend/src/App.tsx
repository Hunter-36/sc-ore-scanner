import { useEffect, useState } from 'react';
import { Overlay } from './components/Overlay';
import { Calibrate } from './components/Calibrate';
import './App.css';

function App() {
  // The calibration overlay is a second Tauri window (label "calibrate") that
  // loads this same bundle; route to it by window label. Defaults to the overlay
  // (and stays there in the browser/tests, where Tauri APIs are unavailable).
  const [isCalibrate, setIsCalibrate] = useState(false);

  useEffect(() => {
    (async () => {
      try {
        const { getCurrentWindow } = await import('@tauri-apps/api/window');
        if (getCurrentWindow().label === 'calibrate') {
          setIsCalibrate(true);
          return;
        }
      } catch {
        // Not in Tauri — fall through to the browser fallback below.
      }
      // Browser/dev/test fallback: ?calibrate previews the calibration UI.
      // (The real calibrate window is created by Rust without a query string,
      // so the production main window never matches this.)
      if (window.location.search.includes('calibrate')) {
        setIsCalibrate(true);
      }
    })();
  }, []);

  return isCalibrate ? <Calibrate /> : <Overlay />;
}

export default App;
