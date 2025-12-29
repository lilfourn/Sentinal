import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { readFile, readTextFile } from '@tauri-apps/plugin-fs';
import { X, File, Loader2 } from 'lucide-react';
import { formatFileSize, formatDate, getFileType } from '../../lib/utils';
import { FolderIcon } from '../icons/FolderIcon';
import { useNavigationStore } from '../../stores/navigation-store';
import type { FileMetadata } from '../../types/file';

export function QuickLook() {
  const { quickLookActive, quickLookPath, closeQuickLook, toggleQuickLook } =
    useNavigationStore();
  const [metadata, setMetadata] = useState<FileMetadata | null>(null);
  const [content, setContent] = useState<string | null>(null);
  const [imageUrl, setImageUrl] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  // Keyboard shortcut (Spacebar)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.code === 'Space' && !isInputFocused()) {
        e.preventDefault();
        toggleQuickLook();
      }
      if (e.key === 'Escape' && quickLookActive) {
        closeQuickLook();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [quickLookActive, toggleQuickLook, closeQuickLook]);

  // Load file metadata and preview content
  useEffect(() => {
    if (!quickLookPath || !quickLookActive) {
      setMetadata(null);
      setContent(null);
      setImageUrl(null);
      return;
    }

    let cancelled = false;

    async function load() {
      setLoading(true);
      try {
        // Get metadata
        const meta = await invoke<FileMetadata>('get_file_metadata', {
          path: quickLookPath,
        });
        if (cancelled) return;
        setMetadata(meta);

        // Load preview content based on type
        if (meta.isFile) {
          const fileType = getFileType(meta.extension, meta.mimeType);

          if (fileType === 'image') {
            // Load image as data URL
            const bytes = await readFile(quickLookPath!);
            const blob = new Blob([bytes]);
            const url = URL.createObjectURL(blob);
            if (cancelled) {
              URL.revokeObjectURL(url);
              return;
            }
            setImageUrl(url);
            setContent(null);
          } else if (['text', 'code', 'config'].includes(fileType)) {
            // Load text content (first 10KB)
            try {
              const text = await readTextFile(quickLookPath!);
              if (cancelled) return;
              setContent(text.slice(0, 10000));
              setImageUrl(null);
            } catch {
              setContent(null);
            }
          } else {
            setContent(null);
            setImageUrl(null);
          }
        }
      } catch (error) {
        console.error('Failed to load preview:', error);
      } finally {
        if (!cancelled) setLoading(false);
      }
    }

    load();

    return () => {
      cancelled = true;
      if (imageUrl) {
        URL.revokeObjectURL(imageUrl);
      }
    };
  }, [quickLookPath, quickLookActive]);

  if (!quickLookActive || !quickLookPath) {
    return null;
  }

  return (
    <div className="glass-preview w-80 flex-shrink-0 h-full overflow-hidden flex flex-col">
      {/* Header */}
      <div className="flex items-center justify-between p-3 border-b border-gray-200 dark:border-gray-700">
        <h3 className="text-sm font-medium truncate flex-1 mr-2">
          {metadata?.name || 'Preview'}
        </h3>
        <button
          onClick={closeQuickLook}
          className="p-1 rounded hover:bg-gray-200 dark:hover:bg-gray-700 text-gray-500"
        >
          <X size={16} />
        </button>
      </div>

      {/* Preview content */}
      <div className="flex-1 overflow-auto p-4">
        {loading ? (
          <div className="flex items-center justify-center h-32">
            <Loader2 className="animate-spin text-gray-400" size={24} />
          </div>
        ) : (
          <>
            {/* Icon */}
            <div className="flex justify-center mb-4">
              {imageUrl ? (
                <img
                  src={imageUrl}
                  alt={metadata?.name}
                  className="max-w-full max-h-48 rounded shadow-lg object-contain"
                />
              ) : metadata?.isDirectory ? (
                <FolderIcon size={64} />
              ) : (
                <File size={64} className="text-gray-400" />
              )}
            </div>

            {/* Text preview */}
            {content && (
              <pre className="text-xs bg-gray-100 dark:bg-gray-800 p-3 rounded overflow-auto max-h-48 font-mono whitespace-pre-wrap break-all">
                {content}
              </pre>
            )}
          </>
        )}
      </div>

      {/* Metadata */}
      {metadata && (
        <div className="p-3 border-t border-gray-200 dark:border-gray-700 text-xs text-gray-500 dark:text-gray-400 space-y-1">
          <div className="flex justify-between">
            <span>Kind</span>
            <span className="text-gray-700 dark:text-gray-300">
              {metadata.isDirectory
                ? 'Folder'
                : metadata.extension?.toUpperCase() || 'File'}
            </span>
          </div>
          {metadata.isFile && (
            <div className="flex justify-between">
              <span>Size</span>
              <span className="text-gray-700 dark:text-gray-300">
                {formatFileSize(metadata.size)}
              </span>
            </div>
          )}
          <div className="flex justify-between">
            <span>Modified</span>
            <span className="text-gray-700 dark:text-gray-300">
              {formatDate(metadata.modifiedAt)}
            </span>
          </div>
          <div className="flex justify-between">
            <span>Created</span>
            <span className="text-gray-700 dark:text-gray-300">
              {formatDate(metadata.createdAt)}
            </span>
          </div>
        </div>
      )}
    </div>
  );
}

function isInputFocused(): boolean {
  const active = document.activeElement;
  return (
    active instanceof HTMLInputElement ||
    active instanceof HTMLTextAreaElement ||
    active?.getAttribute('contenteditable') === 'true'
  );
}
