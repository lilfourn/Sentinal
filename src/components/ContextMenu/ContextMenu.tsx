import { useEffect, useRef, useState, useLayoutEffect } from 'react';
import {
  Sparkles,
  Copy,
  Clipboard,
  Trash2,
  Edit3,
  FileText,
  ExternalLink,
} from 'lucide-react';
import { cn } from '../../lib/utils';

export interface ContextMenuPosition {
  x: number;
  y: number;
}

export interface ContextMenuItem {
  id: string;
  label: string;
  icon?: React.ReactNode;
  shortcut?: string;
  disabled?: boolean;
  danger?: boolean;
  separator?: boolean;
  onClick?: () => void;
}

interface ContextMenuProps {
  position: ContextMenuPosition | null;
  items: ContextMenuItem[];
  onClose: () => void;
}

export function ContextMenu({ position, items, onClose }: ContextMenuProps) {
  const menuRef = useRef<HTMLDivElement>(null);
  const [adjustedPos, setAdjustedPos] = useState<ContextMenuPosition | null>(null);

  // Close on click outside
  useEffect(() => {
    if (!position) return;

    const handleClickOutside = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        onClose();
      }
    };

    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose();
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    document.addEventListener('keydown', handleEscape);

    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
      document.removeEventListener('keydown', handleEscape);
    };
  }, [position, onClose]);

  // Calculate adjusted position after menu renders
  useLayoutEffect(() => {
    if (!position || !menuRef.current) {
      setAdjustedPos(null);
      return;
    }

    const menu = menuRef.current;
    const rect = menu.getBoundingClientRect();

    // Use document.documentElement for more reliable viewport dimensions
    const viewportWidth = document.documentElement.clientWidth;
    const viewportHeight = document.documentElement.clientHeight;

    const padding = 8; // Minimum padding from edges

    let x = position.x;
    let y = position.y;

    // Adjust if menu extends beyond right edge
    if (x + rect.width > viewportWidth - padding) {
      x = Math.max(padding, viewportWidth - rect.width - padding);
    }

    // Adjust if menu extends beyond bottom edge
    if (y + rect.height > viewportHeight - padding) {
      y = Math.max(padding, viewportHeight - rect.height - padding);
    }

    // Ensure minimum padding from left/top edges
    x = Math.max(padding, x);
    y = Math.max(padding, y);

    setAdjustedPos({ x, y });
  }, [position]);

  if (!position) return null;

  // Use adjusted position if calculated, otherwise use original (will be corrected after render)
  const finalPos = adjustedPos || position;

  return (
    <div
      ref={menuRef}
      className={cn(
        'fixed z-[100] min-w-[180px] py-1',
        'bg-white dark:bg-gray-800 rounded-lg shadow-xl',
        'border border-gray-200 dark:border-gray-700',
        // Only show animation after position is calculated to prevent flicker
        adjustedPos ? 'animate-in fade-in-0 zoom-in-95 duration-100' : 'opacity-0'
      )}
      style={{
        left: finalPos.x,
        top: finalPos.y,
      }}
    >
      {items.map((item, index) => {
        if (item.separator) {
          return (
            <div
              key={`sep-${index}`}
              className="h-px my-1 bg-gray-200 dark:bg-gray-700"
            />
          );
        }

        return (
          <button
            key={item.id}
            onClick={() => {
              if (!item.disabled && item.onClick) {
                item.onClick();
                onClose();
              }
            }}
            disabled={item.disabled}
            className={cn(
              'w-full flex items-center gap-3 px-3 py-1.5 text-sm text-left',
              'transition-colors',
              item.disabled && 'opacity-50 cursor-not-allowed',
              !item.disabled && !item.danger && 'hover:bg-gray-100 dark:hover:bg-gray-700',
              !item.disabled && item.danger && 'hover:bg-red-100 dark:hover:bg-red-900/30 text-red-600 dark:text-red-400'
            )}
          >
            {item.icon && (
              <span className="w-4 h-4 flex items-center justify-center">
                {item.icon}
              </span>
            )}
            <span className="flex-1">{item.label}</span>
            {item.shortcut && (
              <span className="text-xs text-gray-400 dark:text-gray-500">
                {item.shortcut}
              </span>
            )}
          </button>
        );
      })}
    </div>
  );
}

// Helper to build context menu items for files/folders
export function buildFileContextMenuItems(
  entry: { name: string; path: string; isDirectory: boolean },
  handlers: {
    onOpen?: () => void;
    onOrganizeWithAI?: () => void;
    onRename?: () => void;
    onCopy?: () => void;
    onPaste?: () => void;
    onMoveToTrash?: () => void;
    onGetInfo?: () => void;
  }
): ContextMenuItem[] {
  const items: ContextMenuItem[] = [];

  // Open
  items.push({
    id: 'open',
    label: 'Open',
    icon: <ExternalLink size={14} />,
    onClick: handlers.onOpen,
  });

  // Organize with AI (for folders only)
  if (entry.isDirectory) {
    items.push({
      id: 'organize-ai',
      label: 'Organize with AI',
      icon: <Sparkles size={14} className="text-purple-500" />,
      onClick: handlers.onOrganizeWithAI,
    });
  }

  items.push({ id: 'sep1', label: '', separator: true });

  // Rename
  items.push({
    id: 'rename',
    label: 'Rename',
    icon: <Edit3 size={14} />,
    onClick: handlers.onRename,
  });

  // Copy
  items.push({
    id: 'copy',
    label: 'Copy',
    icon: <Copy size={14} />,
    shortcut: '⌘C',
    onClick: handlers.onCopy,
  });

  // Paste (if in a folder)
  if (entry.isDirectory) {
    items.push({
      id: 'paste',
      label: 'Paste',
      icon: <Clipboard size={14} />,
      shortcut: '⌘V',
      disabled: true, // Enable when clipboard has content
      onClick: handlers.onPaste,
    });
  }

  items.push({ id: 'sep2', label: '', separator: true });

  // Get Info
  items.push({
    id: 'get-info',
    label: 'Get Info',
    icon: <FileText size={14} />,
    shortcut: '⌘I',
    onClick: handlers.onGetInfo,
  });

  items.push({ id: 'sep3', label: '', separator: true });

  // Move to Trash
  items.push({
    id: 'trash',
    label: 'Move to Trash',
    icon: <Trash2 size={14} />,
    shortcut: '⌘⌫',
    danger: true,
    onClick: handlers.onMoveToTrash,
  });

  return items;
}
