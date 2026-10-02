import { Component, ErrorInfo, ReactNode } from 'react';

interface Props {
  children: ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
  errorInfo: ErrorInfo | null;
  copied: boolean;
}

export class ErrorBoundary extends Component<Props, State> {
  public state: State = {
    hasError: false,
    error: null,
    errorInfo: null,
    copied: false,
  };

  public static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error, errorInfo: null, copied: false };
  }

  public componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error('Uncaught React UI error:', error, errorInfo);
    this.setState({ errorInfo });
  }

  private handleCopy = () => {
    const text = `Error: ${this.state.error?.message || 'Unknown Error'}\n\nStack:\n${
      this.state.error?.stack || this.state.errorInfo?.componentStack || 'No stack trace'
    }`;
    navigator.clipboard.writeText(text);
    this.setState({ copied: true });
    setTimeout(() => this.setState({ copied: false }), 2000);
  };

  public render() {
    if (this.state.hasError) {
      return (
        <div className="h-screen w-screen flex flex-col items-center justify-center bg-background text-foreground p-6 font-mono text-xs select-text">
          <div className="max-w-lg w-full bg-surface border border-rose-500/40 p-5 rounded-lg shadow-xl space-y-4 select-text">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2 text-rose-500 font-bold text-sm">
                <span>UI Rendering Exception</span>
              </div>
              <button
                onClick={this.handleCopy}
                className="px-2.5 py-1 rounded bg-neutral-subtle border border-border text-foreground hover:bg-surface font-sans font-medium transition-colors text-xs cursor-pointer select-none"
              >
                {this.state.copied ? '✓ Copied to Clipboard' : 'Copy Error Details'}
              </button>
            </div>

            <p className="text-muted-foreground text-xs font-sans">
              An unexpected error occurred during rendering. You can select and copy the stack trace below or click below to reload.
            </p>

            <div className="p-3 bg-background border border-border rounded text-2xs text-rose-400 font-mono overflow-auto max-h-60 select-text whitespace-pre-wrap break-all">
              <strong>{this.state.error?.name || 'Error'}: {this.state.error?.message || 'Unknown Error'}</strong>
              {this.state.error?.stack && (
                <div className="mt-2 text-muted-foreground text-3xs opacity-80 select-text">
                  {this.state.error.stack}
                </div>
              )}
            </div>

            <div className="flex gap-2">
              <button
                onClick={() => window.location.reload()}
                className="flex-1 py-1.5 bg-primary text-primary-foreground font-semibold rounded text-xs hover:bg-primary-hover transition-colors cursor-pointer select-none"
              >
                Reload Application
              </button>
            </div>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}
