/**
 * Frontend cycle detection for immediate UX feedback.
 *
 * This provides fast, synchronous validation for drag-drop operations.
 * The backend performs canonical path validation as a safety net.
 */

/**
 * Normalize a path by removing trailing slashes.
 * This is a simple normalization - the backend does full canonicalization.
 */
function normalizePath(path: string): string {
  return path.endsWith('/') ? path.slice(0, -1) : path;
}

/**
 * Check if dropping `source` into `target` would create a cycle.
 *
 * A cycle occurs when:
 * 1. source === target (dropping into itself)
 * 2. target is a descendant of source (e.g., /a into /a/b/c)
 *
 * @param sourcePath - The path being dragged
 * @param targetPath - The destination directory path
 * @returns true if this would create a cycle
 */
export function wouldCreateCycle(
  sourcePath: string,
  targetPath: string
): boolean {
  const source = normalizePath(sourcePath);
  const target = normalizePath(targetPath);

  // Check 1: Same path (dropping into itself)
  if (source === target) {
    return true;
  }

  // Check 2: Target is descendant of source
  // e.g., source = /a/b, target = /a/b/c -> cycle
  if (target.startsWith(source + '/')) {
    return true;
  }

  return false;
}

/**
 * Check if any source would create a cycle when dropped into target.
 * Also checks if target is one of the sources (multi-drag edge case).
 *
 * @param sourcePaths - Array of paths being dragged
 * @param targetPath - The destination directory path
 * @returns true if any source would create a cycle
 */
export function wouldCreateCycleMulti(
  sourcePaths: string[],
  targetPath: string
): boolean {
  const target = normalizePath(targetPath);

  for (const source of sourcePaths) {
    const normalizedSource = normalizePath(source);

    // Target is one of the dragged items
    if (normalizedSource === target) {
      return true;
    }

    // Target is descendant of source
    if (target.startsWith(normalizedSource + '/')) {
      return true;
    }
  }

  return false;
}

/**
 * Determine the specific reason why a drop is invalid.
 *
 * @param sourcePaths - Array of paths being dragged
 * @param targetPath - The destination directory path
 * @returns The specific reason, or undefined if valid
 */
export function getCycleReason(
  sourcePaths: string[],
  targetPath: string
): 'cycle_self' | 'cycle_descendant' | 'target_selected' | undefined {
  const target = normalizePath(targetPath);

  for (const source of sourcePaths) {
    const normalizedSource = normalizePath(source);

    // Target is one of the dragged items
    if (normalizedSource === target) {
      return 'target_selected';
    }

    // Target is same as source
    if (normalizedSource === target) {
      return 'cycle_self';
    }

    // Target is descendant of source
    if (target.startsWith(normalizedSource + '/')) {
      return 'cycle_descendant';
    }
  }

  return undefined;
}

/**
 * Check if a path is a valid drop target for the given sources.
 *
 * @param targetPath - The potential drop target path
 * @param targetIsDirectory - Whether the target is a directory
 * @param sourcePaths - Array of paths being dragged
 * @returns Object with valid flag and optional reason
 */
export function isValidDropTarget(
  targetPath: string,
  targetIsDirectory: boolean,
  sourcePaths: string[]
): { valid: boolean; reason?: string } {
  // Must be a directory
  if (!targetIsDirectory) {
    return { valid: false, reason: 'not_directory' };
  }

  // Check for cycles
  const cycleReason = getCycleReason(sourcePaths, targetPath);
  if (cycleReason) {
    return { valid: false, reason: cycleReason };
  }

  return { valid: true };
}
