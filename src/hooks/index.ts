// Directory and file system hooks
export { useDirectory } from "./useDirectory";
export { useThumbnail, preloadThumbnails, clearThumbnailMemoryCache } from "./useThumbnail";

// Settings hook with local persistence (+ Convex sync when configured)
export { useSyncedSettings } from "./useSyncedSettings";

// Re-export types from settings store
export {
  useSettingsStore,
  getEffectiveTheme,
  type Theme,
  type ViewMode,
  type SortBy,
  type SortDirection,
  type AIModel,
} from "../stores/settings-store";

// Note: The following hooks require Convex to be initialized
// Run `npx convex dev` and add your VITE_CONVEX_URL to enable
// - useConvexUser
// - useConvexSettings
// - useOrganizeHistory
// - useRenameHistory
// - useUsageStats
