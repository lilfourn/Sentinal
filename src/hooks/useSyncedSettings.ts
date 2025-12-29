import { useCallback } from "react";
import {
  useSettingsStore,
  type Theme,
  type ViewMode,
  type SortBy,
  type SortDirection,
  type AIModel,
} from "../stores/settings-store";

/**
 * Hook that provides settings with Convex sync capability
 *
 * Currently uses local-only storage. When Convex is configured,
 * this hook will sync settings bidirectionally.
 *
 * To enable Convex sync:
 * 1. Run `npx convex dev` to initialize Convex
 * 2. Set VITE_CONVEX_URL in .env.local
 * 3. Set VITE_CLERK_PUBLISHABLE_KEY for authentication
 */
export function useSyncedSettings() {
  const store = useSettingsStore();

  // Check if Convex is configured
  const isConvexConfigured = Boolean(import.meta.env.VITE_CONVEX_URL);
  const isClerkConfigured = Boolean(import.meta.env.VITE_CLERK_PUBLISHABLE_KEY);

  /**
   * Update settings (local-first with optional cloud sync)
   */
  const updateSettings = useCallback(
    async (
      updates: Partial<{
        theme: Theme;
        autoRenameEnabled: boolean;
        showHiddenFiles: boolean;
        defaultView: ViewMode;
        sortBy: SortBy;
        sortDirection: SortDirection;
        aiModel: AIModel;
      }>
    ) => {
      // Update local store immediately
      if (updates.theme !== undefined) store.setTheme(updates.theme);
      if (updates.autoRenameEnabled !== undefined)
        store.setAutoRename(updates.autoRenameEnabled);
      if (updates.showHiddenFiles !== undefined)
        store.setShowHiddenFiles(updates.showHiddenFiles);
      if (updates.defaultView !== undefined)
        store.setDefaultView(updates.defaultView);
      if (updates.sortBy !== undefined) store.setSortBy(updates.sortBy);
      if (updates.sortDirection !== undefined)
        store.setSortDirection(updates.sortDirection);
      if (updates.aiModel !== undefined) store.setAIModel(updates.aiModel);

      // TODO: When Convex is configured, sync to cloud here
      // This will be implemented after running `npx convex dev`
    },
    [store]
  );

  return {
    ...store,
    updateSettings,
    isAuthenticated: false, // Will be true when Clerk is configured
    isConvexAvailable: isConvexConfigured,
    isClerkAvailable: isClerkConfigured,
  };
}
