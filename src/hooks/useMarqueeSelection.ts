import { useState, useCallback, useRef, useEffect } from 'react';
import { useSelectionStore } from '../stores/selection-store';

export interface SelectionRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

interface ItemRef {
  path: string;
  element: HTMLElement;
}

export function useMarqueeSelection(containerRef: React.RefObject<HTMLElement | null>) {
  const [isDragging, setIsDragging] = useState(false);
  const [, forceUpdate] = useState({});

  // All mutable state in refs to avoid closure issues
  const startRef = useRef({ x: 0, y: 0 });
  const currentRef = useRef({ x: 0, y: 0 });
  const itemRefsRef = useRef<ItemRef[]>([]);
  const modifiersRef = useRef({ meta: false, shift: false });
  const initialSelectionRef = useRef<Set<string>>(new Set());
  const isDraggingRef = useRef(false);
  const justFinishedDraggingRef = useRef(false);

  const { selectMultiple, clearSelection } = useSelectionStore();
  const selectedPaths = useSelectionStore((state) => state.selectedPaths);

  // Store latest functions in refs to avoid stale closures
  const selectMultipleRef = useRef(selectMultiple);
  const clearSelectionRef = useRef(clearSelection);

  useEffect(() => {
    selectMultipleRef.current = selectMultiple;
    clearSelectionRef.current = clearSelection;
  });

  // Calculate selection rect from current ref values
  const getSelectionRect = useCallback((): SelectionRect | null => {
    if (!containerRef.current) return null;

    const container = containerRef.current;
    const containerRect = container.getBoundingClientRect();
    const scrollLeft = container.scrollLeft;
    const scrollTop = container.scrollTop;

    const startX = startRef.current.x - containerRect.left + scrollLeft;
    const startY = startRef.current.y - containerRect.top + scrollTop;
    const currentX = currentRef.current.x - containerRect.left + scrollLeft;
    const currentY = currentRef.current.y - containerRect.top + scrollTop;

    return {
      x: Math.min(startX, currentX),
      y: Math.min(startY, currentY),
      width: Math.abs(currentX - startX),
      height: Math.abs(currentY - startY),
    };
  }, [containerRef]);

  // AABB intersection test
  const rectsIntersect = useCallback(
    (itemRect: DOMRect, selRect: SelectionRect, containerRect: DOMRect, scrollLeft: number, scrollTop: number): boolean => {
      const itemLeft = itemRect.left - containerRect.left + scrollLeft;
      const itemTop = itemRect.top - containerRect.top + scrollTop;
      const itemRight = itemLeft + itemRect.width;
      const itemBottom = itemTop + itemRect.height;

      return !(
        itemRight < selRect.x ||
        itemLeft > selRect.x + selRect.width ||
        itemBottom < selRect.y ||
        itemTop > selRect.y + selRect.height
      );
    },
    []
  );

  // Get paths of items intersecting with selection rect
  const getIntersectingPaths = useCallback((): string[] => {
    const selRect = getSelectionRect();
    if (!selRect || !containerRef.current) return [];

    // Skip tiny selections (likely just clicks)
    if (selRect.width < 5 && selRect.height < 5) return [];

    const container = containerRef.current;
    const containerRect = container.getBoundingClientRect();
    const scrollLeft = container.scrollLeft;
    const scrollTop = container.scrollTop;

    return itemRefsRef.current
      .filter((item) => {
        // Get FRESH bounding rect
        const itemRect = item.element.getBoundingClientRect();
        return rectsIntersect(itemRect, selRect, containerRect, scrollLeft, scrollTop);
      })
      .map((item) => item.path);
  }, [getSelectionRect, rectsIntersect, containerRef]);

  // Store ref to getIntersectingPaths for use in event handlers
  const getIntersectingPathsRef = useRef(getIntersectingPaths);
  useEffect(() => {
    getIntersectingPathsRef.current = getIntersectingPaths;
  });

  // Store element references
  const updateItemPositions = useCallback((items: { path: string; element: HTMLElement }[]) => {
    itemRefsRef.current = items.map((item) => ({
      path: item.path,
      element: item.element,
    }));
  }, []);

  // Start drag - registers listeners SYNCHRONOUSLY to avoid race conditions
  const startDrag = useCallback((e: React.MouseEvent) => {
    if (e.button !== 0) return;

    // Store modifiers and initial selection
    modifiersRef.current = {
      meta: e.metaKey || e.ctrlKey,
      shift: e.shiftKey,
    };
    initialSelectionRef.current = new Set(selectedPaths);

    // Initialize coordinates
    startRef.current = { x: e.clientX, y: e.clientY };
    currentRef.current = { x: e.clientX, y: e.clientY };
    isDraggingRef.current = true;

    // Clear selection if no modifier held
    if (!modifiersRef.current.meta && !modifiersRef.current.shift) {
      clearSelectionRef.current();
      initialSelectionRef.current = new Set();
    }

    setIsDragging(true);

    // Update selection based on current marquee position
    const updateSelection = () => {
      const intersectingPaths = getIntersectingPathsRef.current();
      const additive = modifiersRef.current.meta || modifiersRef.current.shift;

      if (additive) {
        // Combine initial selection with current marquee
        const combined = [...new Set([...initialSelectionRef.current, ...intersectingPaths])];
        selectMultipleRef.current(combined, false);
      } else {
        // Replace selection with marquee items
        if (intersectingPaths.length > 0) {
          selectMultipleRef.current(intersectingPaths, false);
        }
      }
    };

    // Mouse move handler - updates in real-time
    const handleMouseMove = (ev: MouseEvent) => {
      if (!isDraggingRef.current) return;

      currentRef.current = { x: ev.clientX, y: ev.clientY };
      updateSelection(); // Real-time selection update!
      forceUpdate({}); // Update visual rect
    };

    // Mouse up handler - cleanup
    const handleMouseUp = () => {
      if (!isDraggingRef.current) return;

      // Final selection update
      updateSelection();

      // Mark that we just finished dragging - prevents click handler from clearing selection
      justFinishedDraggingRef.current = true;
      // Reset after a tick to allow click event to check it first
      setTimeout(() => {
        justFinishedDraggingRef.current = false;
      }, 0);

      // Cleanup
      isDraggingRef.current = false;
      startRef.current = { x: 0, y: 0 };
      currentRef.current = { x: 0, y: 0 };
      setIsDragging(false);

      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
    };

    // Add listeners SYNCHRONOUSLY - this is critical!
    // Using useEffect would create a race condition where mouseup
    // could fire before the listener is registered
    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);
  }, [selectedPaths]);

  return {
    isDragging,
    justFinishedDragging: justFinishedDraggingRef,
    selectionRect: isDragging ? getSelectionRect() : null,
    startDrag,
    updateItemPositions,
    getIntersectingPaths,
  };
}
