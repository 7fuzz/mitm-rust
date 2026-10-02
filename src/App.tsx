import { GlobalShell } from './components/layout/GlobalShell';
import { ErrorBoundary } from './components/common/ErrorBoundary';
import { StartupLoadingModal } from './components/common/StartupLoadingModal';
import { InstanceGate } from './components/common/InstanceGate';

export default function App() {
  return (
    <ErrorBoundary>
      <InstanceGate>
        <StartupLoadingModal />
        <GlobalShell />
      </InstanceGate>
    </ErrorBoundary>
  );
}
