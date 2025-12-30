import type { FileEntry } from './file';

/**
 * Ghost state represents the visual state of a file during simulation.
 */
export type GhostState =
  | 'normal'      // Default rendering - file exists as-is
  | 'source'      // Being moved away (semi-transparent with strikethrough)
  | 'destination' // Ghost at destination (green outline, not yet real)
  | 'creating'    // New folder being created (pulsing green)
  | 'deleting'    // Being deleted (red, fading)
  | 'completed';  // Transition animation after operation completes

/**
 * Extended FileEntry with ghost visualization properties.
 */
export interface GhostFileEntry extends FileEntry {
  /** Current ghost visualization state */
  ghostState: GhostState;
  /** ID of the operation this ghost is associated with */
  operationId?: string;
  /** For source/destination pairs, the linked path (source links to dest, dest links to source) */
  linkedPath?: string;
  /** Timestamp when this entry became a ghost (for animation timing) */
  ghostSince?: number;
  /** Whether this entry is virtual (doesn't exist on disk yet) */
  isVirtual?: boolean;
}

/**
 * Map of file paths to their ghost state and metadata.
 */
export type GhostStateMap = Map<string, {
  /** Current ghost visualization state */
  state: GhostState;
  /** ID of the operation this ghost is associated with */
  operationId: string;
  /** For source/destination pairs, the linked path */
  linkedPath?: string;
  /** Whether this entry is virtual (doesn't exist on disk yet) */
  isVirtual?: boolean;
}>;

/**
 * Operation status for tracking execution progress.
 */
export type OperationStatus =
  | 'pending'     // Not yet executed
  | 'executing'   // Currently being executed
  | 'completed'   // Successfully completed
  | 'failed'      // Failed to execute
  | 'skipped';    // Skipped (e.g., due to dependency failure)

/**
 * WAL (Write-Ahead Log) entry for crash recovery.
 */
export interface WalEntry {
  /** Unique identifier for this operation */
  operationId: string;
  /** Type of operation */
  type: 'create_folder' | 'move' | 'rename' | 'trash' | 'copy';
  /** Source path (for move/copy/rename) */
  source?: string;
  /** Destination path (for move/copy) */
  destination?: string;
  /** Path (for create_folder/trash) */
  path?: string;
  /** New name (for rename) */
  newName?: string;
  /** Timestamp when operation was logged */
  timestamp: number;
  /** Current status */
  status: OperationStatus;
  /** Error message if failed */
  error?: string;
}

/**
 * Information about a recoverable WAL job.
 * Returned by wal_check_recovery command.
 */
export interface WalRecoveryInfo {
  /** Job ID of the interrupted job */
  jobId: string;
  /** Target folder that was being organized */
  targetFolder: string;
  /** Number of operations completed before interruption */
  completedCount: number;
  /** Number of operations still pending */
  pendingCount: number;
  /** Number of operations that failed */
  failedCount: number;
  /** When the job was started (ISO 8601 string) */
  startedAt: string;
  /** Descriptions of pending operations */
  pendingOperations: string[];
}

/**
 * Result of a WAL recovery operation.
 * Returned by wal_resume_job and wal_rollback_job commands.
 */
export interface WalRecoveryResult {
  /** Whether recovery was successful */
  success: boolean;
  /** Number of operations completed during recovery */
  completedCount: number;
  /** Number of operations that failed during recovery */
  failedCount: number;
  /** Error messages from failed operations */
  errors: string[];
}
