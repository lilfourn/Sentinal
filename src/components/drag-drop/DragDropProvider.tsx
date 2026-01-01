import { createContext, useContext, type ReactNode } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useDragDrop } from '../../hooks/useDragDrop';
import { useSelectionStore } from '../../stores/selection-store';
import type { DragState, DropTarget } from '../../types/drag-drop';

interface DragDropContextValue {
  /** Current drag state (null if not dragging) */
  dragState: DragState | null;
  /** Currently hovered drop target */
  dropTarget: DropTarget | null;
  /** Start dragging items */
  startDrag: (items: import('../../types/file').FileEntry[], sourceDirectory: string) => void;
  /** Set current drop target */
  setDropTarget: (path: string | null, isDirectory: boolean) => void;
  /** Execute the drop. Pass targetPath to override state. */
  executeDrop: (targetPath?: string) => Promise<boolean>;
  /** Cancel the drag */
  cancelDrag: () => void;
  /** Set copy mode (Alt key held) */
  setCopyMode: (isCopy: boolean) => void;
  /** Check if currently dragging */
  isDragging: boolean;
  /** Check if current drop target is valid */
  isValidTarget: boolean;
}

const DragDropContext = createContext<DragDropContextValue | null>(null);

interface DragDropProviderProps {
  children: ReactNode;
}

export function DragDropProvider({ children }: DragDropProviderProps) {
  const queryClient = useQueryClient();
  const clearSelection = useSelectionStore((state) => state.clearSelection);

  const dragDrop = useDragDrop({
    onDropComplete: (newPaths, isCopy) => {
      console.log('[DragDropProvider] onDropComplete:', { newPaths, isCopy });
      // Invalidate directory queries to refresh the views
      queryClient.invalidateQueries({ queryKey: ['directory'] });
    },
    onDropError: (error) => {
      console.error('[DragDropProvider] onDropError:', error);
    },
    onDragCancel: () => {
      console.log('[DragDropProvider] onDragCancel');
      // Clear selection when drag is cancelled (dropped on blank space)
      clearSelection();
    },
  });

  const contextValue: DragDropContextValue = {
    dragState: dragDrop.dragState,
    dropTarget: dragDrop.dropTarget,
    startDrag: dragDrop.startDrag,
    setDropTarget: dragDrop.setDropTarget,
    executeDrop: dragDrop.executeDrop,
    cancelDrag: dragDrop.cancelDrag,
    setCopyMode: dragDrop.setCopyMode,
    isDragging: dragDrop.isDragging,
    isValidTarget: dragDrop.isValidTarget,
  };

  // Native HTML5 drag uses setDragImage() instead of a React overlay
  return (
    <DragDropContext.Provider value={contextValue}>
      {children}
    </DragDropContext.Provider>
  );
}

export function useDragDropContext(): DragDropContextValue {
  const context = useContext(DragDropContext);
  if (!context) {
    throw new Error('useDragDropContext must be used within DragDropProvider');
  }
  return context;
}
