import { create } from "zustand";
import { persist } from "zustand/middleware";
import { invoke } from "@tauri-apps/api/core";

// ============================================================================
// Types
// ============================================================================

export interface RenameHistoryItem {
  id: string;
  /** @deprecated Use folderId + originalName instead. Kept for migration only. */
  originalPath?: string;
  originalName: string;
  /** @deprecated Use folderId + newName instead. Kept for migration only. */
  newPath?: string;
  newName: string;
  timestamp: number;
  canUndo: boolean;
  undone: boolean;
  /** Folder ID (preferred) or folder path (legacy) for display */
  folderId: string;
  /** Folder display name (cached for offline display) */
  folderName: string;
}

export interface WatchedFolder {
  id: string;
  path: string;
  name: string; // Display name (e.g., "Downloads", "Desktop")
  enabled: boolean;
  addedAt: number;
}

export interface CustomRenameRule {
  id: string;
  name: string;
  description: string;
  enabled: boolean;
  priority: number; // Lower = higher priority
  // Match conditions
  matchType: "extension" | "pattern" | "folder" | "content";
  matchValue: string; // e.g., ".pdf", "screenshot*", "/Downloads"
  // Transform action
  transformType: "prefix" | "suffix" | "replace" | "template" | "ai-prompt";
  transformValue: string; // e.g., "doc-", "-backup", custom prompt
  // Examples for AI
  examples?: string[];
}

export interface WatcherStatus {
  enabled: boolean;
  watchingPaths: string[];
  processingCount: number;
}

// ============================================================================
// Default Rules
// ============================================================================

export const DEFAULT_RULES: CustomRenameRule[] = [
  {
    id: "rule-screenshots",
    name: "Screenshots",
    description: "Clean up screenshot filenames with date",
    enabled: true,
    priority: 1,
    matchType: "pattern",
    matchValue: "Screenshot*|Screen Shot*|Capture*",
    transformType: "template",
    transformValue: "screenshot-{date}",
  },
  {
    id: "rule-downloads",
    name: "Downloaded Files",
    description: "Remove (1), (2) suffixes and clean names",
    enabled: true,
    priority: 2,
    matchType: "pattern",
    matchValue: "* (1)*|* (2)*|* (3)*|*copy*",
    transformType: "ai-prompt",
    transformValue: "Remove duplicate indicators and clean the filename",
  },
  {
    id: "rule-invoices",
    name: "Invoices & Receipts",
    description: "Format as invoice-{vendor}-{date}.pdf",
    enabled: true,
    priority: 3,
    matchType: "content",
    matchValue: "invoice|receipt|payment|order confirmation",
    transformType: "template",
    transformValue: "invoice-{vendor}-{date}",
  },
];

// ============================================================================
// Store State
// ============================================================================

interface DownloadsWatcherState {
  // History
  history: RenameHistoryItem[];
  maxHistoryItems: number;

  // Watched folders
  watchedFolders: WatchedFolder[];

  // Custom rules
  customRules: CustomRenameRule[];
  rulesEnabled: boolean;

  // Status
  isWatching: boolean;
  processingFiles: Set<string>;
}

interface DownloadsWatcherActions {
  // History actions
  addToHistory: (item: Omit<RenameHistoryItem, "id" | "timestamp" | "canUndo" | "undone">) => void;
  markUndone: (id: string) => void;
  clearHistory: () => void;
  removeFromHistory: (id: string) => void;

  // Folder actions
  addWatchedFolder: (path: string, name?: string) => void;
  removeWatchedFolder: (id: string) => void;
  toggleFolderEnabled: (id: string) => void;
  getEnabledFolders: () => WatchedFolder[];

  // Rule actions
  addRule: (rule: Omit<CustomRenameRule, "id">) => void;
  updateRule: (id: string, updates: Partial<CustomRenameRule>) => void;
  removeRule: (id: string) => void;
  toggleRuleEnabled: (id: string) => void;
  reorderRules: (ruleIds: string[]) => void;
  setRulesEnabled: (enabled: boolean) => void;
  resetToDefaultRules: () => void;

  // Status actions
  setIsWatching: (watching: boolean) => void;
  addProcessingFile: (path: string) => void;
  removeProcessingFile: (path: string) => void;

  // Undo action
  undoRename: (historyId: string) => Promise<boolean>;
}

// ============================================================================
// Store
// ============================================================================

