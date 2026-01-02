import { useState } from 'react';
import { X, AlertTriangle, ChevronDown, ChevronUp, Copy, Check } from 'lucide-react';
import { cn } from '../../lib/utils';
import type { ExecutionError } from '../../stores/organize-store';

interface ErrorDetailDialogProps {
  errors: ExecutionError[];
  onClose: () => void;
  className?: string;
}

export function ErrorDetailDialog({ errors, onClose, className }: ErrorDetailDialogProps) {
  const [expandedIndex, setExpandedIndex] = useState<number | null>(null);
  const [copied, setCopied] = useState(false);

  const copyAllErrors = () => {
    const text = errors.map((e, i) => `${i + 1}. ${e.message}`).join('\n');
    navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className={cn(
      'fixed inset-0 z-50 flex items-center justify-center bg-black/50',
      className
    )}>
      <div className="bg-gray-900 rounded-lg shadow-xl w-[480px] max-h-[80vh] flex flex-col border border-white/10">
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-3 border-b border-white/10">
          <div className="flex items-center gap-2">
            <AlertTriangle size={16} className="text-red-400" />
            <span className="text-sm font-medium text-gray-100">
              {errors.length} Operation{errors.length !== 1 ? 's' : ''} Failed
            </span>
          </div>
          <div className="flex items-center gap-2">
            <button
              onClick={copyAllErrors}
              className="p-1.5 rounded hover:bg-white/10 text-gray-400 hover:text-gray-200 transition-colors"
              title="Copy all errors"
            >
              {copied ? <Check size={14} /> : <Copy size={14} />}
            </button>
            <button
              onClick={onClose}
              className="p-1.5 rounded hover:bg-white/10 text-gray-400 hover:text-gray-200 transition-colors"
            >
              <X size={14} />
            </button>
          </div>
        </div>

        {/* Error list */}
        <div className="flex-1 overflow-y-auto p-2 space-y-1">
          {errors.map((error, index) => (
            <div
              key={index}
              className="rounded-lg bg-red-500/10 border border-red-500/20 overflow-hidden"
            >
              <button
                onClick={() => setExpandedIndex(expandedIndex === index ? null : index)}
                className="w-full flex items-start gap-2 p-2 text-left hover:bg-red-500/5 transition-colors"
              >
                <span className="text-[10px] text-red-400 font-mono bg-red-500/20 px-1.5 py-0.5 rounded shrink-0">
                  {index + 1}
                </span>
                <span className="flex-1 text-xs text-red-300 break-words leading-relaxed">
                  {error.message}
                </span>
                {(error.source || error.destination) && (
                  <span className="text-gray-500 shrink-0">
                    {expandedIndex === index ? <ChevronUp size={12} /> : <ChevronDown size={12} />}
                  </span>
                )}
              </button>

              {expandedIndex === index && (error.source || error.destination || error.operationType) && (
                <div className="px-3 pb-2 text-[10px] text-gray-500 space-y-1 border-t border-red-500/10 pt-2 ml-6">
                  {error.operationType && (
                    <div>Type: <span className="text-gray-400">{error.operationType}</span></div>
                  )}
                  {error.source && (
                    <div>Source: <span className="text-gray-400 break-all">{error.source}</span></div>
                  )}
                  {error.destination && (
                    <div>Dest: <span className="text-gray-400 break-all">{error.destination}</span></div>
                  )}
                </div>
              )}
            </div>
          ))}
        </div>

        {/* Footer */}
        <div className="px-4 py-3 border-t border-white/10 bg-gray-900/50">
          <button
            onClick={onClose}
            className="w-full px-3 py-1.5 text-xs font-medium text-gray-300 bg-gray-800 hover:bg-gray-700 rounded transition-colors"
          >
            Close
          </button>
        </div>
      </div>
    </div>
  );
}
