import { useDirectory } from '../../hooks/useDirectory';
import { useNavigationStore } from '../../stores/navigation-store';
import { FileListView } from '../file-list/FileListView';
import { FileGridView } from '../file-list/FileGridView';
import { FileColumnsView } from '../file-list/FileColumnsView';
import { Breadcrumbs } from './Breadcrumbs';
import { Loader2 } from 'lucide-react';

export function MainView() {
  const { currentPath, showHidden, viewMode } = useNavigationStore();
  const { data, isLoading, error } = useDirectory(currentPath, showHidden);

  const renderFileView = () => {
    if (!data) return null;

    switch (viewMode) {
      case 'grid':
        return <FileGridView entries={data.entries} />;
      case 'columns':
        return <FileColumnsView entries={data.entries} />;
      case 'list':
      default:
        return <FileListView entries={data.entries} />;
    }
  };

  return (
    <main className="flex-1 flex flex-col min-w-0 glass-main">
      {/* Breadcrumbs */}
      <Breadcrumbs />

      {/* Divider */}
      <div className="border-b border-gray-200/20" />

      {/* Content area */}
      <div className="flex-1 overflow-hidden">
        {!currentPath ? (
          <div className="flex items-center justify-center h-full text-gray-400">
            Select a folder from the sidebar to get started
          </div>
        ) : isLoading ? (
          <div className="flex items-center justify-center h-full">
            <Loader2 className="animate-spin text-gray-400" size={24} />
          </div>
        ) : error ? (
          <div className="flex flex-col items-center justify-center h-full gap-2">
            <span className="text-red-500">Failed to load directory</span>
            <span className="text-sm text-gray-400">{String(error)}</span>
          </div>
        ) : (
          renderFileView()
        )}
      </div>
    </main>
  );
}
