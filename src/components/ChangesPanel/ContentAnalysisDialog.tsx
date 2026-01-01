import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import {
  FileSearch,
  FileText,
  Database,
  Loader2,
  Sparkles,
  AlertCircle,
  CheckCircle,
  DollarSign,
  Zap,
} from 'lucide-react';
import { cn } from '../../lib/utils';

interface ScanResult {
  totalFiles: number;
  analyzableFiles: number;
  textFiles: number;
  otherFiles: number;
  cachedFiles: number;
  needsAnalysis: number;
  totalSizeBytes: number;
  estimatedCostCents: number;
}

interface AnalysisProgress {
  phase: string;
  current: number;
  total: number;
  currentFile: string | null;
  message: string;
}

interface ContentAnalysisDialogProps {
  folderPath: string;
  onComplete: () => void;
  onSkip: () => void;
}

export function ContentAnalysisDialog({
  folderPath,
  onComplete,
  onSkip,
}: ContentAnalysisDialogProps) {
  const [status, setStatus] = useState<'initializing' | 'scanning' | 'ready' | 'analyzing' | 'complete' | 'error'>('initializing');
  const [scanResult, setScanResult] = useState<ScanResult | null>(null);
  const [progress, setProgress] = useState<AnalysisProgress | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [grokAvailable, setGrokAvailable] = useState(false);

  // Initialize Grok and scan folder
  useEffect(() => {
    let mounted = true;

    async function init() {
      try {
        // Check if Grok API key is available
        const hasKey = await invoke<boolean>('grok_check_api_key');

        if (!mounted) return;

        if (!hasKey) {
          // No API key, skip content analysis
          setGrokAvailable(false);
          setStatus('ready');
          return;
        }

        setGrokAvailable(true);

        // Initialize Grok (will use env key)
        await invoke('grok_init', { apiKey: null });

        if (!mounted) return;
        setStatus('scanning');

        // Scan folder
        const result = await invoke<ScanResult>('grok_scan_folder', { path: folderPath });

        if (!mounted) return;
        setScanResult(result);
        setStatus('ready');
      } catch (e) {
        if (!mounted) return;
        console.error('[ContentAnalysis] Init failed:', e);
        setError(String(e));
        setStatus('error');
      }
    }

    init();
    return () => { mounted = false; };
  }, [folderPath]);

  // Listen for analysis progress
  useEffect(() => {
    let unlisten: UnlistenFn | null = null;

    async function setupListener() {
      unlisten = await listen<AnalysisProgress>('grok:progress', (event) => {
        setProgress(event.payload);

        if (event.payload.phase === 'complete') {
          setStatus('complete');
          // Auto-continue after brief delay
          setTimeout(onComplete, 500);
        } else if (event.payload.phase === 'failed') {
          setStatus('error');
        }
      });
    }

    setupListener();
    return () => { unlisten?.(); };
  }, [onComplete]);

  // Handle analyze button click
  const handleAnalyze = async () => {
    setStatus('analyzing');

    try {
      // The organize command will handle everything
      onComplete();
    } catch (e) {
      setError(String(e));
      setStatus('error');
    }
  };

  // Format cost
  const formatCost = (cents: number) => {
    if (cents === 0) return 'Free';
    if (cents < 100) return `$0.${cents.toString().padStart(2, '0')}`;
    return `$${(cents / 100).toFixed(2)}`;
  };

  // If no Grok API key, show minimal UI and auto-skip
  if (!grokAvailable && status === 'ready') {
    return (
      <div className="p-3 rounded-lg bg-white/[0.02] border border-white/5">
        <div className="flex items-center gap-2 text-gray-400">
          <FileText size={14} />
          <span className="text-xs">Content analysis unavailable (no API key)</span>
        </div>
        <button
          onClick={onSkip}
          className="mt-2 w-full px-3 py-1.5 text-xs bg-white/5 hover:bg-white/10 rounded-md transition-colors"
        >
          Continue with filename-based organization
        </button>
      </div>
    );
  }

  // Loading state
  if (status === 'initializing' || status === 'scanning') {
    return (
      <div className="p-4 rounded-lg bg-white/[0.02] border border-white/5">
        <div className="flex items-center gap-3">
          <Loader2 size={16} className="text-blue-400 animate-spin" />
          <div>
            <p className="text-xs font-medium text-gray-300">
              {status === 'initializing' ? 'Initializing...' : 'Scanning for documents...'}
            </p>
            <p className="text-[10px] text-gray-500 mt-0.5">
              Looking for PDFs and images to analyze
            </p>
          </div>
        </div>
      </div>
    );
  }

  // Error state
  if (status === 'error') {
    return (
      <div className="p-3 rounded-lg bg-red-500/10 border border-red-500/20">
        <div className="flex items-start gap-2">
          <AlertCircle size={14} className="text-red-400 mt-0.5 shrink-0" />
          <div className="min-w-0">
            <p className="text-xs font-medium text-red-400">Analysis failed</p>
            <p className="text-[10px] text-red-400/70 mt-1 break-words">
              {error || 'Unknown error'}
            </p>
          </div>
        </div>
        <button
          onClick={onSkip}
          className="mt-3 w-full px-3 py-1.5 text-xs bg-white/5 hover:bg-white/10 rounded-md transition-colors text-gray-300"
        >
          Continue without content analysis
        </button>
      </div>
    );
  }

  // Analyzing state
  if (status === 'analyzing' && progress) {
    const percent = progress.total > 0 ? (progress.current / progress.total) * 100 : 0;

    return (
      <div className="p-4 rounded-lg bg-blue-500/5 border border-blue-500/20">
        <div className="flex items-center gap-2 mb-3">
          <Sparkles size={14} className="text-blue-400" />
          <span className="text-xs font-medium text-blue-300">
            Analyzing documents with AI
          </span>
        </div>

        <div className="space-y-2">
          <div className="flex items-center gap-2">
            <div className="flex-1 h-1.5 bg-white/10 rounded-full overflow-hidden">
              <div
                className="h-full bg-blue-500 rounded-full transition-all duration-300"
                style={{ width: `${percent}%` }}
              />
            </div>
            <span className="text-[10px] text-gray-400 tabular-nums w-12 text-right">
              {progress.current}/{progress.total}
            </span>
          </div>

          <p className="text-[10px] text-gray-500 truncate">
            {progress.message}
          </p>

          {progress.currentFile && (
            <p className="text-[10px] text-gray-600 truncate font-mono">
              {progress.currentFile.split('/').pop()}
            </p>
          )}
        </div>
      </div>
    );
  }

  // Ready state - show scan results
  if (status === 'ready' && scanResult) {
    const hasDocsToAnalyze = scanResult.needsAnalysis > 0;

    return (
      <div className="p-3 rounded-lg bg-white/[0.02] border border-white/5">
        {/* Header */}
        <div className="flex items-center gap-2 mb-3">
          <div className="w-6 h-6 rounded-lg bg-gradient-to-br from-blue-500 to-purple-500 flex items-center justify-center">
            <FileSearch size={12} className="text-white" />
          </div>
          <div>
            <p className="text-xs font-medium text-gray-200">Smart Content Analysis</p>
            <p className="text-[10px] text-gray-500">
              AI-powered document understanding
            </p>
          </div>
        </div>

        {/* Stats grid */}
        <div className="grid grid-cols-2 gap-2 mb-3">
          <StatCard
            icon={<FileText size={12} />}
            label="Documents found"
            value={scanResult.analyzableFiles}
            color="blue"
          />
          <StatCard
            icon={<Database size={12} />}
            label="Already cached"
            value={scanResult.cachedFiles}
            color="green"
          />
          <StatCard
            icon={<Zap size={12} />}
            label="Need analysis"
            value={scanResult.needsAnalysis}
            color={scanResult.needsAnalysis > 0 ? 'orange' : 'gray'}
          />
          <StatCard
            icon={<DollarSign size={12} />}
            label="Est. cost"
            value={formatCost(scanResult.estimatedCostCents)}
            color="purple"
            isText
          />
        </div>

        {/* Info text */}
        <p className="text-[10px] text-gray-500 mb-3">
          {hasDocsToAnalyze ? (
            <>
              Grok AI will analyze {scanResult.needsAnalysis} PDFs/images to understand
              their content for smarter organization.
            </>
          ) : (
            <>
              All documents are already analyzed and cached.
              Organization will use cached content summaries.
            </>
          )}
        </p>

        {/* Actions */}
        <div className="flex gap-2">
          <button
            onClick={onSkip}
            className="flex-1 px-3 py-2 text-xs bg-white/5 hover:bg-white/10 rounded-md transition-colors text-gray-400"
          >
            Skip analysis
          </button>
          <button
            onClick={handleAnalyze}
            className={cn(
              "flex-1 px-3 py-2 text-xs rounded-md transition-colors flex items-center justify-center gap-1.5",
              hasDocsToAnalyze
                ? "bg-blue-500/20 hover:bg-blue-500/30 text-blue-300"
                : "bg-green-500/20 hover:bg-green-500/30 text-green-300"
            )}
          >
            {hasDocsToAnalyze ? (
              <>
                <Sparkles size={12} />
                Analyze & Organize
              </>
            ) : (
              <>
                <CheckCircle size={12} />
                Continue
              </>
            )}
          </button>
        </div>
      </div>
    );
  }

  // Complete state
  if (status === 'complete') {
    return (
      <div className="p-3 rounded-lg bg-green-500/10 border border-green-500/20">
        <div className="flex items-center gap-2">
          <CheckCircle size={14} className="text-green-400" />
          <span className="text-xs font-medium text-green-300">
            Content analysis complete
          </span>
        </div>
      </div>
    );
  }

  return null;
}

// Stat card component
function StatCard({
  icon,
  label,
  value,
  color,
  isText = false,
}: {
  icon: React.ReactNode;
  label: string;
  value: number | string;
  color: 'blue' | 'green' | 'orange' | 'purple' | 'gray';
  isText?: boolean;
}) {
  const colorClasses = {
    blue: 'text-blue-400 bg-blue-500/10',
    green: 'text-green-400 bg-green-500/10',
    orange: 'text-orange-400 bg-orange-500/10',
    purple: 'text-purple-400 bg-purple-500/10',
    gray: 'text-gray-400 bg-gray-500/10',
  };

  return (
    <div className="p-2 rounded-lg bg-white/[0.02]">
      <div className="flex items-center gap-1.5 mb-1">
        <span className={cn('opacity-70', colorClasses[color].split(' ')[0])}>
          {icon}
        </span>
        <span className="text-[10px] text-gray-500">{label}</span>
      </div>
      <p className={cn(
        'text-sm font-medium',
        colorClasses[color].split(' ')[0]
      )}>
        {isText ? value : value.toLocaleString()}
      </p>
    </div>
  );
}
