import { GlobalShell } from './components/layout/GlobalShell';
import { ErrorBoundary } from './components/common/ErrorBoundary';
import { StartupLoadingModal } from './components/common/StartupLoadingModal';

export default function App() {
  return (
    <ErrorBoundary>
      <StartupLoadingModal />
      <GlobalShell />
    </ErrorBoundary>
  );
}