export const useDownloadsWatcherStore = create<DownloadsWatcherState & DownloadsWatcherActions>()(
  persist(
    (set, get) => ({
      // Initial state
      history: [],
      maxHistoryItems: 100,
      watchedFolders: [],
      customRules: DEFAULT_RULES,
      rulesEnabled: true,
      isWatching: false,
      processingFiles: new Set(),

      // History actions
      addToHistory: (item) => {
        const newItem: RenameHistoryItem = {
          ...item,
          id: `rename-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`,
          timestamp: Date.now(),
          canUndo: true,
          undone: false,
        };

        set((state) => {
          const newHistory = [newItem, ...state.history].slice(0, state.maxHistoryItems);
          return { history: newHistory };
        });
      },

      markUndone: (id) => {
        set((state) => ({
          history: state.history.map((item) =>
            item.id === id ? { ...item, undone: true, canUndo: false } : item
          ),
        }));
      },

      clearHistory: () => set({ history: [] }),

      removeFromHistory: (id) => {
        set((state) => ({
          history: state.history.filter((item) => item.id !== id),
        }));
      },

      // Folder actions
      addWatchedFolder: (path, name) => {
        const folderName = name || path.split("/").pop() || path;
        const newFolder: WatchedFolder = {
          id: `folder-${Date.now()}`,
          path,
          name: folderName,
          enabled: true,
          addedAt: Date.now(),
        };

        set((state) => {
          // Don't add duplicates
          if (state.watchedFolders.some((f) => f.path === path)) {
            return state;
          }
          return { watchedFolders: [...state.watchedFolders, newFolder] };
        });
      },

      removeWatchedFolder: (id) => {
        set((state) => ({
          watchedFolders: state.watchedFolders.filter((f) => f.id !== id),
        }));
      },

      toggleFolderEnabled: (id) => {
        set((state) => ({
          watchedFolders: state.watchedFolders.map((f) =>
            f.id === id ? { ...f, enabled: !f.enabled } : f
          ),
        }));
      },

      getEnabledFolders: () => {
        return get().watchedFolders.filter((f) => f.enabled);
      },

      // Rule actions
      addRule: (rule) => {
        const newRule: CustomRenameRule = {
          ...rule,
          id: `rule-${Date.now()}`,
        };

        set((state) => ({
          customRules: [...state.customRules, newRule],
        }));
      },

      updateRule: (id, updates) => {
        set((state) => ({
          customRules: state.customRules.map((r) =>
            r.id === id ? { ...r, ...updates } : r
          ),
        }));
      },

      removeRule: (id) => {
        set((state) => ({
          customRules: state.customRules.filter((r) => r.id !== id),
        }));
      },

      toggleRuleEnabled: (id) => {
        set((state) => ({
          customRules: state.customRules.map((r) =>
            r.id === id ? { ...r, enabled: !r.enabled } : r
          ),
        }));
      },

      reorderRules: (ruleIds) => {
        set((state) => {
          const ruleMap = new Map(state.customRules.map((r) => [r.id, r]));
          const reordered = ruleIds
            .map((id, index) => {
              const rule = ruleMap.get(id);
              return rule ? { ...rule, priority: index + 1 } : null;
            })
            .filter(Boolean) as CustomRenameRule[];
          return { customRules: reordered };
        });
      },

      setRulesEnabled: (enabled) => set({ rulesEnabled: enabled }),

      resetToDefaultRules: () => set({ customRules: DEFAULT_RULES }),

      // Status actions
      setIsWatching: (watching) => set({ isWatching: watching }),

      addProcessingFile: (path) => {
        set((state) => {
          const newSet = new Set(state.processingFiles);
          newSet.add(path);
          return { processingFiles: newSet };
        });
      },

      removeProcessingFile: (path) => {
        set((state) => {
          const newSet = new Set(state.processingFiles);
          newSet.delete(path);
          return { processingFiles: newSet };
        });
      },

      // Undo action
      undoRename: async (historyId) => {
        const item = get().history.find((h) => h.id === historyId);
        if (!item || !item.canUndo || item.undone) {
          return false;
        }

        // Find the folder to reconstruct full paths
        const folder = get().watchedFolders.find((f) => f.id === item.folderId);
        if (!folder) {
          console.error("Cannot undo: watched folder not found");
          return false;
        }

        // Reconstruct full paths from folder path + filenames
        const currentPath = `${folder.path}/${item.newName}`;
        const originalPath = `${folder.path}/${item.originalName}`;

        try {
          await invoke("undo_rename", {
            currentPath,
            originalPath,
          });

          get().markUndone(historyId);
          return true;
        } catch (error) {
          console.error("Failed to undo rename:", error);
          return false;
        }
      },
    }),
    {
      name: "sentinel-downloads-watcher",
      partialize: (state) => ({
        // Strip deprecated full path fields from history for privacy
        history: state.history.map((item) => ({
          id: item.id,
          originalName: item.originalName,
          newName: item.newName,
          timestamp: item.timestamp,
          canUndo: item.canUndo,
          undone: item.undone,
          folderId: item.folderId,
          folderName: item.folderName,
          // Don't persist: originalPath, newPath (privacy)
        })),
        maxHistoryItems: state.maxHistoryItems,
        // Don't persist full folder paths either - store only display info
        watchedFolders: state.watchedFolders.map((f) => ({
          id: f.id,
          path: f.path, // Still need path for reconstructing undo operations
          name: f.name,
          enabled: f.enabled,
          addedAt: f.addedAt,
        })),
        customRules: state.customRules,
        rulesEnabled: state.rulesEnabled,
      }),
    }
  )
);

