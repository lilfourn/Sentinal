import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  FolderOpen,
  Plus,
  X,
  Eye,
  EyeOff,
  FolderPlus,
  Download,
  Image,
  FileText,
  MonitorPlay,
} from "lucide-react";
import { cn } from "../../lib/utils";
import {
  useDownloadsWatcherStore,
  type WatchedFolder,
} from "../../stores/downloads-watcher-store";
import { showSuccess, showError } from "../../stores/toast-store";

// Common folder suggestions
const SUGGESTED_FOLDERS = [
  {
    id: "downloads",
    name: "Downloads",
    icon: Download,
    getPath: async () => invoke<string>("get_downloads_directory"),
  },
  {
    id: "desktop",
    name: "Desktop",
    icon: MonitorPlay,
    getPath: async () => {
      const home = await invoke<string>("get_home_directory");
      return `${home}/Desktop`;
    },
  },
  {
    id: "documents",
    name: "Documents",
    icon: FileText,
    getPath: async () => {
      const home = await invoke<string>("get_home_directory");
      return `${home}/Documents`;
    },
  },
  {
    id: "pictures",
    name: "Pictures",
    icon: Image,
    getPath: async () => {
      const home = await invoke<string>("get_home_directory");
      return `${home}/Pictures`;
    },
  },
];

interface WatchedFoldersManagerProps {
  onFolderAdded?: () => void;
}

export function WatchedFoldersManager({
  onFolderAdded,
}: WatchedFoldersManagerProps) {
  const {
    watchedFolders,
    addWatchedFolder,
    removeWatchedFolder,
    toggleFolderEnabled,
  } = useDownloadsWatcherStore();

  const [isAdding, setIsAdding] = useState(false);

  const handleAddFolder = async (path: string, name?: string) => {
    try {
      // Add to backend watcher
      await invoke("add_watched_folder", { path });

      // Add to store
      addWatchedFolder(path, name);

      showSuccess("Folder added", `Now watching: ${name || path.split("/").pop()}`);
      onFolderAdded?.();
    } catch (error) {
      showError("Failed to add folder", String(error));
    }
  };

  const handleRemoveFolder = async (folder: WatchedFolder) => {
    try {
      // Remove from backend watcher
      await invoke("remove_watched_folder", { path: folder.path });

      // Remove from store
      removeWatchedFolder(folder.id);

      showSuccess("Folder removed", `Stopped watching: ${folder.name}`);
    } catch (error) {
      showError("Failed to remove folder", String(error));
    }
  };

  const handleToggleFolder = async (folder: WatchedFolder) => {
    try {
      if (folder.enabled) {
        // Disable - remove from backend
        await invoke("remove_watched_folder", { path: folder.path });
      } else {
        // Enable - add to backend
        await invoke("add_watched_folder", { path: folder.path });
      }

      toggleFolderEnabled(folder.id);
    } catch (error) {
      showError("Failed to toggle folder", String(error));
    }
  };

  const handleAddSuggested = async (suggestion: (typeof SUGGESTED_FOLDERS)[0]) => {
    try {
      const path = await suggestion.getPath();
      await handleAddFolder(path, suggestion.name);
    } catch (error) {
      showError("Failed to get folder path", String(error));
    }
  };

  const [customPath, setCustomPath] = useState("");

  const handleAddCustomPath = async () => {
    if (!customPath.trim()) return;
    try {
      await handleAddFolder(customPath.trim());
      setCustomPath("");
    } catch (error) {
      showError("Failed to add folder", String(error));
    }
  };

  // Filter out already-added suggestions
  const availableSuggestions = SUGGESTED_FOLDERS.filter(
    (s) => !watchedFolders.some((f) => f.name === s.name)
  );

  return (
    <div className="space-y-3">
      {/* Watched folders list */}
      {watchedFolders.length > 0 && (
        <div className="space-y-1">
          {watchedFolders.map((folder) => (
            <FolderItem
              key={folder.id}
              folder={folder}
              onToggle={() => handleToggleFolder(folder)}
              onRemove={() => handleRemoveFolder(folder)}
            />
          ))}
        </div>
      )}

      {/* Empty state */}
      {watchedFolders.length === 0 && (
        <div className="p-4 text-center text-sm text-gray-500 dark:text-gray-400 border-2 border-dashed border-gray-200 dark:border-gray-700 rounded-lg">
          <FolderOpen size={24} className="mx-auto mb-2 opacity-50" />
          <p>No folders being watched</p>
          <p className="text-xs mt-1">Add folders to auto-rename new files</p>
        </div>
      )}

      {/* Add folder section */}
      {isAdding ? (
        <div className="space-y-2 p-3 bg-gray-50 dark:bg-gray-800/50 rounded-lg">
          <div className="flex items-center justify-between">
            <span className="text-sm font-medium text-gray-700 dark:text-gray-300">
              Add Folder
            </span>
            <button
              onClick={() => setIsAdding(false)}
              className="p-1 text-gray-400 hover:text-gray-600"
            >
              <X size={14} />
            </button>
          </div>

          {/* Quick add suggestions */}
          {availableSuggestions.length > 0 && (
            <div className="flex flex-wrap gap-1.5">
              {availableSuggestions.map((suggestion) => (
                <button
                  key={suggestion.id}
                  onClick={() => handleAddSuggested(suggestion)}
                  className="flex items-center gap-1.5 px-2 py-1 text-xs rounded-lg bg-white dark:bg-gray-700 border border-gray-200 dark:border-gray-600 hover:border-orange-300 dark:hover:border-orange-600 transition-colors"
                >
                  <suggestion.icon size={12} />
                  {suggestion.name}
                </button>
              ))}
            </div>
          )}

          {/* Custom path input */}
          <div className="flex gap-2">
            <input
              type="text"
              value={customPath}
              onChange={(e) => setCustomPath(e.target.value)}
              placeholder="/path/to/folder"
              className="flex-1 px-3 py-2 text-sm rounded-lg border border-gray-200 dark:border-gray-600 bg-white dark:bg-gray-700 placeholder-gray-400"
              onKeyDown={(e) => e.key === "Enter" && handleAddCustomPath()}
            />
            <button
              onClick={handleAddCustomPath}
              disabled={!customPath.trim()}
              className="px-3 py-2 text-sm rounded-lg bg-orange-500 text-white hover:bg-orange-600 disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-1"
            >
              <FolderPlus size={14} />
              Add
            </button>
          </div>
        </div>
      ) : (
        <button
          onClick={() => setIsAdding(true)}
          className="w-full flex items-center justify-center gap-2 py-2 text-sm text-gray-600 dark:text-gray-400 hover:text-orange-500 dark:hover:text-orange-400 transition-colors"
        >
          <Plus size={16} />
          Add watched folder
        </button>
      )}
    </div>
  );
}

