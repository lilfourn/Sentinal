import { useState, useRef, useEffect, KeyboardEvent, useCallback } from 'react';
import { ArrowUp, StopCircle, Plus, ChevronDown, Brain, X } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { useChatStore, type ChatModel, type MentionItem, getContextStrategy } from '../../stores/chat-store';
import { useNavigationStore } from '../../stores/navigation-store';
import { InlineMentionDropdown } from './InlineMentionDropdown';

const MODEL_OPTIONS: { value: ChatModel; label: string }[] = [
  { value: 'claude-sonnet-4-5', label: 'Sonnet 4.5' },
  { value: 'claude-haiku-4-5', label: 'Haiku 4.5' },
  { value: 'claude-opus-4-5', label: 'Opus 4.5' },
];

// Debounce search delay
const MENTION_SEARCH_DEBOUNCE_MS = 150;

// Extract mention query from text at cursor position
function extractMentionQuery(text: string, cursorPos: number): { query: string; startIndex: number } | null {
  const textBeforeCursor = text.slice(0, cursorPos);
  const lastAtIndex = textBeforeCursor.lastIndexOf('@');

  if (lastAtIndex === -1) return null;

  // Check @ is at start or after whitespace
  const charBefore = lastAtIndex > 0 ? text[lastAtIndex - 1] : ' ';
  if (!/\s/.test(charBefore) && lastAtIndex !== 0) return null;

  const query = textBeforeCursor.slice(lastAtIndex + 1);
  // Space after @ closes the mention
  if (/\s/.test(query)) return null;

  return { query, startIndex: lastAtIndex };
}

