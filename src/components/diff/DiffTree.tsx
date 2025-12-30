import { useState, useCallback } from 'react';
import { ChevronRight, ChevronDown, Folder, File, ArrowRight } from 'lucide-react';
import { cn } from '../../lib/utils';
import type { DiffNode } from '../../types/vfs';

interface DiffTreeProps {
  /** Tree nodes to display */
  tree: DiffNode[];
  /** Set of expanded folder paths */
  expandedFolders: Set<string>;
  /** Callback when a folder is toggled */
  onToggle: (path: string) => void;
  /** Which side of the diff this tree represents */
  side: 'current' | 'proposed';
  /** Optional additional class names */
  className?: string;
}

/**
 * Expandable folder tree component for diff view.
 * Shows files and folders with color-coded change indicators.
 */
export function DiffTree({
  tree,
  expandedFolders,
  onToggle,
  side,
  className,
}: DiffTreeProps) {
  return (
    <div className={cn('text-sm', className)}>
      {tree.map((node) => (
        <DiffTreeNode
          key={node.path}
          node={node}
          depth={0}
          expandedFolders={expandedFolders}
          onToggle={onToggle}
          side={side}
        />
      ))}
      {tree.length === 0 && (
        <div className="text-xs text-gray-500 italic p-2">No items</div>
      )}
    </div>
  );
}

interface DiffTreeNodeProps {
  node: DiffNode;
  depth: number;
  expandedFolders: Set<string>;
  onToggle: (path: string) => void;
  side: 'current' | 'proposed';
}

function DiffTreeNode({
  node,
  depth,
  expandedFolders,
  onToggle,
  side,
}: DiffTreeNodeProps) {
  const isExpanded = expandedFolders.has(node.path);
  const hasChildren = node.isDirectory && node.children && node.children.length > 0;

  const handleToggle = useCallback(() => {
    if (hasChildren) {
      onToggle(node.path);
    }
  }, [hasChildren, node.path, onToggle]);

  // Determine styling based on change type
  const changeClasses = getChangeClasses(node.changeType, side);

  return (
    <div>
      {/* Node row */}
      <div
        className={cn(
          'flex items-center gap-1 py-0.5 px-1 rounded cursor-default',
          'hover:bg-white/5',
          changeClasses.row
        )}
        style={{ paddingLeft: `${depth * 16 + 4}px` }}
        onClick={handleToggle}
      >
        {/* Expand/collapse toggle */}
        {hasChildren ? (
          <button
            className="p-0.5 hover:bg-white/10 rounded"
            onClick={(e) => {
              e.stopPropagation();
              handleToggle();
            }}
          >
            {isExpanded ? (
              <ChevronDown size={12} className="text-gray-500" />
            ) : (
              <ChevronRight size={12} className="text-gray-500" />
            )}
          </button>
        ) : (
          <span className="w-4" />
        )}

        {/* Icon */}
        {node.isDirectory ? (
          <Folder size={14} className={cn('flex-shrink-0', changeClasses.icon)} />
        ) : (
          <File size={14} className={cn('flex-shrink-0', changeClasses.icon)} />
        )}

        {/* Name */}
        <span className={cn('truncate flex-1', changeClasses.text)}>{node.name}</span>

        {/* Change indicator badge */}
        <ChangeIndicator changeType={node.changeType} side={side} />

        {/* Link indicator for moved items */}
        {node.linkedPath && node.changeType === 'moved' && (
          <div className="flex items-center gap-1 text-[10px] text-yellow-500/70">
            <ArrowRight size={10} />
            <span className="truncate max-w-[80px]">
              {node.linkedPath.split('/').pop()}
            </span>
          </div>
        )}
      </div>

      {/* Children */}
      {isExpanded && hasChildren && (
        <div>
          {node.children!.map((child) => (
            <DiffTreeNode
              key={child.path}
              node={child}
              depth={depth + 1}
              expandedFolders={expandedFolders}
              onToggle={onToggle}
              side={side}
            />
          ))}
        </div>
      )}
    </div>
  );
}

interface ChangeIndicatorProps {
  changeType: DiffNode['changeType'];
  side: 'current' | 'proposed';
}

function ChangeIndicator({ changeType, side }: ChangeIndicatorProps) {
  if (changeType === 'unchanged') return null;

  const labels: Record<DiffNode['changeType'], { current: string; proposed: string }> = {
    added: { current: '', proposed: 'NEW' },
    removed: { current: 'DEL', proposed: '' },
    moved: { current: 'FROM', proposed: 'TO' },
    unchanged: { current: '', proposed: '' },
  };

  const label = labels[changeType][side];
  if (!label) return null;

  const colorClasses: Record<DiffNode['changeType'], string> = {
    added: 'bg-green-500/20 text-green-400 border-green-500/30',
    removed: 'bg-red-500/20 text-red-400 border-red-500/30',
    moved: 'bg-yellow-500/20 text-yellow-400 border-yellow-500/30',
    unchanged: '',
  };

  return (
    <span
      className={cn(
        'px-1 py-0.5 rounded text-[9px] font-medium border',
        colorClasses[changeType]
      )}
    >
      {label}
    </span>
  );
}

function getChangeClasses(
  changeType: DiffNode['changeType'],
  side: 'current' | 'proposed'
): { row: string; icon: string; text: string } {
  switch (changeType) {
    case 'added':
      return side === 'proposed'
        ? {
            row: 'bg-green-500/5',
            icon: 'text-green-500',
            text: 'text-green-400',
          }
        : { row: '', icon: 'text-gray-500', text: 'text-gray-500' };

    case 'removed':
      return side === 'current'
        ? {
            row: 'bg-red-500/5',
            icon: 'text-red-500',
            text: 'text-red-400 line-through',
          }
        : { row: '', icon: 'text-gray-500', text: 'text-gray-500' };

    case 'moved':
      return {
        row: 'bg-yellow-500/5',
        icon: 'text-yellow-500',
        text: 'text-yellow-400',
      };

    case 'unchanged':
    default:
      return {
        row: '',
        icon: 'text-gray-500',
        text: 'text-gray-400',
      };
  }
}

/**
 * Hook to manage expanded folder state for DiffTree.
 */
export function useDiffTreeState(initialExpanded: string[] = []) {
  const [expandedFolders, setExpandedFolders] = useState<Set<string>>(
    new Set(initialExpanded)
  );

  const toggle = useCallback((path: string) => {
    setExpandedFolders((prev) => {
      const next = new Set(prev);
      if (next.has(path)) {
        next.delete(path);
      } else {
        next.add(path);
      }
      return next;
    });
  }, []);

  const expandAll = useCallback((paths: string[]) => {
    setExpandedFolders(new Set(paths));
  }, []);

  const collapseAll = useCallback(() => {
    setExpandedFolders(new Set());
  }, []);

  return {
    expandedFolders,
    toggle,
    expandAll,
    collapseAll,
  };
}
