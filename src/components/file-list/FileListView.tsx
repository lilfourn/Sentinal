import { useRef, useCallback, useEffect, useState } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import { invoke } from '@tauri-apps/api/core';
import { FileRow } from './FileRow';
import { SelectionOverlay } from './SelectionOverlay';
import { ContextMenu, buildFileContextMenuItems, type ContextMenuPosition } from '../ContextMenu/ContextMenu';
import { useNavigationStore } from '../../stores/navigation-store';
import { useSelectionStore } from '../../stores/selection-store';
import { useOrganizeStore } from '../../stores/organize-store';
import { showSuccess, showError } from '../../stores/toast-store';
import type { FileEntry } from '../../types/file';
import type { SelectionRect } from '../../hooks/useMarqueeSelection';

// Re-export for type compatibility
export type { FileEntry };

interface FileListViewProps {
  entries: FileEntry[];
}

const ROW_HEIGHT = 28;
const HEADER_HEIGHT = 30;

export function FileListView({ entries }: FileListViewProps) {
  const parentRef = useRef<HTMLDivElement>(null);
  const { navigateTo, setQuickLookPath } = useNavigationStore();
  const {
    selectedPaths,
    focusedPath,
    select,
    selectRange,
    clearSelection,
    selectMultiple,
  } = useSelectionStore();
  const { startOrganize } = useOrganizeStore();

  // Marquee selection state
  const [isDragging, setIsDragging] = useState(false);
  const [dragStart, setDragStart] = useState({ x: 0, y: 0 });
  const [dragCurrent, setDragCurrent] = useState({ x: 0, y: 0 });
  const [dragModifiers, setDragModifiers] = useState({ meta: false, shift: false });
  const justFinishedDraggingRef = useRef(false);

  // Context menu state
  const [contextMenu, setContextMenu] = useState<{
    position: ContextMenuPosition;
    entry: FileEntry;
  } | null>(null);

  const virtualizer = useVirtualizer({
    count: entries.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => ROW_HEIGHT,
    overscan: 10,
  });

  // Calculate selection rectangle
  const getSelectionRect = useCallback((): SelectionRect | null => {
    if (!isDragging || !parentRef.current) return null;

    const container = parentRef.current;
    const containerRect = container.getBoundingClientRect();
    const scrollTop = container.scrollTop;

    const startX = dragStart.x - containerRect.left;
    const startY = dragStart.y - containerRect.top + scrollTop - HEADER_HEIGHT;
    const currentX = dragCurrent.x - containerRect.left;
    const currentY = dragCurrent.y - containerRect.top + scrollTop - HEADER_HEIGHT;

    return {
      x: Math.min(startX, currentX),
      y: Math.min(startY, currentY),
      width: Math.abs(currentX - startX),
      height: Math.abs(currentY - startY),
    };
  }, [isDragging, dragStart, dragCurrent]);

  // Get paths of items that intersect with selection rectangle (mathematical approach for virtualized list)
  const getIntersectingPaths = useCallback((): string[] => {
    const rect = getSelectionRect();
    if (!rect) return [];

    const paths: string[] = [];
    const minRowIndex = Math.floor(rect.y / ROW_HEIGHT);
    const maxRowIndex = Math.ceil((rect.y + rect.height) / ROW_HEIGHT);

    for (let i = Math.max(0, minRowIndex); i < Math.min(entries.length, maxRowIndex); i++) {
      paths.push(entries[i].path);
    }

    return paths;
  }, [getSelectionRect, entries]);

  const handleClick = useCallback(
    (entry: FileEntry, e: React.MouseEvent) => {
      e.stopPropagation();

      if (e.shiftKey) {
        selectRange(entry.path, entries);
      } else if (e.metaKey || e.ctrlKey) {
        select(entry.path, true);
      } else {
        select(entry.path, false);
      }

      // Update quick look path if active
      setQuickLookPath(entry.path);
    },
    [entries, select, selectRange, setQuickLookPath]
  );

  const handleDoubleClick = useCallback(
    (entry: FileEntry) => {
      if (entry.isDirectory) {
        navigateTo(entry.path);
      }
    },
    [navigateTo]
  );

  const handleContainerClick = useCallback((e: React.MouseEvent) => {
    // Don't clear selection if we just finished a marquee drag
    if (justFinishedDraggingRef.current) return;

    if (e.target === e.currentTarget) {
      clearSelection();
      setContextMenu(null);
    }
  }, [clearSelection]);

  const handleContainerMouseDown = useCallback((e: React.MouseEvent) => {
    // Only start drag on left button, on container background
    const target = e.target as HTMLElement;
    const isListArea = target === parentRef.current ||
      target.classList.contains('virtual-list-inner');

    if (isListArea && e.button === 0) {
      setDragModifiers({
        meta: e.metaKey || e.ctrlKey,
        shift: e.shiftKey,
      });
      setDragStart({ x: e.clientX, y: e.clientY });
      setDragCurrent({ x: e.clientX, y: e.clientY });
      setIsDragging(true);

      if (!e.metaKey && !e.ctrlKey && !e.shiftKey) {
        clearSelection();
      }
    }
  }, [clearSelection]);

  // Global mouse event handlers for drag
  useEffect(() => {
    if (!isDragging) return;

    const handleMouseMove = (e: MouseEvent) => {
      setDragCurrent({ x: e.clientX, y: e.clientY });
    };

    const handleMouseUp = () => {
      const paths = getIntersectingPaths();
      if (paths.length > 0) {
        selectMultiple(paths, dragModifiers.meta || dragModifiers.shift);
      }

      // Mark that we just finished dragging - prevents click handler from clearing selection
      justFinishedDraggingRef.current = true;
      setTimeout(() => {
        justFinishedDraggingRef.current = false;
      }, 0);

      setIsDragging(false);
    };

    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);

    return () => {
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
    };
  }, [isDragging, getIntersectingPaths, selectMultiple, dragModifiers]);

  const handleContextMenu = useCallback(
    (entry: FileEntry, e: React.MouseEvent) => {
      e.preventDefault();
      e.stopPropagation();

      // Select the item if not already selected
      if (!selectedPaths.has(entry.path)) {
        select(entry.path, false);
      }

      setContextMenu({
        position: { x: e.clientX, y: e.clientY },
        entry,
      });
    },
    [selectedPaths, select]
  );

  const handleMoveToTrash = useCallback(async (path: string) => {
    try {
      await invoke('delete_to_trash', { path });
      showSuccess('Moved to Trash', path.split('/').pop() || path);
    } catch (error) {
      showError('Failed to move to Trash', String(error));
    }
  }, []);

  // Keyboard navigation
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (!entries.length) return;

      const currentIndex = entries.findIndex((e) => e.path === focusedPath);

      switch (e.key) {
        case 'ArrowDown': {
          e.preventDefault();
          const nextIndex = Math.min(currentIndex + 1, entries.length - 1);
          const nextEntry = entries[nextIndex];
          if (nextEntry) {
            if (e.shiftKey) {
              selectRange(nextEntry.path, entries);
            } else {
              select(nextEntry.path, false);
            }
            virtualizer.scrollToIndex(nextIndex, { align: 'auto' });
          }
          break;
        }
        case 'ArrowUp': {
          e.preventDefault();
          const prevIndex = Math.max(currentIndex - 1, 0);
          const prevEntry = entries[prevIndex];
          if (prevEntry) {
            if (e.shiftKey) {
              selectRange(prevEntry.path, entries);
            } else {
              select(prevEntry.path, false);
            }
            virtualizer.scrollToIndex(prevIndex, { align: 'auto' });
          }
          break;
        }
        case 'Enter': {
          e.preventDefault();
          const currentEntry = entries[currentIndex];
          if (currentEntry?.isDirectory) {
            navigateTo(currentEntry.path);
          }
          break;
        }
        case 'a':
          if (e.metaKey || e.ctrlKey) {
            e.preventDefault();
            const allPaths = entries.map((e) => e.path);
            selectMultiple(allPaths, false);
          }
          break;
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [entries, focusedPath, select, selectRange, selectMultiple, navigateTo, virtualizer]);

  const selectionRect = getSelectionRect();

  return (
    <div
      ref={parentRef}
      onClick={handleContainerClick}
      onMouseDown={handleContainerMouseDown}
      className="relative h-full overflow-auto focus:outline-none select-none"
      tabIndex={0}
    >
      {/* Header */}
      <div className="sticky top-0 z-10 flex items-center gap-3 px-4 py-1.5 glass-file-header border-b border-gray-200/20 text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
        <span className="w-[18px]" /> {/* Icon space */}
        <span className="flex-1">Name</span>
        <span className="w-36">Date Modified</span>
        <span className="w-20 text-right">Size</span>
      </div>

      {/* Virtualized list */}
      <div
        className="virtual-list-inner"
        style={{
          height: `${virtualizer.getTotalSize()}px`,
          width: '100%',
          position: 'relative',
        }}
      >
        {virtualizer.getVirtualItems().map((virtualRow) => {
          const entry = entries[virtualRow.index];
          return (
            <FileRow
              key={entry.path}
              entry={entry}
              isSelected={selectedPaths.has(entry.path)}
              isFocused={focusedPath === entry.path}
              style={{
                position: 'absolute',
                top: 0,
                left: 0,
                width: '100%',
                height: `${virtualRow.size}px`,
                transform: `translateY(${virtualRow.start}px)`,
              }}
              onClick={(e) => handleClick(entry, e)}
              onDoubleClick={() => handleDoubleClick(entry)}
              onContextMenu={(e) => handleContextMenu(entry, e)}
            />
          );
        })}
      </div>

      {/* Empty state */}
      {entries.length === 0 && (
        <div className="flex items-center justify-center h-32 text-gray-400">
          This folder is empty
        </div>
      )}

      {/* Selection overlay (marquee rectangle) */}
      {isDragging && selectionRect && (
        <SelectionOverlay
          rect={{
            ...selectionRect,
            y: selectionRect.y + HEADER_HEIGHT, // Adjust for header
          }}
        />
      )}

      {/* Context Menu */}
      {contextMenu && (
        <ContextMenu
          position={contextMenu.position}
          items={buildFileContextMenuItems(
            {
              name: contextMenu.entry.name,
              path: contextMenu.entry.path,
              isDirectory: contextMenu.entry.isDirectory,
            },
            {
              onOpen: () => {
                if (contextMenu.entry.isDirectory) {
                  navigateTo(contextMenu.entry.path);
                }
              },
              onOrganizeWithAI: () => {
                startOrganize(contextMenu.entry.path);
              },
              onMoveToTrash: () => {
                handleMoveToTrash(contextMenu.entry.path);
              },
            }
          )}
          onClose={() => setContextMenu(null)}
        />
      )}
    </div>
  );
}
