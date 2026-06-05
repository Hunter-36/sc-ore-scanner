import { Component, ErrorInfo, ReactNode } from 'react';

interface Props {
  children: ReactNode;
}
interface State {
  error: Error | null;
}

/**
 * Top-level boundary so a render-time throw shows a small message in the overlay
 * instead of a white screen with no recovery.
 */
export class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error('Overlay crashed:', error, info);
  }

  render() {
    if (this.state.error) {
      return (
        <div className="overlay">
          <div className="message">
            <p>Overlay error</p>
            <p className="hint">{this.state.error.message}</p>
            <p className="hint">See logs/scanner.log; restart the app.</p>
          </div>
        </div>
      );
    }
    return this.props.children;
  }
}
