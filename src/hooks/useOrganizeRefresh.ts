import { useEffect, useRef } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useOrganizeStore } from '../stores/organize-store';

/**
 * Hook that watches for organization completion and triggers a file list refresh.
 *
 * When an organization operation completes successfully, this hook invalidates
 * the React Query cache for directory listings, causing them to re-fetch and
 * display the newly organized file structure.
 *
 * Usage: Place this hook in a component near the root of the app, such as App.tsx
 * or MainView.tsx. It doesn't render anything, just handles the refresh side effect.
 */
export function useOrganizeRefresh() {
  const queryClient = useQueryClient();
  const lastCompletedAt = useOrganizeStore((state) => state.lastCompletedAt);
  const completedTargetFolder = useOrganizeStore((state) => state.completedTargetFolder);

  // Track the last completion we've handled to avoid duplicate refreshes
  const lastHandledRef = useRef<number | null>(null);

  useEffect(() => {
    // Only trigger refresh if:
    // 1. There's a completion timestamp
    // 2. We haven't already handled this completion
    // 3. There's a target folder
    if (
      lastCompletedAt !== null &&
      lastCompletedAt !== lastHandledRef.current &&
      completedTargetFolder
    ) {
      lastHandledRef.current = lastCompletedAt;

      // Invalidate all directory queries to refresh file listings
      // This is a broad invalidation that ensures all views update
      queryClient.invalidateQueries({ queryKey: ['directory'] });

      console.log('[useOrganizeRefresh] Organization complete, refreshing file listings for:', completedTargetFolder);
    }
  }, [lastCompletedAt, completedTargetFolder, queryClient]);
}
