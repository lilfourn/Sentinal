import { UserButton, useUser } from "@clerk/clerk-react";

/**
 * User menu button for the toolbar
 * Shows user avatar with dropdown for account management
 */
export function UserMenu() {
  const { isSignedIn, user } = useUser();

  if (!isSignedIn) {
    return null;
  }

  return (
    <div className="flex items-center gap-2">
      <span className="text-xs text-gray-400 hidden sm:block">
        {user?.firstName || user?.emailAddresses[0]?.emailAddress}
      </span>
      <UserButton
        appearance={{
          elements: {
            avatarBox: "w-7 h-7",
            userButtonPopoverCard: "bg-gray-800 border border-gray-700",
            userButtonPopoverActionButton: "text-gray-300 hover:bg-gray-700",
            userButtonPopoverActionButtonText: "text-gray-300",
            userButtonPopoverActionButtonIcon: "text-gray-400",
            userButtonPopoverFooter: "hidden",
          },
        }}
      />
    </div>
  );
}
