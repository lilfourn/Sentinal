import type { SelectionRect } from '../../hooks/useMarqueeSelection';

interface SelectionOverlayProps {
  rect: SelectionRect | null;
}

export function SelectionOverlay({ rect }: SelectionOverlayProps) {
  if (!rect || rect.width < 2 || rect.height < 2) {
    return null;
  }

  return (
    <div
      className="absolute pointer-events-none z-10 bg-orange-500/10 border border-orange-500/50 rounded-sm"
      style={{
        left: rect.x,
        top: rect.y,
        width: rect.width,
        height: rect.height,
      }}
    />
  );
}
