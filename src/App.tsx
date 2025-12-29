import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { FinderLayout } from './components/layout/FinderLayout';
import { ToastContainer } from './components/Toast/ToastContainer';
import { AuthGuard } from './components/auth/AuthGuard';
import { InterruptedJobBanner } from './components/organize/InterruptedJobBanner';
import { useNavigationStore } from './stores/navigation-store';
import { useOrganizeStore } from './stores/organize-store';
import { useSettingsStore } from './stores/settings-store';
import { useAutoRename, useWatcher } from './hooks/useAutoRename';
import { useConvexSettingsSync } from './hooks/useSyncedSettings';
import { getHomeDirectory } from './hooks/useDirectory';
import type { DirectoryContents } from './types/file';
import './App.css';

function AppContent() {
  const { currentPath, navigateTo } = useNavigationStore();
  const { checkForInterruptedJob } = useOrganizeStore();
  const { watchDownloads } = useSettingsStore();
  const [watcherEnabled, setWatcherEnabled] = useState(false);
  const { startWatcher, stopWatcher, getStatus } = useWatcher();
  const hasAutoStarted = useRef(false);

  // Sync settings from Convex on initial load
  useConvexSettingsSync();

  // Initialize with last visited directory or home
  useEffect(() => {
    if (!currentPath) {
      const initializeDirectory = async () => {
        const { lastVisitedPath, setLastVisitedPath } = useSettingsStore.getState();

        // Try last visited path first
        if (lastVisitedPath) {
          try {
            await invoke<DirectoryContents>('read_directory', {
              path: lastVisitedPath,
              showHidden: false
            });
            navigateTo(lastVisitedPath);
            return;
          } catch {
            // Path no longer valid, clear it
            setLastVisitedPath('');
          }
        }

        // Fall back to home directory
        const home = await getHomeDirectory();
        navigateTo(home);
      };

      initializeDirectory();
    }
  }, [currentPath, navigateTo]);

  // Save current path to settings (debounced)
  const saveTimeoutRef = useRef<number | null>(null);

  useEffect(() => {
    if (currentPath) {
      if (saveTimeoutRef.current) {
        clearTimeout(saveTimeoutRef.current);
      }

      saveTimeoutRef.current = window.setTimeout(() => {
        useSettingsStore.getState().setLastVisitedPath(currentPath);
      }, 1000);
    }

    return () => {
      if (saveTimeoutRef.current) {
        clearTimeout(saveTimeoutRef.current);
      }
    };
  }, [currentPath]);

  // Check for interrupted jobs on startup
  useEffect(() => {
    checkForInterruptedJob();
  }, [checkForInterruptedJob]);

  // Check watcher status on mount and auto-start if watchDownloads is enabled
  useEffect(() => {
    const checkAndAutoStart = async () => {
      const status = await getStatus();
      setWatcherEnabled(status.enabled);

      // Auto-start watcher if watchDownloads setting is enabled and watcher is not running
      if (watchDownloads && !status.enabled && !hasAutoStarted.current) {
        hasAutoStarted.current = true;
        const success = await startWatcher();
        if (success) {
          setWatcherEnabled(true);
        }
      }

      // Auto-stop watcher if watchDownloads setting is disabled
      if (!watchDownloads && status.enabled) {
        await stopWatcher();
        setWatcherEnabled(false);
      }
    };

    checkAndAutoStart();

    // Poll for status changes
    const interval = setInterval(async () => {
      const status = await getStatus();
      setWatcherEnabled(status.enabled);

      // Keep watcher in sync with setting
      if (watchDownloads && !status.enabled) {
        const success = await startWatcher();
        if (success) {
          setWatcherEnabled(true);
        }
      }
    }, 5000);

    return () => clearInterval(interval);
  }, [watchDownloads, getStatus, startWatcher, stopWatcher]);

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
