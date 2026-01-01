import { File, Folder, Image, Monitor, HardDrive } from 'lucide-react';
import type { PendingDropItem } from '../../hooks/useChatDropZone';

interface DropZoneOverlayProps {
  /** Whether the overlay should be visible */
  isVisible: boolean;
  /** Source of the drag (internal from file list, or external from Finder) */
  dragSource: 'internal' | 'external' | null;
  /** Items pending to be dropped */
  pendingItems: PendingDropItem[];
}

/** Get icon for a pending item based on its type */
function ItemIcon({ item }: { item: PendingDropItem }) {
  if (item.type === 'folder') {
    return <Folder size={14} className="text-blue-400" />;
  }

  if (item.type === 'file') {
    // Check if it's an image based on mimeType
    if (item.mimeType?.startsWith('image/')) {
      return <Image size={14} className="text-purple-400" />;
    }
    return <File size={14} className="text-gray-400" />;
  }

  // Unknown type (external drag before metadata fetch)
  return <File size={14} className="text-gray-500" />;
}

/** Source indicator icon */
function SourceIcon({ source }: { source: 'internal' | 'external' }) {
  if (source === 'internal') {
    return <Monitor size={16} className="text-orange-400" />;
  }
  return <HardDrive size={16} className="text-orange-400" />;
}

/** Preview chip for a single pending item */
function PreviewChip({ item }: { item: PendingDropItem }) {
  return (
    <div className="flex items-center gap-1.5 px-2 py-1 bg-white/10 rounded-md text-xs">
      <ItemIcon item={item} />
      <span className="truncate max-w-[120px] text-gray-300">{item.name}</span>
    </div>
  );
}

export function DropZoneOverlay({ isVisible, dragSource, pendingItems }: DropZoneOverlayProps) {
  if (!isVisible) {
    return null;
  }

  const itemCount = pendingItems.length;
  const showItems = pendingItems.slice(0, 3);
  const remainingCount = itemCount - showItems.length;

  return (
    <div
      className={`
        absolute inset-0 z-10 m-4
        flex flex-col items-center justify-center gap-3
        border-2 border-dashed border-orange-500 rounded-lg
        bg-orange-500/10 backdrop-blur-sm
        pointer-events-none
        transition-all duration-200 ease-out
        animate-in fade-in zoom-in-95
      `}
    >
      {/* Source indicator */}
      {dragSource && (
        <div className="flex items-center gap-2 text-orange-400/80 text-xs">
          <SourceIcon source={dragSource} />
          <span>{dragSource === 'internal' ? 'From app' : 'From Finder'}</span>
        </div>
      )}

      {/* Main message */}
      <div className="text-center">
        <p className="text-orange-500 font-medium text-base">
          Drop to add context
        </p>
        <p className="text-xs text-orange-400/70 mt-0.5">
          {itemCount === 0
            ? 'Files or folders'
            : itemCount === 1
              ? '1 item'
              : `${itemCount} items`}
        </p>
      </div>

      {/* Item preview chips */}
      {showItems.length > 0 && (
        <div className="flex flex-wrap items-center justify-center gap-1.5 max-w-[280px]">
          {showItems.map((item, index) => (
            <PreviewChip key={`${item.path}-${index}`} item={item} />
          ))}
          {remainingCount > 0 && (
            <div className="px-2 py-1 bg-white/5 rounded-md text-xs text-gray-400">
              +{remainingCount} more
            </div>
          )}
        </div>
      )}
    </div>
  );
}