// ============================================================================
// Selectors
// ============================================================================

export const selectRecentRenames = (limit = 10) => {
  const { history } = useDownloadsWatcherStore.getState();
  return history.slice(0, limit);
};

export const selectUndoableRenames = () => {
  const { history } = useDownloadsWatcherStore.getState();
  return history.filter((h) => h.canUndo && !h.undone);
};

/**
 * Convert a glob pattern to a safe regex pattern
 * Escapes regex special chars first, then converts glob wildcards
 */
function globToSafeRegex(pattern: string): RegExp | null {
  try {
    // Limit pattern length to prevent DoS
    if (pattern.length > 200) {
      console.warn("Pattern too long, skipping:", pattern.slice(0, 50));
      return null;
    }

    // Escape regex special characters FIRST (except * and ?)
    const escaped = pattern
      .replace(/[.+^${}()|[\]\\]/g, "\\$&")
      // Then convert glob wildcards to regex
      .replace(/\*/g, "[^/]*")  // * matches any chars except path separator
      .replace(/\?/g, ".");     // ? matches single char

    return new RegExp("^" + escaped + "$", "i");
  } catch (e) {
    console.warn("Invalid pattern:", pattern);
    return null;
  }
}

/**
 * Match a filename against rules - returns the first matching rule
 * SECURITY: Patterns are sanitized to prevent ReDoS attacks
 */
export const selectRuleByMatch = (filename: string, content?: string) => {
  const { customRules, rulesEnabled } = useDownloadsWatcherStore.getState();

  if (!rulesEnabled) return null;

  // Limit filename length for regex matching
  const safeFilename = filename.slice(0, 500);

  const enabledRules = customRules
    .filter((r) => r.enabled)
    .sort((a, b) => a.priority - b.priority);

  for (const rule of enabledRules) {
    switch (rule.matchType) {
      case "extension": {
        const ext = safeFilename.split(".").pop()?.toLowerCase();
        if (ext && rule.matchValue.toLowerCase().includes(ext)) {
          return rule;
        }
        break;
      }
      case "pattern": {
        const patterns = rule.matchValue.split("|").map((p) => p.trim()).slice(0, 20); // Limit patterns
        for (const pattern of patterns) {
          const regex = globToSafeRegex(pattern);
          if (regex && regex.test(safeFilename)) {
            return rule;
          }
        }
        break;
      }
      case "folder": {
        // Match against the folder portion of the path if available
        // For now, just check if the matchValue appears in the filename context
        if (rule.matchValue && safeFilename.toLowerCase().includes(rule.matchValue.toLowerCase())) {
          return rule;
        }
        break;
      }
      case "content": {
        if (content) {
          const keywords = rule.matchValue.toLowerCase().split("|").map((k) => k.trim()).slice(0, 20);
          // Limit content search to first 10KB to prevent DoS
          const safeContent = content.slice(0, 10000).toLowerCase();
          if (keywords.some((k) => k.length > 0 && safeContent.includes(k))) {
            return rule;
          }
        }
        break;
      }
    }
  }

  return null;
};
