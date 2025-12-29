import { create } from 'zustand';
import type { FileEntry } from '../types/file';

interface SelectionState {
  /** Set of selected file paths */
  selectedPaths: Set<string>;
  /** Currently focused path (for keyboard navigation) */
  focusedPath: string | null;
  /** Last selected path (for shift-click range selection) */
  lastSelectedPath: string | null;
  /** Path currently being renamed (inline editing) */
  editingPath: string | null;
  /** Type of item being created ('file' | 'folder' | null) */
  creatingType: 'file' | 'folder' | null;
  /** Directory where new item is being created */
  creatingInPath: string | null;
}

interface SelectionActions {
  /** Select a single file (optionally additive with Cmd/Ctrl) */
  select: (path: string, additive?: boolean) => void;
  /** Select a range of files (Shift+click) */
  selectRange: (targetPath: string, entries: FileEntry[]) => void;
  /** Select multiple files at once (for marquee/drag selection) */
  selectMultiple: (paths: string[], additive?: boolean) => void;
  /** Toggle selection of a single file */
  toggleSelection: (path: string) => void;
  /** Clear all selections */
  clearSelection: () => void;
  /** Select all entries */
  selectAll: (entries: FileEntry[]) => void;
  /** Set focus to a path */
  setFocus: (path: string | null) => void;
  /** Check if a path is selected */
  isSelected: (path: string) => boolean;
  /** Get selected entries from a list */
  getSelectedEntries: (entries: FileEntry[]) => FileEntry[];
  /** Start inline editing (renaming) for a path */
  startEditing: (path: string) => void;
  /** Stop inline editing */
  stopEditing: () => void;
  /** Start creating a new file or folder */
  startCreating: (type: 'file' | 'folder', inPath: string) => void;
  /** Stop creating (cancel or complete) */
  stopCreating: () => void;
}

export const useSelectionStore = create<SelectionState & SelectionActions>((set, get) => ({
  selectedPaths: new Set<string>(),
  focusedPath: null,
  lastSelectedPath: null,
  editingPath: null,
  creatingType: null,
  creatingInPath: null,

  select: (path: string, additive = false) => {
    set((state) => {
      if (additive) {
        const newSelected = new Set(state.selectedPaths);
        if (newSelected.has(path)) {
          newSelected.delete(path);
        } else {
          newSelected.add(path);
        }
        return {
          selectedPaths: newSelected,
          lastSelectedPath: path,
          focusedPath: path,
        };
      } else {
        return {
          selectedPaths: new Set([path]),
          lastSelectedPath: path,
          focusedPath: path,
        };
      }
    });
  },

  selectRange: (targetPath: string, entries: FileEntry[]) => {
    const { lastSelectedPath, selectedPaths } = get();

    if (!lastSelectedPath) {
      set({
        selectedPaths: new Set([targetPath]),
        lastSelectedPath: targetPath,
        focusedPath: targetPath,
      });
      return;
    }

    const paths = entries.map((e) => e.path);
    const startIdx = paths.indexOf(lastSelectedPath);
    const endIdx = paths.indexOf(targetPath);

    if (startIdx === -1 || endIdx === -1) {
      set({
        selectedPaths: new Set([targetPath]),
        lastSelectedPath: targetPath,
        focusedPath: targetPath,
      });
      return;
    }

    const [from, to] = startIdx < endIdx ? [startIdx, endIdx] : [endIdx, startIdx];
    const rangePaths = paths.slice(from, to + 1);

    set({
      selectedPaths: new Set([...selectedPaths, ...rangePaths]),
      focusedPath: targetPath,
    });
  },

  selectMultiple: (paths: string[], additive = false) => {
    if (paths.length === 0) return;

    set((state) => {
      const newSelected = additive
        ? new Set([...state.selectedPaths, ...paths])
        : new Set(paths);

      return {
        selectedPaths: newSelected,
        lastSelectedPath: paths[paths.length - 1],
        focusedPath: paths[paths.length - 1],
      };
    });
  },

  toggleSelection: (path: string) => {
    set((state) => {
      const newSelected = new Set(state.selectedPaths);
      if (newSelected.has(path)) {
        newSelected.delete(path);
      } else {
        newSelected.add(path);
      }
      return {
        selectedPaths: newSelected,
        lastSelectedPath: path,
        focusedPath: path,
      };
    });
  },

  clearSelection: () => {
    set({
      selectedPaths: new Set(),
      lastSelectedPath: null,
    });
  },

  selectAll: (entries: FileEntry[]) => {
    const paths = entries.map((e) => e.path);
    set({
      selectedPaths: new Set(paths),
      lastSelectedPath: paths[paths.length - 1] || null,
    });
  },

  setFocus: (path: string | null) => {
    set({ focusedPath: path });
  },

  isSelected: (path: string) => {
    return get().selectedPaths.has(path);
  },

  getSelectedEntries: (entries: FileEntry[]) => {
    const { selectedPaths } = get();
    return entries.filter((e) => selectedPaths.has(e.path));
  },

  startEditing: (path: string) => {
    set({
      editingPath: path,
      creatingType: null,
      creatingInPath: null,
    });
  },

  stopEditing: () => {
    set({ editingPath: null });
  },

  startCreating: (type: 'file' | 'folder', inPath: string) => {
    set({
      creatingType: type,
      creatingInPath: inPath,
      editingPath: null,
      selectedPaths: new Set(),
    });
  },

  stopCreating: () => {
    set({
      creatingType: null,
      creatingInPath: null,
    });
  },
}));
