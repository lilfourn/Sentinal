import { cn } from '../../lib/utils';
import {
  Home,
  FileText,
  Download,
  Image,
  Music,
  Video,
  Folder,
  type LucideIcon,
} from 'lucide-react';

interface SidebarItemProps {
  name: string;
  path: string;
  isActive: boolean;
  onClick: (path: string) => void;
}

const iconMap: Record<string, LucideIcon> = {
  Home: Home,
  Desktop: Folder,
  Documents: FileText,
  Downloads: Download,
  Pictures: Image,
  Music: Music,
  Videos: Video,
  Movies: Video,
};

export function SidebarItem({ name, path, isActive, onClick }: SidebarItemProps) {
  const Icon = iconMap[name] || Folder;

  return (
    <button
      onClick={() => onClick(path)}
      className={cn(
        'w-full flex items-center gap-2 px-3 py-1.5 rounded-md text-left',
        'transition-colors duration-150',
        isActive
          ? 'bg-[color:var(--color-sidebar-item-selected)] text-orange-500'
          : 'hover:bg-[color:var(--color-sidebar-item-hover)] text-gray-700 dark:text-gray-300'
      )}
    >
      <Icon
        size={16}
        className={cn(
          isActive ? 'text-orange-500' : 'text-gray-500 dark:text-gray-400'
        )}
      />
      <span className="text-sm font-medium truncate">{name}</span>
    </button>
  );
}
