import { useState } from 'react';
import { FolderTree, Calendar, Tags, Pencil, ArrowRight, Loader2 } from 'lucide-react';
import { cn } from '../../lib/utils';

interface OrganizeMethod {
  id: string;
  label: string;
  description: string;
  icon: React.ReactNode;
  instruction: string;
}

const ORGANIZE_METHODS: OrganizeMethod[] = [
  {
    id: 'by-type',
    label: 'By file type',
    description: 'Documents, Images, Videos, etc.',
    icon: <FolderTree size={16} />,
    instruction: 'Organize files by their type into folders: Documents, Images, Videos, Audio, Archives, and Other. Keep the folder structure clean and logical.',
  },
  {
    id: 'by-date',
    label: 'By date',
    description: 'Year and month folders',
    icon: <Calendar size={16} />,
    instruction: 'Organize files by their creation or modification date into Year/Month folders (e.g., 2024/January, 2024/February). Group files chronologically.',
  },
  {
    id: 'by-topic',
    label: 'By topic',
    description: 'AI groups by content similarity',
    icon: <Tags size={16} />,
    instruction: 'Analyze file names and content to group files by topic or project. Create descriptive folder names based on what the files are about. Use semantic understanding to find related files.',
  },
];

interface OrganizeMethodSelectorProps {
  onSelect: (instruction: string) => void;
  isDisabled: boolean;
  folderName: string;
}

export function OrganizeMethodSelector({
  onSelect,
  isDisabled,
  folderName,
}: OrganizeMethodSelectorProps) {
  const [selectedMethod, setSelectedMethod] = useState<string | null>(null);
  const [customInstruction, setCustomInstruction] = useState('');
  const [showCustomInput, setShowCustomInput] = useState(false);

  const handleMethodClick = (method: OrganizeMethod) => {
    if (isDisabled) return;
    setSelectedMethod(method.id);
    setShowCustomInput(false);
    // Auto-submit after brief delay for visual feedback
    setTimeout(() => onSelect(method.instruction), 150);
  };

  const handleCustomClick = () => {
    if (isDisabled) return;
    setSelectedMethod('custom');
    setShowCustomInput(true);
  };

  const handleCustomSubmit = () => {
    if (customInstruction.trim() && !isDisabled) {
      onSelect(customInstruction.trim());
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && customInstruction.trim() && !isDisabled) {
      e.preventDefault();
      handleCustomSubmit();
    }
  };

  return (
    <div className="p-3">
      <p className="text-xs text-gray-400 mb-3">
        How would you like to organize <span className="text-gray-200 font-medium">{folderName}</span>?
      </p>

      <div className="space-y-1.5">
        {ORGANIZE_METHODS.map((method) => (
          <button
            key={method.id}
            onClick={() => handleMethodClick(method)}
            disabled={isDisabled}
            className={cn(
              'w-full flex items-center gap-3 p-2.5 rounded-lg text-left transition-all',
              'border border-transparent',
              selectedMethod === method.id
                ? 'bg-orange-500/15 border-orange-500/30'
                : 'bg-white/[0.03] hover:bg-white/[0.06] hover:border-white/10',
              isDisabled && 'opacity-50 cursor-not-allowed'
            )}
          >
            <div
              className={cn(
                'w-8 h-8 rounded-lg flex items-center justify-center shrink-0',
                selectedMethod === method.id
                  ? 'bg-orange-500/20 text-orange-400'
                  : 'bg-white/5 text-gray-400'
              )}
            >
              {selectedMethod === method.id && isDisabled ? (
                <Loader2 size={16} className="animate-spin" />
              ) : (
                method.icon
              )}
            </div>
            <div className="flex-1 min-w-0">
              <p
                className={cn(
                  'text-sm font-medium',
                  selectedMethod === method.id ? 'text-orange-300' : 'text-gray-200'
                )}
              >
                {method.label}
              </p>
              <p className="text-[11px] text-gray-500 truncate">{method.description}</p>
            </div>
            <ArrowRight
              size={14}
              className={cn(
                'text-gray-600 transition-transform',
                selectedMethod === method.id && 'text-orange-400 translate-x-0.5'
              )}
            />
          </button>
        ))}

        {/* Custom option */}
        <button
          onClick={handleCustomClick}
          disabled={isDisabled}
          className={cn(
            'w-full flex items-center gap-3 p-2.5 rounded-lg text-left transition-all',
            'border border-transparent',
            selectedMethod === 'custom'
              ? 'bg-orange-500/15 border-orange-500/30'
              : 'bg-white/[0.03] hover:bg-white/[0.06] hover:border-white/10',
            isDisabled && 'opacity-50 cursor-not-allowed'
          )}
        >
          <div
            className={cn(
              'w-8 h-8 rounded-lg flex items-center justify-center shrink-0',
              selectedMethod === 'custom'
                ? 'bg-orange-500/20 text-orange-400'
                : 'bg-white/5 text-gray-400'
            )}
          >
            <Pencil size={16} />
          </div>
          <div className="flex-1 min-w-0">
            <p
              className={cn(
                'text-sm font-medium',
                selectedMethod === 'custom' ? 'text-orange-300' : 'text-gray-200'
              )}
            >
              Custom instructions
            </p>
            <p className="text-[11px] text-gray-500">Describe your own organization</p>
          </div>
          <ArrowRight
            size={14}
            className={cn(
              'text-gray-600 transition-transform',
              selectedMethod === 'custom' && 'text-orange-400 translate-x-0.5'
            )}
          />
        </button>

        {/* Custom input field */}
        {showCustomInput && (
          <div className="mt-2 pl-11">
            <div className="relative">
              <input
                type="text"
                value={customInstruction}
                onChange={(e) => setCustomInstruction(e.target.value)}
                onKeyDown={handleKeyDown}
                disabled={isDisabled}
                placeholder="e.g., Group by client name..."
                autoFocus
                className={cn(
                  'w-full px-3 py-2 rounded-lg',
                  'bg-white/[0.03] border border-white/10',
                  'text-sm text-gray-200 placeholder:text-gray-600',
                  'focus:outline-none focus:border-orange-500/50 focus:bg-orange-500/5',
                  'disabled:opacity-50 disabled:cursor-not-allowed',
                  'transition-all'
                )}
              />
              {customInstruction.trim() && (
                <button
                  onClick={handleCustomSubmit}
                  disabled={isDisabled}
                  className={cn(
                    'absolute right-1.5 top-1/2 -translate-y-1/2',
                    'px-2 py-1 rounded text-xs font-medium',
                    'bg-orange-500/20 text-orange-300 hover:bg-orange-500/30',
                    'disabled:opacity-50 transition-colors'
                  )}
                >
                  {isDisabled ? <Loader2 size={12} className="animate-spin" /> : 'Go'}
                </button>
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
