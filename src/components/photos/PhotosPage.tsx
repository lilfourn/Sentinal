import { useEffect, useMemo } from 'react';
import { usePhotoStore } from '../../stores/photo-store';
import { PhotoGrid } from './PhotoGrid';
import { Lightbox } from './Lightbox';
import { PhotoToolbar } from './PhotoToolbar';
import { Loader2, Image, RefreshCw } from 'lucide-react';
import { cn } from '../../lib/utils';

export function PhotosPage() {
  const {
    groupedPhotos,
    photos,
    isLoading,
    isScanning,
    error,
    lightboxOpen,
    lightboxIndex,
    openLightbox,
    closeLightbox,
    nextPhoto,
    prevPhoto,
    scanPhotos,
  } = usePhotoStore();

  // Scan on mount
  useEffect(() => {
    if (photos.length === 0 && !isLoading) {
      scanPhotos();
    }
  }, [photos.length, isLoading, scanPhotos]);

  // Flatten photos for lightbox navigation
  const allPhotos = useMemo(() => groupedPhotos.flatMap((g) => g.photos), [groupedPhotos]);

  if (isLoading && photos.length === 0) {
    return (
      <main className="flex-1 flex flex-col min-w-0 glass-main">
        <PhotoToolbar />
        <div className="flex-1 flex flex-col items-center justify-center gap-4">
          <div className="relative">
            <div className="w-16 h-16 rounded-2xl bg-gradient-to-br from-orange-100 to-orange-200 dark:from-orange-900/30 dark:to-orange-800/30 flex items-center justify-center">
              <Image className="text-orange-500" size={28} />
            </div>
            <Loader2 className="absolute -bottom-1 -right-1 animate-spin text-orange-500" size={20} />
          </div>
          <div className="text-center">
            <p className="text-sm font-medium text-gray-700 dark:text-gray-300">
              {isScanning ? 'Scanning for photos...' : 'Loading photos...'}
            </p>
            <p className="text-xs text-gray-500 mt-1">This may take a moment</p>
          </div>
        </div>
      </main>
    );
  }

  if (error) {
    return (
      <main className="flex-1 flex flex-col min-w-0 glass-main">
        <PhotoToolbar />
        <div className="flex-1 flex flex-col items-center justify-center gap-4 p-8">
          <div className="w-16 h-16 rounded-2xl bg-red-100 dark:bg-red-900/30 flex items-center justify-center">
            <Image className="text-red-500" size={28} />
          </div>
          <div className="text-center">
            <p className="text-sm font-medium text-gray-700 dark:text-gray-300">Unable to load photos</p>
            <p className="text-xs text-gray-500 mt-1 max-w-xs">{error}</p>
          </div>
          <button
            onClick={scanPhotos}
            className={cn(
              'flex items-center gap-2 px-4 py-2 text-sm font-medium rounded-lg',
              'bg-gray-100 dark:bg-gray-800 text-gray-700 dark:text-gray-300',
              'hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors'
            )}
          >
            <RefreshCw size={14} />
            Try Again
          </button>
        </div>
      </main>
    );
  }

  if (photos.length === 0) {
    return (
      <main className="flex-1 flex flex-col min-w-0 glass-main">
        <PhotoToolbar />
        <div className="flex-1 flex flex-col items-center justify-center gap-4 p-8">
          <div className="w-16 h-16 rounded-2xl bg-gray-100 dark:bg-gray-800 flex items-center justify-center">
            <Image className="text-gray-400" size={28} />
          </div>
          <div className="text-center">
            <p className="text-sm font-medium text-gray-700 dark:text-gray-300">No photos found</p>
            <p className="text-xs text-gray-500 mt-1 max-w-xs">
              Photos from your Pictures folder, Desktop, Downloads, and Documents will appear here
            </p>
          </div>
        </div>
      </main>
    );
  }

  return (
    <main className="flex-1 flex flex-col min-w-0 glass-main">
      <PhotoToolbar />

      <PhotoGrid groups={groupedPhotos} onPhotoClick={(_, globalIndex) => openLightbox(globalIndex)} />

      {lightboxOpen && (
        <Lightbox
          photos={allPhotos}
          currentIndex={lightboxIndex}
          onClose={closeLightbox}
          onNext={nextPhoto}
          onPrev={prevPhoto}
        />
      )}
    </main>
  );
}
