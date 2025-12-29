import { useEffect, useRef } from "react";
import { useUser } from "@clerk/clerk-react";
import { useMutation } from "convex/react";
import { api } from "../../../convex/_generated/api";

/**
 * Invisible component that syncs Clerk user to Convex on sign-in.
 * Creates user record and default settings in Convex database.
 */
export function AuthSync() {
  const { isSignedIn, isLoaded } = useUser();
  const getOrCreateUser = useMutation(api.users.getOrCreateUser);
  const hasSynced = useRef(false);

  useEffect(() => {
    if (isLoaded && isSignedIn && !hasSynced.current) {
      hasSynced.current = true;
      getOrCreateUser()
        .then(() => {
          console.log("[AuthSync] User synced to Convex");
        })
        .catch((error) => {
          console.error("[AuthSync] Failed to sync user:", error);
          // Reset so we can retry
          hasSynced.current = false;
        });
    }
  }, [isLoaded, isSignedIn, getOrCreateUser]);

  return null;
}
