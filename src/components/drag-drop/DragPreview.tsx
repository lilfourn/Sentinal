import { File } from 'lucide-react';
import { FolderIcon } from '../icons/FolderIcon';
import type { DragState } from '../../types/drag-drop';

interface DragPreviewProps {
  dragState: DragState;
}

export function DragPreview({ dragState }: DragPreviewProps) {
  const { items, position, isCopy } = dragState;
  const count = items.length;
  const firstItem = items[0];

  if (!firstItem) return null;

  return (
    <div
      className="fixed pointer-events-none z-50 flex items-center gap-2 px-3 py-2 rounded-lg shadow-lg bg-white/95 dark:bg-gray-800/95 backdrop-blur-sm border border-gray-200 dark:border-gray-700"
      style={{
        left: position.x + 16,
        top: position.y + 16,
        transform: 'translate(0, 0)',
      }}
    >
      {/* Icon */}
      {firstItem.isDirectory ? (
        <FolderIcon size={20} />
      ) : (
        <File size={20} className="text-gray-400" />
      )}

      {/* Label */}
      <span className="text-sm text-gray-800 dark:text-gray-200 max-w-48 truncate">
        {count === 1 ? firstItem.name : `${count} items`}
      </span>

      {/* Badge for count > 1 */}
      {count > 1 && (
        <span className="flex items-center justify-center min-w-5 h-5 px-1.5 rounded-full bg-orange-500 text-white text-xs font-medium">
          {count}
        </span>
      )}

      {/* Copy indicator */}
      {isCopy && (
        <span className="text-xs text-green-600 dark:text-green-400 font-medium ml-1">
          + Copy
        </span>
      )}
    </div>
  );
}
