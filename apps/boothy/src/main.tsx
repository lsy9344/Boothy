import React from 'react';
import { invoke } from '@tauri-apps/api/core';
import { createRoot } from 'react-dom/client';
import App from './App';
import './styles.css';
import { Invokes } from './components/ui/AppProperties';

type FrontendLogLevel = 'debug' | 'info' | 'warn' | 'error';

const sendFrontendLog = (level: FrontendLogLevel, message: string, context?: Record<string, any>) => {
  invoke(Invokes.BoothyLogFrontend, {
    level,
    message,
    context: context ?? null,
  }).catch(() => {});
};

window.addEventListener('error', (event) => {
  sendFrontendLog('error', 'window-error', {
    message: event.message,
    filename: event.filename,
    lineno: event.lineno,
    colno: event.colno,
    stack: event.error?.stack,
  });
});

window.addEventListener('unhandledrejection', (event) => {
  let reason = '';
  try {
    reason = typeof event.reason === 'string' ? event.reason : JSON.stringify(event.reason);
  } catch {
    reason = String(event.reason);
  }
  sendFrontendLog('error', 'unhandledrejection', { reason });
});

class AppErrorBoundary extends React.Component<{ children: React.ReactNode }, { hasError: boolean }> {
  constructor(props: { children: React.ReactNode }) {
    super(props);
    this.state = { hasError: false };
  }

  componentDidCatch(error: Error, info: React.ErrorInfo) {
    sendFrontendLog('error', 'react-error-boundary', {
      message: error.message,
      stack: error.stack,
      componentStack: info.componentStack,
    });
    this.setState({ hasError: true });
  }

  render() {
    if (this.state.hasError) {
      return (
        <div className="h-screen w-screen flex items-center justify-center bg-black text-white">
          <div className="text-center space-y-2">
            <div className="text-lg font-semibold">Something went wrong</div>
            <div className="text-sm opacity-80">Check the logs for details.</div>
          </div>
        </div>
      );
    }
    return this.props.children;
  }
}

const rootElement = document.getElementById('root');
if (!rootElement) {
  sendFrontendLog('error', 'root-element-missing');
  throw new Error('Root element not found');
}

sendFrontendLog('info', 'frontend-bootstrap', { location: window.location.href });

const root = createRoot(rootElement);
root.render(
  <React.StrictMode>
    <AppErrorBoundary>
      <App />
    </AppErrorBoundary>
  </React.StrictMode>,
);
