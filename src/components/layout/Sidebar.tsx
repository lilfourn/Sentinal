import { useUserDirectories } from '../../hooks/useDirectory';
import { useNavigationStore } from '../../stores/navigation-store';
import { SidebarItem } from '../sidebar/SidebarItem';

export function Sidebar() {
  const { data: directories, isLoading } = useUserDirectories();
  const { currentPath, navigateTo } = useNavigationStore();

  const handleClick = (path: string) => {
    navigateTo(path);
  };

  return (
    <aside className="glass-sidebar w-52 flex-shrink-0 h-full overflow-hidden flex flex-col">
      {/* Favorites section */}
      <div className="p-3 flex-1 overflow-y-auto">
        <h3 className="text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider mb-2 px-1">
          Favorites
        </h3>
        <nav className="space-y-0.5">
          {isLoading ? (
            <div className="px-3 py-2 text-sm text-gray-400">Loading...</div>
          ) : (
            directories?.map(({ name, path }) => (
              <SidebarItem
                key={path}
                name={name}
                path={path}
                isActive={currentPath === path}
                onClick={handleClick}
              />
            ))
          )}
        </nav>
      </div>
    </aside>
  );
}
