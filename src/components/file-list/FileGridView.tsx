import { useCallback, useState, useRef, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useQueryClient } from '@tanstack/react-query';
import {
  File,
  FileText,
  FileImage,
  FileVideo,
  FileAudio,
  FileCode,
  FileArchive,
  FileJson,
  type LucideIcon,
} from 'lucide-react';
import { ContextMenu, buildFileContextMenuItems, buildBackgroundContextMenuItems, type ContextMenuPosition } from '../ContextMenu/ContextMenu';
import { FolderIcon } from '../icons/FolderIcon';
import { SelectionOverlay } from './SelectionOverlay';
import { InlineNameEditor } from './InlineNameEditor';
import { useNavigationStore } from '../../stores/navigation-store';
import { useSelectionStore } from '../../stores/selection-store';
import { useOrganizeStore } from '../../stores/organize-store';
import { showSuccess, showError } from '../../stores/toast-store';
import { useThumbnail } from '../../hooks/useThumbnail';
import { useMarqueeSelection } from '../../hooks/useMarqueeSelection';
import { cn, getFileType, isThumbnailSupported, openFile } from '../../lib/utils';
import type { FileEntry } from '../../types/file';

interface FileGridViewProps {
  entries: FileEntry[];
}

const fileTypeIcons: Record<string, LucideIcon> = {
  image: FileImage,
  video: FileVideo,
  audio: FileAudio,
  code: FileCode,
  config: FileJson,
  text: FileText,
  document: FileText,
  archive: FileArchive,
  unknown: File,
};

function getFileIcon(entry: FileEntry): LucideIcon | null {
  if (entry.isDirectory) return null;
  const fileType = getFileType(entry.extension, entry.mimeType);
  return fileTypeIcons[fileType] || File;
}

// Separate component for grid items to enable hook usage
interface FileGridItemProps {
  entry: FileEntry;
  isSelected: boolean;
  isEditing?: boolean;
  onClick: (e: React.MouseEvent) => void;
  onDoubleClick: () => void;
  onContextMenu: (e: React.MouseEvent) => void;
  onRenameConfirm?: (newName: string) => void;
  onRenameCancel?: () => void;
}

function FileGridItem({
  entry,
  isSelected,
  isEditing = false,
  onClick,
  onDoubleClick,
  onContextMenu,
  onRenameConfirm,
  onRenameCancel,
}: FileGridItemProps) {
  const Icon = getFileIcon(entry);
  const supportsThumbnail = !entry.isDirectory && isThumbnailSupported(entry.extension);
  const { thumbnail, loading } = useThumbnail(
    supportsThumbnail ? entry.path : null,
    entry.extension,
    96
  );

  return (
    <div
      data-path={entry.path}
      onClick={isEditing ? undefined : onClick}
      onDoubleClick={isEditing ? undefined : onDoubleClick}
      onContextMenu={isEditing ? undefined : onContextMenu}
      className={cn(
        'flex flex-col items-center p-2 rounded-lg cursor-default select-none',
        'transition-colors duration-75',
        isSelected && 'bg-orange-500/20',
        !isSelected && !isEditing && 'hover:bg-gray-500/10'
      )}
    >
      {/* Icon/Thumbnail area */}
      <div className="w-12 h-12 mb-1 flex items-center justify-center flex-shrink-0">
        {entry.isDirectory ? (
          <FolderIcon size={48} />
        ) : thumbnail ? (
          <img
            src={`data:image/png;base64,${thumbnail}`}
            alt={entry.name}
            className="max-w-full max-h-full object-contain rounded"
          />
        ) : loading && supportsThumbnail ? (
          <div className="w-10 h-10 bg-gray-200 dark:bg-gray-700 rounded animate-pulse" />
        ) : (
          Icon && <Icon size={48} className="text-gray-400 dark:text-gray-500" />
        )}
      </div>

      {/* Filename - either editable or static */}
      {isEditing && onRenameConfirm && onRenameCancel ? (
        <InlineNameEditor
          initialValue={entry.name}
          onConfirm={onRenameConfirm}
          onCancel={onRenameCancel}
          selectNameOnly={!entry.isDirectory}
          className="w-full text-xs text-center"
        />
      ) : (
        <span
          className={cn(
            'text-xs text-center line-clamp-2 break-all w-full',
            'text-gray-800 dark:text-gray-200',
            entry.isHidden && 'text-gray-400 dark:text-gray-500'
          )}
          title={entry.name}
        >
          {entry.name}
        </span>
      )}
    </div>
  );
}

