import type { FileEntry } from '../types/file';

/**
 * Creates a stacked-card ghost element for native HTML5 drag image.
 * This element is temporarily appended to the DOM for the browser to snapshot,
 * then removed after the drag starts.
 *
 * Features:
 * - Stacked card visual (max 3 layers with offset)
 * - Red badge counter for multi-item drag
 * - Mac Finder-like styling
 *
 * Usage:
 * ```ts
 * const ghost = createDragGhost(items);
 * document.body.appendChild(ghost);
 * e.dataTransfer.setDragImage(ghost, 16, 16);
 * requestAnimationFrame(() => ghost.remove());
 * ```
 */
export function createDragGhost(items: FileEntry[]): HTMLElement {
  const container = document.createElement('div');

  // Position off-screen but visible in DOM for the browser snapshot
  Object.assign(container.style, {
    position: 'absolute',
    top: '-9999px',
    left: '-9999px',
    zIndex: '9999',
    pointerEvents: 'none',
  });

  // Limit stack visual to 3 items
  const stackLimit = Math.min(items.length, 3);

  for (let i = stackLimit - 1; i >= 0; i--) {
    const layer = document.createElement('div');
    const offset = i * 2;
    const zIndex = 10 - i;

    // Mac-style Card Visuals
    Object.assign(layer.style, {
      position: 'absolute',
      top: `${offset}px`,
      left: `${offset}px`,
      zIndex: String(zIndex),
      width: '220px',
      padding: '8px 12px',
      background: 'rgba(255, 255, 255, 0.95)',
      backdropFilter: 'blur(8px)',
      WebkitBackdropFilter: 'blur(8px)',
      borderRadius: '8px',
      border: '1px solid rgba(0, 0, 0, 0.1)',
      boxShadow:
        '0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06)',
      display: 'flex',
      alignItems: 'center',
      gap: '8px',
    });

    // Content (Icon + Name) - Only on top card (i === 0)
    if (i === 0) {
      const firstItem = items[0];

      // Icon span
      const iconSpan = document.createElement('span');
      Object.assign(iconSpan.style, {
        fontSize: '16px',
        flexShrink: '0',
      });
      iconSpan.textContent = firstItem.isDirectory ? '\ud83d\udcc1' : '\ud83d\udcc4';

      // Label span
      const labelSpan = document.createElement('span');
      Object.assign(labelSpan.style, {
        fontSize: '13px',
        fontWeight: '500',
        color: '#374151',
        whiteSpace: 'nowrap',
        overflow: 'hidden',
        textOverflow: 'ellipsis',
        flex: '1',
      });
      labelSpan.textContent =
        items.length > 1 ? `${items.length} items` : firstItem.name;

      layer.appendChild(iconSpan);
      layer.appendChild(labelSpan);
    }

    container.appendChild(layer);
  }

  // Red Badge Counter for multi-item
  if (items.length > 1) {
    const badge = document.createElement('div');
    Object.assign(badge.style, {
      position: 'absolute',
      top: '-8px',
      right: '-8px',
      zIndex: '20',
      background: '#ef4444',
      color: 'white',
      fontWeight: 'bold',
      fontSize: '11px',
      padding: '2px 6px',
      borderRadius: '10px',
      border: '2px solid white',
      boxShadow: '0 1px 3px rgba(0, 0, 0, 0.2)',
      minWidth: '20px',
      textAlign: 'center',
    });
    badge.textContent = String(items.length);
    container.appendChild(badge);
  }

  return container;
}
