import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ClerkProvider } from "@clerk/clerk-react";
import { ConvexProviderWithClerk } from "convex/react-clerk";
import { ConvexReactClient } from "convex/react";
import { useAuth } from "@clerk/clerk-react";
import { AuthSync } from "./components/auth/AuthSync";
import App from "./App";

// TanStack Query client for useDirectory hook
const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 1000 * 60,
      retry: 1,
    },
  },
});

// Clerk publishable key
const PUBLISHABLE_KEY = import.meta.env.VITE_CLERK_PUBLISHABLE_KEY;

// Convex client (only if URL is configured)
const CONVEX_URL = import.meta.env.VITE_CONVEX_URL;
const convex = CONVEX_URL ? new ConvexReactClient(CONVEX_URL) : null;

// Inner component that uses useAuth (must be inside ClerkProvider)
function ConvexClientProvider({ children }: { children: React.ReactNode }) {
  if (!convex) {
    return <>{children}</>;
  }
  return (
    <ConvexProviderWithClerk client={convex} useAuth={useAuth}>
      <AuthSync />
      {children}
    </ConvexProviderWithClerk>
  );
}

// Main app with providers
function Root() {
  // If Clerk is not configured, run without auth
  if (!PUBLISHABLE_KEY) {
    console.info("Running without Clerk auth. Set VITE_CLERK_PUBLISHABLE_KEY to enable.");
    return (
      <StrictMode>
        <QueryClientProvider client={queryClient}>
          <App />
        </QueryClientProvider>
      </StrictMode>
    );
  }

  return (
    <StrictMode>
      <ClerkProvider publishableKey={PUBLISHABLE_KEY} afterSignOutUrl="/">
        <QueryClientProvider client={queryClient}>
          <ConvexClientProvider>
            <App />
          </ConvexClientProvider>
        </QueryClientProvider>
      </ClerkProvider>
    </StrictMode>
  );
}

createRoot(document.getElementById("root")!).render(<Root />);
