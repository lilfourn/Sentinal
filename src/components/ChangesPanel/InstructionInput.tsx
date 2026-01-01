import { Loader2 } from 'lucide-react';
import { cn } from '../../lib/utils';

interface InstructionInputProps {
  instruction: string;
  onInstructionChange: (value: string) => void;
  onSubmit: () => void;
  isDisabled: boolean;
  folderName: string;
}

export function InstructionInput({
  instruction,
  onInstructionChange,
  onSubmit,
  isDisabled,
}: InstructionInputProps) {
  const isValid = instruction.trim().length > 0;

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter' && isValid && !isDisabled) {
      e.preventDefault();
      onSubmit();
    }
  };

  return (
    <div className="p-3">
      <div className="relative">
        <input
          type="text"
          value={instruction}
          onChange={(e) => onInstructionChange(e.target.value)}
          onKeyDown={handleKeyDown}
          disabled={isDisabled}
          placeholder="Describe how you want to organize..."
          className={cn(
            'w-full px-3 py-2.5 rounded-lg',
            'bg-white/[0.03] border border-white/10',
            'text-sm text-gray-200 placeholder:text-gray-600',
            'focus:outline-none focus:border-orange-500/50 focus:bg-orange-500/5',
            'disabled:opacity-50 disabled:cursor-not-allowed',
            'transition-all'
          )}
        />
        {isDisabled && (
          <div className="absolute right-3 top-1/2 -translate-y-1/2">
            <Loader2 size={14} className="animate-spin text-orange-500" />
          </div>
        )}
      </div>
    </div>
  );
}
