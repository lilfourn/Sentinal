import { useDirectory } from '../../hooks/useDirectory';
import { useNavigationStore } from '../../stores/navigation-store';
import { useOrganizeRefresh } from '../../hooks/useOrganizeRefresh';
import { FileListView } from '../file-list/FileListView';
import { FileGridView } from '../file-list/FileGridView';
import { FileColumnsView } from '../file-list/FileColumnsView';
import { Breadcrumbs } from './Breadcrumbs';
import { PermissionErrorView } from '../permissions/PermissionErrorView';
import { PhotosPage } from '../photos/PhotosPage';
import { Loader2 } from 'lucide-react';

export function MainView() {
  const { currentPath, showHidden, viewMode, appMode } = useNavigationStore();

  // Watch for organization completion and auto-refresh file listings
  useOrganizeRefresh();
  const { data, isLoading, error } = useDirectory(currentPath, showHidden);

  // Photos mode
  if (appMode === 'photos') {
    return <PhotosPage />;
  }

  // Check if error is a permission error
  const isPermissionError =
    error &&
    typeof error === 'object' &&
    'isPermissionError' in error &&
    error.isPermissionError;

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
        ) : isPermissionError ? (
          <PermissionErrorView path={currentPath} />
        ) : error ? (
          <div className="flex flex-col items-center justify-center h-full gap-2">
            <span className="text-red-500">Failed to load directory</span>
            <span className="text-sm text-gray-400">{error.message || String(error)}</span>
          </div>
        ) : (
          renderFileView()
        )}
      </div>
    </main>
  );
}
