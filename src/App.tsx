import { GlobalShell } from './components/layout/GlobalShell';
import { ErrorBoundary } from './components/common/ErrorBoundary';

export default function App() {
  return (
    <ErrorBoundary>
      <GlobalShell />
    </ErrorBoundary>
  );
}
