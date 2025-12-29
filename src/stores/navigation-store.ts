import { create } from 'zustand';
import type { ViewMode, SortField, SortDirection } from '../types/file';

interface NavigationState {
  /** Current directory path */
  currentPath: string;
  /** Navigation history */
  history: string[];
  /** Current position in history */
  historyIndex: number;
  /** View mode */
  viewMode: ViewMode;
  /** Sort field */
  sortField: SortField;
  /** Sort direction */
  sortDirection: SortDirection;
  /** Show hidden files */
  showHidden: boolean;
  /** Quick Look active */
  quickLookActive: boolean;
  /** Quick Look target path */
  quickLookPath: string | null;
}

interface NavigationActions {
  /** Navigate to a path */
  navigateTo: (path: string) => void;
  /** Go back in history */
  goBack: () => void;
  /** Go forward in history */
  goForward: () => void;
  /** Go to parent directory */
  goUp: () => void;
  /** Set view mode */
  setViewMode: (mode: ViewMode) => void;
  /** Set sort field */
  setSortField: (field: SortField) => void;
  /** Toggle sort direction */
  toggleSortDirection: () => void;
  /** Toggle hidden files */
  toggleShowHidden: () => void;
  /** Toggle Quick Look */
  toggleQuickLook: (path?: string) => void;
  /** Close Quick Look */
  closeQuickLook: () => void;
  /** Set Quick Look path */
  setQuickLookPath: (path: string | null) => void;
}

// Helper to get initial view mode from localStorage
const getInitialViewMode = (): ViewMode => {
  if (typeof window !== 'undefined') {
    const saved = localStorage.getItem('sentinel-view-mode');
    if (saved === 'list' || saved === 'grid' || saved === 'columns') {
      return saved;
    }
  }
  return 'list';
};

export const useNavigationStore = create<NavigationState & NavigationActions>((set, get) => ({
  currentPath: '',
  history: [],
  historyIndex: -1,
  viewMode: getInitialViewMode(),
  sortField: 'name',
  sortDirection: 'asc',
  showHidden: false,
  quickLookActive: false,
  quickLookPath: null,

  navigateTo: (path: string) => {
    const { history, historyIndex } = get();

    // Don't navigate to the same path
    if (path === get().currentPath) return;

    // Remove forward history when navigating to new path
    const newHistory = history.slice(0, historyIndex + 1);
    newHistory.push(path);

    set({
      currentPath: path,
      history: newHistory,
      historyIndex: newHistory.length - 1,
    });
  },

  goBack: () => {
    const { history, historyIndex } = get();
    if (historyIndex > 0) {
      const newIndex = historyIndex - 1;
      set({
        currentPath: history[newIndex],
        historyIndex: newIndex,
      });
    }
  },

  goForward: () => {
    const { history, historyIndex } = get();
    if (historyIndex < history.length - 1) {
      const newIndex = historyIndex + 1;
      set({
        currentPath: history[newIndex],
        historyIndex: newIndex,
      });
    }
  },

  goUp: () => {
    const { currentPath, navigateTo } = get();
    const parentPath = currentPath.split('/').slice(0, -1).join('/') || '/';
    navigateTo(parentPath);
  },

  setViewMode: (mode: ViewMode) => {
    localStorage.setItem('sentinel-view-mode', mode);
    set({ viewMode: mode });
  },

  setSortField: (field: SortField) => {
    set({ sortField: field });
  },

  toggleSortDirection: () => {
    set((state) => ({
      sortDirection: state.sortDirection === 'asc' ? 'desc' : 'asc',
    }));
  },

  toggleShowHidden: () => {
    set((state) => ({ showHidden: !state.showHidden }));
  },

  toggleQuickLook: (path?: string) => {
    const { quickLookActive, quickLookPath } = get();
    if (quickLookActive) {
      set({ quickLookActive: false, quickLookPath: null });
    } else {
      set({
        quickLookActive: true,
        quickLookPath: path || quickLookPath,
      });
    }
  },

  closeQuickLook: () => {
    set({ quickLookActive: false, quickLookPath: null });
  },

  setQuickLookPath: (path: string | null) => {
    set({ quickLookPath: path });
  },
}));
