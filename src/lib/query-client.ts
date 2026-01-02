import { QueryClient } from '@tanstack/react-query';

/**
 * Shared QueryClient instance for TanStack Query.
 *
 * Exported separately so it can be used in both:
 * - React components (via QueryClientProvider)
 * - Non-React code (like Zustand stores for cache invalidation)
 */
export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 1000 * 60,
      retry: 1,
    },
  },
});
