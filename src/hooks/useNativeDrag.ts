import { useRef, useCallback } from 'react';
import { useNavigationStore } from '../stores/navigation-store';

/** Spring loading delay in milliseconds (1.5 seconds like macOS Finder) */
const SPRING_LOAD_DELAY = 1500;

interface UseNativeDragReturn {
  /**
   * Call when drag enters a potential drop target.
   * Starts spring loading timer for directories.
   */
  handleDragEnter: (path: string, isDirectory: boolean) => void;

  /**
   * Call when drag leaves a drop target.
   * Uses counter pattern to prevent flicker from child elements.
   */
  handleDragLeave: () => void;

  /**
   * Call when drop occurs.
   * Clears spring loading timer and resets counter.
   */
  handleDrop: () => void;

  /**
   * Ref to track enter/leave balance for anti-flicker.
   * When counter reaches 0, the element is truly left.
   */
  dragCounter: React.MutableRefObject<number>;
}

/**
 * Hook for native HTML5 drag-and-drop behavior with:
 * - Spring loading: Hover on folder for 1.5s to navigate into it
 * - Anti-flicker: Counter pattern to prevent highlight flashing
 *
 * Usage:
 * ```tsx
 * const { handleDragEnter, handleDragLeave, handleDrop, dragCounter } = useNativeDrag();
 *
 * <div
 *   onDragEnter={(e) => {
 *     handleDragEnter(entry.path, entry.isDirectory);
 *     if (dragCounter.current === 1) setIsHighlighted(true);
 *   }}
 *   onDragLeave={() => {
 *     handleDragLeave();
 *     if (dragCounter.current === 0) setIsHighlighted(false);
 *   }}
 *   onDrop={() => {
 *     handleDrop();
 *     setIsHighlighted(false);
 *     // ... execute drop
 *   }}
 * />
 * ```
 */
export function useNativeDrag(): UseNativeDragReturn {
  const { navigateTo } = useNavigationStore();

  // Spring loading timer ref
  const springTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Anti-flicker counter ref
  // Problem: Child elements cause enter/leave to fire repeatedly
  // Solution: Only truly "leave" when counter returns to 0
  const dragCounter = useRef(0);

  // Track current spring-load target to avoid duplicate timers
  const springTargetRef = useRef<string | null>(null);

  const clearSpringTimer = useCallback(() => {
    if (springTimerRef.current) {
      clearTimeout(springTimerRef.current);
      springTimerRef.current = null;
    }
    springTargetRef.current = null;
  }, []);

  const handleDragEnter = useCallback(
    (path: string, isDirectory: boolean) => {
      dragCounter.current++;

      // Only start spring loading for directories
      if (!isDirectory) return;

      // Don't restart timer if already targeting this path
      if (springTargetRef.current === path) return;

      // Clear any existing timer
      clearSpringTimer();

      // Start new spring loading timer
      springTargetRef.current = path;
      springTimerRef.current = setTimeout(() => {
        navigateTo(path);
        springTimerRef.current = null;
        springTargetRef.current = null;
      }, SPRING_LOAD_DELAY);
    },
    [navigateTo, clearSpringTimer]
  );

  const handleDragLeave = useCallback(() => {
    dragCounter.current--;

    // Only clear timer when truly leaving (counter reaches 0)
    if (dragCounter.current === 0) {
      clearSpringTimer();
    }
  }, [clearSpringTimer]);

  const handleDrop = useCallback(() => {
    // Reset counter and clear timer on drop
    dragCounter.current = 0;
    clearSpringTimer();
  }, [clearSpringTimer]);

  return {
    handleDragEnter,
    handleDragLeave,
    handleDrop,
    dragCounter,
  };
}
