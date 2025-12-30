import { FolderPlus, ArrowRightLeft, Trash2 } from 'lucide-react';
import { cn } from '../../lib/utils';
import type { OrganizePlan } from '../../stores/organize-store';

interface DiffSummaryProps {
  /** The organization plan to summarize */
  plan: OrganizePlan;
  /** Optional additional class names */
  className?: string;
}

interface OperationCounts {
  moves: number;
  creates: number;
  deletes: number;
  renames: number;
  copies: number;
}

/**
 * Summarizes the operation counts from an organization plan.
 */
function countOperations(plan: OrganizePlan): OperationCounts {
  const counts: OperationCounts = {
    moves: 0,
    creates: 0,
    deletes: 0,
    renames: 0,
    copies: 0,
  };

  for (const op of plan.operations) {
    switch (op.type) {
      case 'move':
        counts.moves++;
        break;
      case 'create_folder':
        counts.creates++;
        break;
      case 'trash':
        counts.deletes++;
        break;
      case 'rename':
        counts.renames++;
        break;
      case 'copy':
        counts.copies++;
        break;
    }
  }

  return counts;
}

/**
 * Operation count summary component showing badges for moves, creates, and deletes.
 */
export function DiffSummary({ plan, className }: DiffSummaryProps) {
  const counts = countOperations(plan);
  const total = plan.operations.length;

  if (total === 0) {
    return (
      <div className={cn('flex items-center gap-2 text-sm text-gray-400', className)}>
        <span>No changes proposed</span>
      </div>
    );
  }

  return (
    <div className={cn('flex flex-wrap items-center gap-2', className)}>
      {/* Total operations */}
      <span className="text-xs text-gray-500">
        {total} {total === 1 ? 'operation' : 'operations'}:
      </span>

      {/* Moves badge */}
      {counts.moves > 0 && (
        <Badge
          icon={<ArrowRightLeft size={12} />}
          count={counts.moves}
          label={counts.moves === 1 ? 'move' : 'moves'}
          colorClass="bg-yellow-500/20 text-yellow-400 border-yellow-500/30"
        />
      )}

      {/* Creates badge */}
      {counts.creates > 0 && (
        <Badge
          icon={<FolderPlus size={12} />}
          count={counts.creates}
          label={counts.creates === 1 ? 'new folder' : 'new folders'}
          colorClass="bg-green-500/20 text-green-400 border-green-500/30"
        />
      )}

      {/* Deletes badge */}
      {counts.deletes > 0 && (
        <Badge
          icon={<Trash2 size={12} />}
          count={counts.deletes}
          label={counts.deletes === 1 ? 'deletion' : 'deletions'}
          colorClass="bg-red-500/20 text-red-400 border-red-500/30"
        />
      )}

      {/* Renames badge */}
      {counts.renames > 0 && (
        <Badge
          icon={<span className="text-[10px]">Aa</span>}
          count={counts.renames}
          label={counts.renames === 1 ? 'rename' : 'renames'}
          colorClass="bg-blue-500/20 text-blue-400 border-blue-500/30"
        />
      )}

      {/* Copies badge */}
      {counts.copies > 0 && (
        <Badge
          icon={<span className="text-[10px]">++</span>}
          count={counts.copies}
          label={counts.copies === 1 ? 'copy' : 'copies'}
          colorClass="bg-purple-500/20 text-purple-400 border-purple-500/30"
        />
      )}
    </div>
  );
}

interface BadgeProps {
  icon: React.ReactNode;
  count: number;
  label: string;
  colorClass: string;
}

function Badge({ icon, count, label, colorClass }: BadgeProps) {
  return (
    <div
      className={cn(
        'inline-flex items-center gap-1 px-2 py-0.5 rounded-full',
        'text-xs font-medium border',
        colorClass
      )}
    >
      {icon}
      <span>
        {count} {label}
      </span>
    </div>
  );
}
