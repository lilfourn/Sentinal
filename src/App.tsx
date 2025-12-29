import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { FinderLayout } from './components/layout/FinderLayout';
import { ToastContainer } from './components/Toast/ToastContainer';
import { AuthGuard } from './components/auth/AuthGuard';
import { InterruptedJobBanner } from './components/organize/InterruptedJobBanner';
import { useNavigationStore } from './stores/navigation-store';
import { useOrganizeStore } from './stores/organize-store';
import { useAutoRename } from './hooks/useAutoRename';
import { getHomeDirectory } from './hooks/useDirectory';
import './App.css';

function AppContent() {
  const { currentPath, navigateTo } = useNavigationStore();
  const { checkForInterruptedJob } = useOrganizeStore();
  const [watcherEnabled, setWatcherEnabled] = useState(false);

  // Initialize with home directory
  useEffect(() => {
    if (!currentPath) {
      getHomeDirectory().then((home) => {
        navigateTo(home);
      });
    }
  }, [currentPath, navigateTo]);

  // Check for interrupted jobs on startup
  useEffect(() => {
    checkForInterruptedJob();
  }, [checkForInterruptedJob]);

  // Check watcher status on mount
  useEffect(() => {
    invoke<{ enabled: boolean }>('get_watcher_status').then((status) => {
      setWatcherEnabled(status.enabled);
    });

    // Poll for status changes
    const interval = setInterval(() => {
      invoke<{ enabled: boolean }>('get_watcher_status').then((status) => {
        setWatcherEnabled(status.enabled);
      });
    }, 5000);

    return () => clearInterval(interval);
  }, []);

  // Enable auto-rename when watcher is active
  useAutoRename(watcherEnabled);

  return (
    <>
      <FinderLayout />
      <ToastContainer />
      <InterruptedJobBanner />
    </>
  );
}

function App() {
  // Check if Clerk is configured
  const isClerkConfigured = Boolean(import.meta.env.VITE_CLERK_PUBLISHABLE_KEY);

  // If Clerk is not configured, run without auth guard
  if (!isClerkConfigured) {
    return <AppContent />;
  }

  // Wrap with AuthGuard when Clerk is configured
  return (
    <AuthGuard>
      <AppContent />
    </AuthGuard>
  );
}

export default App;