interface FolderItemProps {
  folder: WatchedFolder;
  onToggle: () => void;
  onRemove: () => void;
}

function FolderItem({ folder, onToggle, onRemove }: FolderItemProps) {
  return (
    <div
      className={cn(
        "group flex items-center justify-between p-2 rounded-lg border transition-colors",
        folder.enabled
          ? "bg-white dark:bg-[#2a2a2a] border-gray-200 dark:border-gray-700"
          : "bg-gray-50 dark:bg-gray-800/50 border-gray-200 dark:border-gray-700 opacity-60"
      )}
    >
      <div className="flex items-center gap-2 min-w-0">
        <FolderOpen
          size={16}
          className={cn(
            folder.enabled
              ? "text-orange-500"
              : "text-gray-400"
          )}
        />
        <div className="min-w-0">
          <p
            className={cn(
              "text-sm font-medium truncate",
              folder.enabled
                ? "text-gray-900 dark:text-gray-100"
                : "text-gray-500 dark:text-gray-400"
            )}
          >
            {folder.name}
          </p>
          <p className="text-xs text-gray-400 truncate">{folder.path}</p>
        </div>
      </div>

      <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
        <button
          onClick={onToggle}
          className={cn(
            "p-1.5 rounded transition-colors",
            folder.enabled
              ? "text-green-500 hover:bg-green-50 dark:hover:bg-green-900/20"
              : "text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700"
          )}
          title={folder.enabled ? "Pause watching" : "Resume watching"}
        >
          {folder.enabled ? <Eye size={14} /> : <EyeOff size={14} />}
        </button>
        <button
          onClick={onRemove}
          className="p-1.5 rounded text-gray-400 hover:text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors"
          title="Remove folder"
        >
          <X size={14} />
        </button>
      </div>
    </div>
  );
}
