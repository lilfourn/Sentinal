import { ArrowRight, Plus, Trash2 } from 'lucide-react';
import { cn } from '../../lib/utils';
import type { GhostState } from '../../types/ghost';
import './GhostAnimations.css';

interface GhostOverlayProps {
  /** Current ghost visualization state */
  ghostState: GhostState;
  /** For source/destination pairs, the linked path */
  linkedPath?: string;
  /** Whether to show the link indicator */
  showLinkIndicator?: boolean;
}

/**
 * Overlay component that shows ghost state indicators on file items.
 *
 * Shows:
 * - Green dot for destination (file will appear here)
 * - Arrow for source -> destination link
 * - Plus icon for creating
 * - Trash icon for deleting
 */
export function GhostOverlay({
  ghostState,
  linkedPath,
  showLinkIndicator = false,
}: GhostOverlayProps) {
  if (ghostState === 'normal') return null;

  return (
    <div
      className={cn(
        'absolute inset-0 pointer-events-none',
        'flex items-center justify-end pr-2',
        ghostState === 'destination' && 'ghost-destination-overlay',
        ghostState === 'creating' && 'ghost-creating-overlay',
        ghostState === 'deleting' && 'ghost-deleting-overlay',
        ghostState === 'source' && 'ghost-source-overlay',
        ghostState === 'completed' && 'ghost-complete-animation'
      )}
    >
      {/* State indicator badges */}
      <div className="flex items-center gap-1">
        {ghostState === 'destination' && (
          <div className="flex items-center gap-1 px-1.5 py-0.5 rounded-full bg-green-500/20 border border-green-500/30">
            <div className="w-1.5 h-1.5 rounded-full bg-green-400 animate-pulse" />
            <span className="text-[10px] font-medium text-green-400">NEW</span>
          </div>
        )}

        {ghostState === 'creating' && (
          <div className="flex items-center gap-1 px-1.5 py-0.5 rounded-full bg-green-500/20 border border-green-500/30">
            <Plus size={10} className="text-green-400" />
            <span className="text-[10px] font-medium text-green-400">CREATE</span>
          </div>
        )}

        {ghostState === 'deleting' && (
          <div className="flex items-center gap-1 px-1.5 py-0.5 rounded-full bg-red-500/20 border border-red-500/30">
            <Trash2 size={10} className="text-red-400" />
            <span className="text-[10px] font-medium text-red-400">DELETE</span>
          </div>
        )}

        {ghostState === 'source' && linkedPath && (
          <div className="flex items-center gap-1 px-1.5 py-0.5 rounded-full bg-yellow-500/20 border border-yellow-500/30">
            <ArrowRight size={10} className="text-yellow-400" />
            <span className="text-[10px] font-medium text-yellow-400">MOVING</span>
          </div>
        )}
      </div>

      {/* Link indicator tooltip */}
      {showLinkIndicator && linkedPath && (
        <div
          className={cn(
            'absolute right-full mr-2 px-2 py-1 rounded',
            'text-[10px] font-mono whitespace-nowrap',
            'bg-gray-800 text-gray-300 border border-gray-700',
            'opacity-0 group-hover:opacity-100 transition-opacity'
          )}
        >
          {ghostState === 'source' ? 'To: ' : 'From: '}
          {linkedPath.split('/').pop()}
        </div>
      )}
    </div>
  );
}

/**
 * Returns CSS classes for applying ghost styling to file items.
 */
export function getGhostClasses(ghostState: GhostState): string {
  switch (ghostState) {
    case 'source':
      return 'opacity-50 ghost-source';
    case 'destination':
      return 'ghost-destination';
    case 'creating':
      return 'ghost-creating';
    case 'deleting':
      return 'opacity-30 ghost-deleting';
    case 'completed':
      return 'ghost-complete-animation';
    default:
      return '';
  }
}

/**
 * Returns whether a ghost state represents a virtual (non-existent) file.
 */
export function isVirtualGhost(ghostState: GhostState): boolean {
  return ghostState === 'destination' || ghostState === 'creating';
}
