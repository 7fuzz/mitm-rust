import { Component, ErrorInfo, ReactNode } from 'react';

interface Props {
  children: ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
}

export class ErrorBoundary extends Component<Props, State> {
  public state: State = {
    hasError: false,
    error: null,
  };

  public static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  public componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error('Uncaught React UI error:', error, errorInfo);
  }

  public render() {
    if (this.state.hasError) {
      return (
        <div className="h-screen w-screen flex flex-col items-center justify-center bg-background text-foreground p-6 font-mono text-xs select-none">
          <div className="max-w-md w-full bg-surface border border-rose-500/40 p-4 rounded-lg shadow-lg space-y-3">
            <div className="flex items-center gap-2 text-rose-500 font-bold text-sm">
              <span>UI Rendering Exception</span>
            </div>
            <p className="text-muted-foreground text-xs font-sans">
              An unexpected error occurred during rendering. Click below to reload the workspace.
            </p>
            <div className="p-2 bg-background border border-border rounded text-[11px] text-rose-400 overflow-x-auto">
              {this.state.error?.message || 'Unknown Error'}
            </div>
            <button
              onClick={() => window.location.reload()}
              className="w-full py-1.5 bg-primary text-primary-foreground font-semibold rounded text-xs hover:bg-primary-hover transition-colors cursor-pointer"
            >
              Reload Application
            </button>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}
