import { useEffect, useCallback, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { showRenameToast, showError } from '../stores/toast-store';

interface FileCreatedEvent {
  id: string;
  eventType: string;
  path: string;
  fileName: string;
  extension: string | null;
  size: number;
  contentPreview: string | null;
}

interface RenameSuggestion {
  originalName: string;
  suggestedName: string;
  path: string;
}

interface RenameResult {
  success: boolean;
  oldPath: string;
  newPath: string;
}

export function useAutoRename(enabled: boolean) {
  const processingRef = useRef<Set<string>>(new Set());

  const handleFileCreated = useCallback(async (event: FileCreatedEvent) => {
    // Skip if already processing
    if (processingRef.current.has(event.path)) {
      return;
    }

    processingRef.current.add(event.path);

    try {
      // Get rename suggestion from AI
      const suggestion = await invoke<RenameSuggestion>('get_rename_suggestion', {
        path: event.path,
        filename: event.fileName,
        extension: event.extension,
        size: event.size,
        contentPreview: event.contentPreview,
      });

      // Skip if suggestion is the same as original
      if (suggestion.suggestedName === suggestion.originalName) {
        return;
      }

      // Apply the rename
      const result = await invoke<RenameResult>('apply_rename', {
        oldPath: event.path,
        newName: suggestion.suggestedName,
      });

      if (result.success) {
        // Show toast with undo option
        showRenameToast(
          'File renamed',
          `${suggestion.originalName} → ${suggestion.suggestedName}`,
          async () => {
            try {
              await invoke('undo_rename', {
                currentPath: result.newPath,
                originalPath: result.oldPath,
              });
            } catch (error) {
              showError('Failed to undo', String(error));
            }
          }
        );
      }
    } catch (error) {
      console.error('Auto-rename failed:', error);
      // Don't show error toast for every failure - could be API key not configured
    } finally {
      processingRef.current.delete(event.path);
    }
  }, []);

  useEffect(() => {
    if (!enabled) return;

    const unlisten = listen<FileCreatedEvent>('sentinel://file-created', (event) => {
      handleFileCreated(event.payload);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [enabled, handleFileCreated]);
}

// Hook to control the watcher
export function useWatcher() {
  const startWatcher = useCallback(async (path?: string) => {
    try {
      await invoke('start_downloads_watcher', { path });
      return true;
    } catch (error) {
      showError('Failed to start watcher', String(error));
      return false;
    }
  }, []);

  const stopWatcher = useCallback(async () => {
    try {
      await invoke('stop_downloads_watcher');
      return true;
    } catch (error) {
      showError('Failed to stop watcher', String(error));
      return false;
    }
  }, []);

  const getStatus = useCallback(async () => {
    return invoke<{ enabled: boolean; watchingPath: string | null }>('get_watcher_status');
  }, []);

  return { startWatcher, stopWatcher, getStatus };
}
