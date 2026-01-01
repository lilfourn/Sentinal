import { useEffect, useRef } from 'react';
import { SquarePen, X } from 'lucide-react';
import { useChatStore } from '../../stores/chat-store';
import { useChatDropZone } from '../../hooks/useChatDropZone';
import { MessageList } from './MessageList';
import { ChatInput } from './ChatInput';
import { DropZoneOverlay } from './DropZoneOverlay';

interface ChatPanelProps {
  isOpen: boolean;
  onClose: () => void;
}

export function ChatPanel({ isOpen, onClose }: ChatPanelProps) {
  const { addContextBatch } = useChatStore();
  const panelRef = useRef<HTMLDivElement>(null);

  // Combined drop zone handling for internal and external drops
  const { isDragOver, dragSource, pendingItems, handlers } = useChatDropZone({
    onContextAdd: (items) => addContextBatch(items),
    enabled: isOpen,
  });

  // Sync external isOpen prop with store
  useEffect(() => {
    if (isOpen) {
      useChatStore.getState().open();
    } else {
      useChatStore.getState().close();
    }
  }, [isOpen]);

  if (!isOpen) {
    return null;
  }

  return (
    <div
      ref={panelRef}
      className={`
        relative w-[420px] flex-shrink-0 h-full overflow-hidden flex flex-col
        glass-sidebar
        border-l border-white/5
        transition-all duration-200
        ${isDragOver ? 'ring-2 ring-inset ring-orange-500 bg-orange-50/50 dark:bg-orange-900/20 select-none' : ''}
      `}
      onDragOver={handlers.onDragOver}
      onDragLeave={handlers.onDragLeave}
      onDrop={handlers.onDrop}
    >
      {/* Top right controls */}
      <div className="absolute top-2 right-2 z-20 flex items-center gap-1">
        <button
          onClick={() => useChatStore.getState().clearHistory()}
          className="p-1.5 rounded-md hover:bg-white/5 text-gray-500 hover:text-gray-300 transition-colors"
          title="New chat"
        >
          <SquarePen size={16} />
        </button>
        <button
          onClick={onClose}
          className="p-1.5 rounded-md hover:bg-white/5 text-gray-500 hover:text-gray-300 transition-colors"
          title="Close chat"
        >
          <X size={16} />
        </button>
      </div>

      {/* Drop zone overlay with item preview */}
      <DropZoneOverlay
        isVisible={isDragOver}
        dragSource={dragSource}
        pendingItems={pendingItems}
      />

      {/* Messages */}
      <MessageList />

      {/* Input with inline mention dropdown */}
      <ChatInput />
    </div>
  );
}
