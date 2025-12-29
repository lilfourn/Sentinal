import { useRef, useEffect, useState, useCallback } from 'react';
import { cn } from '../../lib/utils';

interface InlineNameEditorProps {
  /** Initial value to display in the input */
  initialValue: string;
  /** Called when editing is confirmed (Enter pressed or blur) */
  onConfirm: (newValue: string) => void;
  /** Called when editing is cancelled (Escape pressed) */
  onCancel: () => void;
  /** Whether to select just the name part (without extension) on focus */
  selectNameOnly?: boolean;
  /** Additional class names */
  className?: string;
}

export function InlineNameEditor({
  initialValue,
  onConfirm,
  onCancel,
  selectNameOnly = true,
  className,
}: InlineNameEditorProps) {
  const inputRef = useRef<HTMLInputElement>(null);
  const [value, setValue] = useState(initialValue);
  const [hasSubmitted, setHasSubmitted] = useState(false);

  // Focus and select on mount
  useEffect(() => {
    const input = inputRef.current;
    if (!input) return;

    input.focus();

    if (selectNameOnly && initialValue.includes('.')) {
      // Select only the name part (before the last dot)
      const lastDotIndex = initialValue.lastIndexOf('.');
      input.setSelectionRange(0, lastDotIndex);
    } else {
      // Select all
      input.select();
    }
  }, [initialValue, selectNameOnly]);

  const handleConfirm = useCallback(() => {
    if (hasSubmitted) return;

    const trimmedValue = value.trim();
    if (trimmedValue && trimmedValue !== initialValue) {
      setHasSubmitted(true);
      onConfirm(trimmedValue);
    } else if (!trimmedValue || trimmedValue === initialValue) {
      onCancel();
    }
  }, [value, initialValue, onConfirm, onCancel, hasSubmitted]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLInputElement>) => {
      if (e.key === 'Enter') {
        e.preventDefault();
        e.stopPropagation();
        handleConfirm();
      } else if (e.key === 'Escape') {
        e.preventDefault();
        e.stopPropagation();
        onCancel();
      }
    },
    [handleConfirm, onCancel]
  );

  const handleBlur = useCallback(() => {
    // Small delay to allow click events to fire first
    setTimeout(() => {
      if (!hasSubmitted) {
        handleConfirm();
      }
    }, 100);
  }, [handleConfirm, hasSubmitted]);

  // Prevent clicks from bubbling to parent (which might clear selection)
  const handleClick = useCallback((e: React.MouseEvent) => {
    e.stopPropagation();
  }, []);

  return (
    <input
      ref={inputRef}
      type="text"
      value={value}
      onChange={(e) => setValue(e.target.value)}
      onKeyDown={handleKeyDown}
      onBlur={handleBlur}
      onClick={handleClick}
      onMouseDown={(e) => e.stopPropagation()}
      className={cn(
        'bg-white dark:bg-gray-800 border border-blue-500 dark:border-blue-400',
        'rounded px-1.5 py-0.5 text-sm outline-none',
        'text-gray-900 dark:text-gray-100',
        'focus:ring-2 focus:ring-blue-500/30',
        className
      )}
      style={{ minWidth: '100px' }}
    />
  );
}
