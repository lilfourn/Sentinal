import { File } from 'lucide-react';
import { cn } from '../../lib/utils';
import { FolderIcon } from '../icons/FolderIcon';
import { InlineNameEditor } from './InlineNameEditor';

interface NewItemRowProps {
  type: 'file' | 'folder';
  style?: React.CSSProperties;
  onConfirm: (name: string) => void;
  onCancel: () => void;
}

export function NewItemRow({ type, style, onConfirm, onCancel }: NewItemRowProps) {
  const defaultName = type === 'folder' ? 'untitled folder' : 'untitled';

  return (
    <div
      style={style}
      className={cn(
        'flex items-center gap-3 px-4 cursor-default select-none',
        'bg-[color:var(--color-file-selected-focused)]'
      )}
    >
      {/* Icon */}
      {type === 'folder' ? (
        <FolderIcon size={18} className="flex-shrink-0" />
      ) : (
        <File size={18} className="text-gray-400 dark:text-gray-500" />
      )}

      {/* Name input */}
      <InlineNameEditor
        initialValue={defaultName}
        onConfirm={onConfirm}
        onCancel={onCancel}
        selectNameOnly={false}
        className="flex-1"
      />

      {/* Modified date placeholder */}
      <span className="w-36 text-xs text-gray-400 dark:text-gray-600 truncate">
        —
      </span>

      {/* Size placeholder */}
      <span className="w-20 text-xs text-gray-400 dark:text-gray-600 text-right">
        —
      </span>
    </div>
  );
}
