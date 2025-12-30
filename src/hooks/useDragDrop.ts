import { useState, useCallback, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { wouldCreateCycleMulti, getCycleReason } from '../lib/cycle-detection';
import { mapErrorToReason } from '../types/drag-drop';
import type { FileEntry } from '../types/file';
import type { DragState, DropTarget, DropInvalidReason } from '../types/drag-drop';

interface UseDragDropOptions {
  /** Callback when drop completes successfully */
  onDropComplete?: (newPaths: string[], isCopy: boolean) => void;
  /** Callback on drop error */
  onDropError?: (error: string) => void;
}

interface UseDragDropReturn {
  /** Current drag state (null if not dragging) */
  dragState: DragState | null;
  /** Currently hovered drop target */
  dropTarget: DropTarget | null;

  /** Start dragging - call from onMouseDown when initiating drag */
  startDrag: (items: FileEntry[], sourceDirectory: string) => void;
  /** Update drag position - called automatically from global mousemove */
  updateDragPosition: (x: number, y: number, altKey: boolean) => void;
  /** Set current drop target - call when hovering over a directory */
  setDropTarget: (path: string | null, isDirectory: boolean) => void;
  /** Execute the drop - call from onMouseUp on valid target */
  executeDrop: () => Promise<boolean>;
  /** Cancel the drag */
  cancelDrag: () => void;

  /** Check if currently dragging */
  isDragging: boolean;
  /** Check if current drop target is valid */
  isValidTarget: boolean;
}

export function useDragDrop(options: UseDragDropOptions = {}): UseDragDropReturn {
  const { onDropComplete, onDropError } = options;

  const [dragState, setDragState] = useState<DragState | null>(null);
  const [dropTarget, setDropTargetState] = useState<DropTarget | null>(null);

  const startDrag = useCallback(
    (items: FileEntry[], sourceDirectory: string) => {
      setDragState({
        items,
        sourceDirectory,
        isCopy: false,
        position: { x: 0, y: 0 },
      });
    },
    []
  );

  const updateDragPosition = useCallback((x: number, y: number, altKey: boolean) => {
    setDragState((prev) =>
      prev
        ? {
            ...prev,
            position: { x, y },
            isCopy: altKey,
          }
        : null
    );
  }, []);

  const setDropTarget = useCallback(
    (path: string | null, isDirectory: boolean) => {
      if (!path || !dragState) {
        setDropTargetState(null);
        return;
      }

      const sourcePaths = dragState.items.map((item) => item.path);

      // Quick frontend validation for immediate feedback
      if (!isDirectory) {
        setDropTargetState({ path, isValid: false, reason: 'not_directory' });
        return;
      }

      // Check for cycles synchronously
      if (wouldCreateCycleMulti(sourcePaths, path)) {
        const reason = getCycleReason(sourcePaths, path) as DropInvalidReason;
        setDropTargetState({ path, isValid: false, reason });
        return;
      }

      // Optimistically show as valid, then validate with backend
      setDropTargetState({ path, isValid: true });

      // Async backend validation for edge cases (symlinks, permissions, etc.)
      invoke('validate_drag_drop', {
        sources: sourcePaths,
        target: path,
      })
        .then(() => {
          // Already set as valid, no change needed
        })
        .catch((error: unknown) => {
          // Update with backend error
          const errorObj = error as { type?: string };
          const reason = mapErrorToReason(errorObj?.type);
          setDropTargetState((prev) =>
            prev?.path === path ? { path, isValid: false, reason } : prev
          );
        });
    },
    [dragState]
  );

  const executeDrop = useCallback(async (): Promise<boolean> => {
    if (!dragState || !dropTarget?.isValid) {
      return false;
    }

    const sourcePaths = dragState.items.map((item) => item.path);
    const command = dragState.isCopy ? 'copy_files_batch' : 'move_files_batch';

    try {
      const newPaths = await invoke<string[]>(command, {
        sources: sourcePaths,
        targetDirectory: dropTarget.path,
      });

      onDropComplete?.(newPaths, dragState.isCopy);
      setDragState(null);
      setDropTargetState(null);
      return true;
    } catch (error) {
      const errorMessage =
        error instanceof Error
          ? error.message
          : typeof error === 'object' && error !== null && 'message' in error
            ? String((error as { message: unknown }).message)
            : String(error);
      onDropError?.(errorMessage);
      setDragState(null);
      setDropTargetState(null);
      return false;
    }
  }, [dragState, dropTarget, onDropComplete, onDropError]);

  const cancelDrag = useCallback(() => {
    setDragState(null);
    setDropTargetState(null);
  }, []);

  // Global mouse and keyboard event handlers
  useEffect(() => {
    if (!dragState) return;

    const handleMouseMove = (e: MouseEvent) => {
      updateDragPosition(e.clientX, e.clientY, e.altKey);
    };

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        cancelDrag();
      }
      // Update copy mode on Alt key
      if (e.key === 'Alt') {
        setDragState((prev) => (prev ? { ...prev, isCopy: true } : null));
      }
    };

    const handleKeyUp = (e: KeyboardEvent) => {
      if (e.key === 'Alt') {
        setDragState((prev) => (prev ? { ...prev, isCopy: false } : null));
      }
    };

    // Handle losing focus (e.g., switching windows)
    const handleBlur = () => {
      cancelDrag();
    };

    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('keydown', handleKeyDown);
    document.addEventListener('keyup', handleKeyUp);
    window.addEventListener('blur', handleBlur);

    return () => {
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('keydown', handleKeyDown);
      document.removeEventListener('keyup', handleKeyUp);
      window.removeEventListener('blur', handleBlur);
    };
  }, [dragState, updateDragPosition, cancelDrag]);

  return {
    dragState,
    dropTarget,
    startDrag,
    updateDragPosition,
    setDropTarget,
    executeDrop,
    cancelDrag,
    isDragging: dragState !== null,
    isValidTarget: dropTarget?.isValid ?? false,
  };
}
