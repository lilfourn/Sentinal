import { useMemo } from 'react';
import { GitCompare, ChevronDown, ChevronUp } from 'lucide-react';
import { cn } from '../../lib/utils';
import { DiffTree, useDiffTreeState } from './DiffTree';
import { DiffSummary } from './DiffSummary';
import type { OrganizePlan } from '../../stores/organize-store';
import type { DiffNode } from '../../types/vfs';

interface DiffViewProps {
  /** The organization plan to visualize */
  plan: OrganizePlan;
  /** Target folder being organized */
  targetFolder: string;
  /** Optional additional class names */
  className?: string;
}

/**
 * Side-by-side diff view showing current state vs proposed changes.
 */
export function DiffView({ plan, targetFolder, className }: DiffViewProps) {
  // Build diff trees from the plan
  const { currentTree, proposedTree, allFolderPaths } = useMemo(
    () => buildDiffTrees(plan, targetFolder),
    [plan, targetFolder]
  );

  // Shared expanded state for both trees
  const { expandedFolders, toggle, expandAll, collapseAll } = useDiffTreeState(
    allFolderPaths.slice(0, 5) // Start with first 5 folders expanded
  );

  const allExpanded = expandedFolders.size === allFolderPaths.length;

  return (
    <div className={cn('flex flex-col h-full', className)}>
      {/* Header */}
      <div className="flex items-center justify-between px-3 py-2 border-b border-white/5 bg-white/[0.02]">
        <div className="flex items-center gap-2">
          <GitCompare size={14} className="text-orange-500/70" />
          <span className="text-xs font-medium text-gray-300">Change Preview</span>
        </div>
        <button
          onClick={() => (allExpanded ? collapseAll() : expandAll(allFolderPaths))}
          className="flex items-center gap-1 px-2 py-1 rounded text-xs text-gray-400 hover:text-gray-300 hover:bg-white/5"
        >
          {allExpanded ? (
            <>
              <ChevronUp size={12} />
              Collapse
            </>
          ) : (
            <>
              <ChevronDown size={12} />
              Expand
            </>
          )}
        </button>
      </div>

      {/* Summary bar */}
      <div className="px-3 py-2 border-b border-white/5">
        <DiffSummary plan={plan} />
      </div>

      {/* Two-column diff */}
      <div className="flex-1 flex overflow-hidden">
        {/* Current state column */}
        <div className="flex-1 flex flex-col border-r border-white/5 overflow-hidden">
          <div className="px-3 py-1.5 border-b border-white/5 bg-red-500/5">
            <span className="text-xs font-medium text-red-400">Current</span>
          </div>
          <div className="flex-1 overflow-auto p-2">
            <DiffTree
              tree={currentTree}
              expandedFolders={expandedFolders}
              onToggle={toggle}
              side="current"
            />
          </div>
        </div>

        {/* Proposed state column */}
        <div className="flex-1 flex flex-col overflow-hidden">
          <div className="px-3 py-1.5 border-b border-white/5 bg-green-500/5">
            <span className="text-xs font-medium text-green-400">Proposed</span>
          </div>
          <div className="flex-1 overflow-auto p-2">
            <DiffTree
              tree={proposedTree}
              expandedFolders={expandedFolders}
              onToggle={toggle}
              side="proposed"
            />
          </div>
        </div>
      </div>
    </div>
  );
}

/**
 * Builds diff trees from an organization plan.
 * Returns both current and proposed tree views.
 */
function buildDiffTrees(
  plan: OrganizePlan,
  _targetFolder: string
): {
  currentTree: DiffNode[];
  proposedTree: DiffNode[];
  allFolderPaths: string[];
} {
  // Track changes by path
  const removedPaths = new Set<string>();
  const addedPaths = new Set<string>();
  const movedFrom = new Map<string, string>(); // source -> destination
  const movedTo = new Map<string, string>(); // destination -> source
  const createdFolders = new Set<string>();

  // Analyze operations
  for (const op of plan.operations) {
    switch (op.type) {
      case 'move':
        if (op.source && op.destination) {
          movedFrom.set(op.source, op.destination);
          movedTo.set(op.destination, op.source);
        }
        break;
      case 'rename':
        if (op.path && op.newName) {
          const parentPath = op.path.split('/').slice(0, -1).join('/');
          const newPath = `${parentPath}/${op.newName}`;
          movedFrom.set(op.path, newPath);
          movedTo.set(newPath, op.path);
        }
        break;
      case 'trash':
        if (op.path) {
          removedPaths.add(op.path);
        }
        break;
      case 'create_folder':
        if (op.path) {
          addedPaths.add(op.path);
          createdFolders.add(op.path);
        }
        break;
      case 'copy':
        if (op.destination) {
          addedPaths.add(op.destination);
        }
        break;
    }
  }

  // Build current tree (showing what will be removed/moved)
  const currentTree: DiffNode[] = [];
  const proposedTree: DiffNode[] = [];
  const allFolderPaths: string[] = [];

  // For simplicity, create a flat list of affected items
  // In a full implementation, you'd build a proper tree structure

  // Current side: show items being removed/moved away
  for (const path of removedPaths) {
    currentTree.push({
      name: path.split('/').pop() || '',
      path,
      isDirectory: false, // Would need to check actual type
      changeType: 'removed',
    });
  }

  for (const [source, dest] of movedFrom) {
    currentTree.push({
      name: source.split('/').pop() || '',
      path: source,
      isDirectory: createdFolders.has(dest),
      changeType: 'moved',
      linkedPath: dest,
    });
  }

  // Proposed side: show items being added/moved to
  for (const path of addedPaths) {
    const isFolder = createdFolders.has(path);
    proposedTree.push({
      name: path.split('/').pop() || '',
      path,
      isDirectory: isFolder,
      changeType: 'added',
    });
    if (isFolder) {
      allFolderPaths.push(path);
    }
  }

  for (const [dest, source] of movedTo) {
    if (!addedPaths.has(dest)) {
      proposedTree.push({
        name: dest.split('/').pop() || '',
        path: dest,
        isDirectory: false,
        changeType: 'moved',
        linkedPath: source,
      });
    }
  }

  // Sort trees alphabetically
  currentTree.sort((a, b) => a.name.localeCompare(b.name));
  proposedTree.sort((a, b) => a.name.localeCompare(b.name));

  return {
    currentTree,
    proposedTree,
    allFolderPaths,
  };
}