// New item component for grid view when creating files/folders
interface NewGridItemProps {
  type: 'file' | 'folder';
  onConfirm: (name: string) => void;
  onCancel: () => void;
}

function NewGridItem({ type, onConfirm, onCancel }: NewGridItemProps) {
  const defaultName = type === 'folder' ? 'untitled folder' : 'untitled';

  return (
    <div
      className={cn(
        'flex flex-col items-center p-2 rounded-lg cursor-default select-none',
        'bg-orange-500/20'
      )}
    >
      {/* Icon */}
      <div className="w-12 h-12 mb-1 flex items-center justify-center flex-shrink-0">
        {type === 'folder' ? (
          <FolderIcon size={48} />
        ) : (
          <File size={48} className="text-gray-400 dark:text-gray-500" />
        )}
      </div>

      {/* Name input */}
      <InlineNameEditor
        initialValue={defaultName}
        onConfirm={onConfirm}
        onCancel={onCancel}
        selectNameOnly={false}
        className="w-full text-xs text-center"
      />
    </div>
  );
}

export function FileGridView({ entries }: FileGridViewProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const gridRef = useRef<HTMLDivElement>(null);
  const queryClient = useQueryClient();
  const { navigateTo, setQuickLookPath, currentPath, showHidden } = useNavigationStore();
  const {
    selectedPaths,
    focusedPath,
    select,
    selectRange,
    selectMultiple,
    clearSelection,
    editingPath,
    creatingType,
    creatingInPath,
    startEditing,
    stopEditing,
    startCreating,
    stopCreating,
  } = useSelectionStore();
  const { startOrganize } = useOrganizeStore();

  // Check if we should show the new item in this directory
  const isCreatingHere = creatingType !== null && creatingInPath === currentPath;

  // Refresh directory after changes
  const refreshDirectory = useCallback(() => {
    queryClient.invalidateQueries({ queryKey: ['directory', currentPath, showHidden] });
  }, [queryClient, currentPath, showHidden]);

  // Handle rename confirmation
  const handleRenameConfirm = useCallback(
    async (oldPath: string, newName: string) => {
      const parentDir = oldPath.substring(0, oldPath.lastIndexOf('/'));
      const newPath = `${parentDir}/${newName}`;

      try {
        await invoke('rename_file', { oldPath, newPath });
        showSuccess('Renamed', `${oldPath.split('/').pop()} → ${newName}`);
        stopEditing();
        refreshDirectory();
        select(newPath, false);
      } catch (error) {
        showError('Rename failed', String(error));
      }
    },
    [stopEditing, refreshDirectory, select]
  );

  // Handle creating a new file or folder
  const handleCreateConfirm = useCallback(
    async (name: string) => {
      if (!creatingType || !creatingInPath) return;

      const newPath = `${creatingInPath}/${name}`;

      try {
        if (creatingType === 'folder') {
          await invoke('create_directory', { path: newPath });
          showSuccess('Created folder', name);
        } else {
          await invoke('create_file', { path: newPath });
          showSuccess('Created file', name);
        }
        stopCreating();
        refreshDirectory();
        setTimeout(() => select(newPath, false), 100);
      } catch (error) {
        showError(`Failed to create ${creatingType}`, String(error));
      }
    },
    [creatingType, creatingInPath, stopCreating, refreshDirectory, select]
  );

  const handleCreateCancel = useCallback(() => {
    stopCreating();
  }, [stopCreating]);

  const handleRenameCancel = useCallback(() => {
    stopEditing();
  }, [stopEditing]);

  // Marquee selection
  const {
    isDragging,
    justFinishedDragging,
    selectionRect,
    startDrag,
    updateItemPositions,
  } = useMarqueeSelection(containerRef);

  // Update item positions when entries change or when starting drag
  useEffect(() => {
    if (!gridRef.current) return;

    const items = Array.from(gridRef.current.querySelectorAll('[data-path]')).map((el) => ({
      path: el.getAttribute('data-path')!,
      element: el as HTMLElement,
    }));

    updateItemPositions(items);
  }, [entries, updateItemPositions]);

  // Context menu state (for file/folder items)
  const [contextMenu, setContextMenu] = useState<{
    position: ContextMenuPosition;
    entry: FileEntry;
  } | null>(null);

  // Background context menu state (for empty space)
  const [backgroundContextMenu, setBackgroundContextMenu] = useState<ContextMenuPosition | null>(null);

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

      setQuickLookPath(entry.path);
    },
    [entries, select, selectRange, setQuickLookPath]
  );

  const handleDoubleClick = useCallback(
    async (entry: FileEntry) => {
      if (entry.isDirectory) {
        navigateTo(entry.path);
      } else {
        await openFile(entry.path);
      }
    },
    [navigateTo]
  );

  const handleContainerClick = useCallback((e: React.MouseEvent) => {
    // Don't clear selection if we just finished a marquee drag
    if (justFinishedDragging.current) return;

    // Only clear if clicking directly on container (not bubbled from child)
    if (e.target === e.currentTarget || e.target === gridRef.current) {
      clearSelection();
      setContextMenu(null);
      setBackgroundContextMenu(null);
    }
  }, [clearSelection, justFinishedDragging]);

  // Handle right-click on empty space (background)
  // File items call e.stopPropagation(), so only background clicks reach here
  const handleBackgroundContextMenu = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setContextMenu(null); // Close file context menu if open
    setBackgroundContextMenu({ x: e.clientX, y: e.clientY });
  }, []);

  const handleContainerMouseDown = useCallback((e: React.MouseEvent) => {
    // Start marquee selection if clicking on empty space (container or grid, not items)
    const target = e.target as HTMLElement;
    const isContainer = target === containerRef.current || target === gridRef.current;

    if (isContainer && e.button === 0) {
      // Update positions before starting drag
      if (gridRef.current) {
        const items = Array.from(gridRef.current.querySelectorAll('[data-path]')).map((el) => ({
          path: el.getAttribute('data-path')!,
          element: el as HTMLElement,
        }));
        updateItemPositions(items);
      }
      startDrag(e);
    }
  }, [startDrag, updateItemPositions]);

  const handleContextMenu = useCallback(
    (entry: FileEntry, e: React.MouseEvent) => {
      e.preventDefault();
      e.stopPropagation();

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
      refreshDirectory();
    } catch (error) {
      showError('Failed to move to Trash', String(error));
    }
  }, [refreshDirectory]);

  // Keyboard navigation and shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Don't handle keys if we're editing
      if (editingPath || isCreatingHere) {
        if (e.key === 'Escape') {
          e.preventDefault();
          stopEditing();
          stopCreating();
        }
        return;
      }

      const currentIndex = entries.findIndex((entry) => entry.path === focusedPath);

      // New Folder: Cmd+Shift+N
      if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key === 'N') {
        e.preventDefault();
        useSelectionStore.getState().startCreating('folder', currentPath);
        return;
      }

      // New File: Cmd+N (without shift)
      if ((e.metaKey || e.ctrlKey) && !e.shiftKey && e.key === 'n') {
        e.preventDefault();
        useSelectionStore.getState().startCreating('file', currentPath);
        return;
      }

      if (!entries.length) return;

      switch (e.key) {
        case 'Enter': {
          e.preventDefault();
          const currentEntry = entries[currentIndex];
          if (!currentEntry) break;

          if (currentEntry.isDirectory && selectedPaths.size === 1) {
            navigateTo(currentEntry.path);
          } else if (selectedPaths.size === 1) {
            startEditing(currentEntry.path);
          }
          break;
        }
        case 'F2': {
          e.preventDefault();
          if (selectedPaths.size === 1 && focusedPath) {
            startEditing(focusedPath);
          }
          break;
        }
        case 'Backspace':
        case 'Delete': {
          if (e.metaKey || e.ctrlKey) {
            e.preventDefault();
            const selectedEntries = entries.filter((entry) => selectedPaths.has(entry.path));
            selectedEntries.forEach((entry) => handleMoveToTrash(entry.path));
          }
          break;
        }
        case 'a':
          if (e.metaKey || e.ctrlKey) {
            e.preventDefault();
            const allPaths = entries.map((entry) => entry.path);
            selectMultiple(allPaths, false);
          }
          break;
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [entries, focusedPath, navigateTo, editingPath, isCreatingHere, selectedPaths, startEditing, stopEditing, stopCreating, currentPath, handleMoveToTrash, selectMultiple]);

  return (
    <div
      ref={containerRef}
      onClick={handleContainerClick}
      onMouseDown={handleContainerMouseDown}
      onContextMenu={handleBackgroundContextMenu}
      className="relative h-full overflow-auto p-4 focus:outline-none select-none"
      tabIndex={0}
    >
      {/* Grid of items */}
      <div
        ref={gridRef}
        className="grid grid-cols-4 sm:grid-cols-5 md:grid-cols-6 lg:grid-cols-8 xl:grid-cols-10 gap-2"
      >
        {/* New item at the beginning when creating */}
        {isCreatingHere && (
          <NewGridItem
            type={creatingType!}
            onConfirm={handleCreateConfirm}
            onCancel={handleCreateCancel}
          />
        )}

        {entries.map((entry) => (
          <FileGridItem
            key={entry.path}
            entry={entry}
            isSelected={selectedPaths.has(entry.path)}
            isEditing={editingPath === entry.path}
            onClick={(e) => handleClick(entry, e)}
            onDoubleClick={() => handleDoubleClick(entry)}
            onContextMenu={(e) => handleContextMenu(entry, e)}
            onRenameConfirm={(newName) => handleRenameConfirm(entry.path, newName)}
            onRenameCancel={handleRenameCancel}
          />
        ))}
      </div>

      {/* Empty state */}
      {entries.length === 0 && (
        <div className="flex items-center justify-center h-32 text-gray-400">
          This folder is empty
        </div>
      )}

      {/* Selection overlay (marquee rectangle) */}
      {isDragging && <SelectionOverlay rect={selectionRect} />}

      {/* Context Menu (for files/folders) */}
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
              onOpen: async () => {
                if (contextMenu.entry.isDirectory) {
                  navigateTo(contextMenu.entry.path);
                } else {
                  await openFile(contextMenu.entry.path);
                }
              },
              onOrganizeWithAI: () => {
                startOrganize(contextMenu.entry.path);
              },
              onRename: () => {
                startEditing(contextMenu.entry.path);
              },
              onMoveToTrash: () => {
                handleMoveToTrash(contextMenu.entry.path);
              },
            }
          )}
          onClose={() => setContextMenu(null)}
        />
      )}

      {/* Background Context Menu (for empty space) */}
      {backgroundContextMenu && (
        <ContextMenu
          position={backgroundContextMenu}
          items={buildBackgroundContextMenuItems({
            onNewFolder: () => {
              startCreating('folder', currentPath);
            },
            onNewFile: () => {
              startCreating('file', currentPath);
            },
          })}
          onClose={() => setBackgroundContextMenu(null)}
        />
      )}
    </div>
  );
}