export function ChatInput() {
  const [input, setInput] = useState('');
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  // Refs for debounce cleanup and search race condition prevention
  const debounceTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const searchSequenceRef = useRef(0);

  const currentPath = useNavigationStore((s) => s.currentPath);

  const {
    sendMessage,
    abort,
    status,
    activeContext,
    removeContext,
    model,
    setModel,
    extendedThinking,
    setExtendedThinking,
    addContext,
    // Mention state
    isMentionOpen,
    mentionQuery,
    mentionResults,
    selectedMentionIndex,
    openMention,
    closeMention,
    setMentionQuery,
    setMentionResults,
    setMentionLoading,
    selectNextMention,
    selectPrevMention,
    resetMentionSelection,
  } = useChatStore();

  const isProcessing = status === 'thinking' || status === 'streaming';

  // Cleanup debounce timeout on unmount
  useEffect(() => {
    return () => {
      if (debounceTimeoutRef.current) {
        clearTimeout(debounceTimeoutRef.current);
        debounceTimeoutRef.current = null;
      }
    };
  }, []);

  // Debounced search function with race condition protection
  const debouncedSearch = useCallback(
    (query: string, directory: string) => {
      // Clear any pending debounce
      if (debounceTimeoutRef.current) {
        clearTimeout(debounceTimeoutRef.current);
      }

      debounceTimeoutRef.current = setTimeout(async () => {
        // Increment sequence to track this specific search
        const currentSequence = ++searchSequenceRef.current;

        try {
          setMentionLoading(true);

          // Search current directory
          let results = await invoke<MentionItem[]>('list_files_for_mention', {
            directory,
            query: query || null,
            limit: 15,
          });

          // Check if this search is still current (no newer search started)
          if (currentSequence !== searchSequenceRef.current) {
            return; // Stale result, discard
          }

          // If few results, also search home directory
          const homeDir = await invoke<string>('get_home_dir').catch(() => null);
          if (results.length < 5 && homeDir && directory !== homeDir) {
            const homeResults = await invoke<MentionItem[]>('list_files_for_mention', {
              directory: homeDir,
              query: query || null,
              limit: 10,
            });

            // Check again after second async operation
            if (currentSequence !== searchSequenceRef.current) {
              return; // Stale result, discard
            }

            // Dedupe and merge
            const seen = new Set(results.map((r) => r.path));
            results = [...results, ...homeResults.filter((r) => !seen.has(r.path))];
          }

          setMentionResults(results.slice(0, 15));
          resetMentionSelection();
        } catch (err) {
          // Only log error if this search is still current
          if (currentSequence === searchSequenceRef.current) {
            console.error('Mention search failed:', err);
            setMentionResults([]);
          }
        }
      }, MENTION_SEARCH_DEBOUNCE_MS);
    },
    [setMentionLoading, setMentionResults, resetMentionSelection]
  );

  // Trigger search when mention query changes
  useEffect(() => {
    if (isMentionOpen && currentPath) {
      debouncedSearch(mentionQuery, currentPath);
    }
  }, [mentionQuery, isMentionOpen, currentPath, debouncedSearch]);

  // Auto-resize textarea
  useEffect(() => {
    const textarea = textareaRef.current;
    if (textarea) {
      textarea.style.height = 'auto';
      textarea.style.height = `${Math.min(textarea.scrollHeight, 120)}px`;
    }
  }, [input]);

  const handleSend = async () => {
    if (!input.trim() || isProcessing) return;

    const message = input;
    setInput('');
    closeMention();
    await sendMessage(message);
  };

  // Handle mention selection
  const handleMentionSelect = useCallback(
    (item: MentionItem) => {
      const startIndex = useChatStore.getState().mentionStartIndex;
      const query = useChatStore.getState().mentionQuery;

      // Remove @query from input
      const before = input.slice(0, startIndex);
      const after = input.slice(startIndex + query.length + 1); // +1 for @
      setInput(before + after);

      // Add to context
      addContext({
        type: item.isDirectory ? 'folder' : 'file',
        path: item.path,
        name: item.name,
        strategy: getContextStrategy(item.isDirectory ? 'folder' : 'file'),
      });

      closeMention();

      // Refocus textarea
      setTimeout(() => textareaRef.current?.focus(), 0);
    },
    [input, addContext, closeMention]
  );

  const handleKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    // When mention dropdown is open, intercept navigation keys
    if (isMentionOpen) {
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        selectNextMention();
        return;
      }
      if (e.key === 'ArrowUp') {
        e.preventDefault();
        selectPrevMention();
        return;
      }
      if (e.key === 'Enter' || e.key === 'Tab') {
        e.preventDefault();
        if (mentionResults.length > 0) {
          handleMentionSelect(mentionResults[selectedMentionIndex]);
        }
        return;
      }
      if (e.key === 'Escape') {
        e.preventDefault();
        closeMention();
        return;
      }
    }

    // Submit on Enter (without Shift) when dropdown is closed
    if (e.key === 'Enter' && !e.shiftKey && !isMentionOpen) {
      e.preventDefault();
      handleSend();
      return;
    }
  };

  const handleChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const value = e.target.value;
    const cursorPos = e.target.selectionStart;
    setInput(value);

    // Check for @ mention
    const mention = extractMentionQuery(value, cursorPos);

    if (mention) {
      if (!isMentionOpen) {
        openMention(mention.startIndex);
      }
      setMentionQuery(mention.query);
    } else if (isMentionOpen) {
      closeMention();
    }
  };

  const handlePlusClick = () => {
    // Insert @ at cursor position and open mention
    const textarea = textareaRef.current;
    if (!textarea) return;

    const start = textarea.selectionStart;
    const end = textarea.selectionEnd;
    const newValue = input.slice(0, start) + '@' + input.slice(end);
    setInput(newValue);
    openMention(start);
    setMentionQuery('');

    // Set cursor after @
    setTimeout(() => {
      textarea.selectionStart = textarea.selectionEnd = start + 1;
      textarea.focus();
    }, 0);
  };

  return (
    <div className="p-3">
      {/* Unified prompt container */}
      <div ref={containerRef} className="rounded-xl bg-[#2a2a2a] border border-white/10 overflow-hidden relative">
        {/* Inline mention dropdown */}
        <InlineMentionDropdown anchorRef={containerRef} onSelect={handleMentionSelect} />

        {/* Context chips - inline above textarea */}
        {activeContext.length > 0 && (
          <div className="px-3 pt-3 pb-1 flex flex-wrap gap-1.5">
            {activeContext.map((item) => (
              <div
                key={item.id}
                className="flex items-center gap-1.5 pl-2 pr-1 py-0.5 bg-white/[0.06] rounded text-[11px] text-gray-400 group"
              >
                <span className="truncate max-w-[120px]">{item.name}</span>
                <button
                  onClick={() => removeContext(item.id)}
                  className="p-0.5 rounded hover:bg-white/10 text-gray-500 hover:text-gray-300 opacity-60 group-hover:opacity-100 transition-opacity"
                  aria-label={`Remove ${item.name}`}
                >
                  <X size={10} />
                </button>
              </div>
            ))}
          </div>
        )}

        {/* Textarea area */}
        <div className={`px-4 ${activeContext.length > 0 ? 'pt-1' : 'pt-3'} pb-2`}>
          <textarea
            ref={textareaRef}
            value={input}
            onChange={handleChange}
            onKeyDown={handleKeyDown}
            placeholder={
              activeContext.length > 0
                ? 'Ask about the selected files...'
                : 'How can I help you today?'
            }
            className="w-full resize-none bg-transparent text-sm text-gray-100 placeholder-gray-500 focus:outline-none min-h-[24px] max-h-[120px]"
            rows={1}
            disabled={isProcessing}
            aria-label="Chat message input"
            aria-describedby={activeContext.length > 0 ? 'context-hint' : undefined}
          />
        </div>

        {/* Bottom toolbar */}
        <div className="flex items-center justify-between px-3 pb-3">
          {/* Left side buttons */}
          <div className="flex items-center gap-1">
            {/* Add context button */}
            <button
              onClick={handlePlusClick}
              className="p-2 rounded-lg hover:bg-white/10 text-gray-400 hover:text-gray-200 transition-colors"
              title="Add file or folder context (@)"
              aria-label="Add file or folder context"
            >
              <Plus size={18} aria-hidden="true" />
            </button>

            {/* Extended thinking toggle */}
            <button
              onClick={() => setExtendedThinking(!extendedThinking)}
              className={`p-2 rounded-lg transition-colors ${
                extendedThinking
                  ? 'bg-purple-500/20 text-purple-400 hover:bg-purple-500/30'
                  : 'hover:bg-white/10 text-gray-400 hover:text-gray-200'
              }`}
              title={extendedThinking ? 'Extended thinking enabled' : 'Extended thinking disabled'}
              aria-label={extendedThinking ? 'Disable extended thinking' : 'Enable extended thinking'}
              aria-pressed={extendedThinking}
              disabled={isProcessing}
            >
              <Brain size={18} aria-hidden="true" />
            </button>
          </div>

          {/* Right side - Model selector + Send */}
          <div className="flex items-center gap-2">
            {/* Model selector */}
            <div className="relative">
              <select
                value={model}
                onChange={(e) => setModel(e.target.value as ChatModel)}
                className="appearance-none text-xs text-gray-400 bg-transparent pr-5 pl-2 py-1 focus:outline-none cursor-pointer hover:text-gray-200 transition-colors"
                disabled={isProcessing}
                aria-label="Select AI model"
              >
                {MODEL_OPTIONS.map((opt) => (
                  <option key={opt.value} value={opt.value} className="bg-[#2a2a2a] text-gray-200">
                    {opt.label}
                  </option>
                ))}
              </select>
              <ChevronDown size={12} className="absolute right-0 top-1/2 -translate-y-1/2 text-gray-500 pointer-events-none" aria-hidden="true" />
            </div>

            {/* Send/Stop button */}
            {isProcessing ? (
              <button
                onClick={abort}
                className="p-2 rounded-lg bg-red-500/80 hover:bg-red-500 text-white transition-colors"
                title="Stop generation"
                aria-label="Stop generation"
              >
                <StopCircle size={18} aria-hidden="true" />
              </button>
            ) : (
              <button
                onClick={handleSend}
                disabled={!input.trim()}
                className="p-2 rounded-lg bg-orange-600/80 hover:bg-orange-600 disabled:bg-gray-700 disabled:text-gray-500 disabled:cursor-not-allowed text-white transition-colors"
                title="Send message"
                aria-label="Send message"
              >
                <ArrowUp size={18} aria-hidden="true" />
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
