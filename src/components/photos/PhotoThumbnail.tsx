import { useThumbnail } from '../../hooks/useThumbnail';
import type { PhotoEntry } from '../../types/photo';
import { cn } from '../../lib/utils';
import { Image as ImageIcon } from 'lucide-react';

interface PhotoThumbnailProps {
  photo: PhotoEntry;
  size: number;
  onClick: () => void;
}

export function PhotoThumbnail({ photo, size, onClick }: PhotoThumbnailProps) {
  const { thumbnail, loading } = useThumbnail(photo.path, photo.extension, size);

  return (
    <div
      onClick={onClick}
      className="group relative cursor-pointer flex-shrink-0"
      style={{ width: size, height: size }}
    >
      <div
        className={cn(
          'absolute inset-0 rounded-xl overflow-hidden',
          'bg-gray-100 dark:bg-gray-800/50',
          'transition-all duration-200 ease-out',
          'group-hover:shadow-lg group-hover:shadow-black/20',
          'group-hover:scale-[1.03]'
        )}
      >
        {loading ? (
          <div className="w-full h-full animate-pulse bg-gradient-to-br from-gray-200 to-gray-300 dark:from-gray-700 dark:to-gray-800" />
        ) : thumbnail ? (
          <img
            src={`data:image/webp;base64,${thumbnail}`}
            alt={photo.name}
            className="w-full h-full object-cover"
            loading="lazy"
          />
        ) : (
          <div className="w-full h-full flex items-center justify-center bg-gradient-to-br from-gray-100 to-gray-200 dark:from-gray-800 dark:to-gray-900">
            <ImageIcon size={28} className="text-gray-300 dark:text-gray-600" />
          </div>
        )}

        {/* Subtle overlay on hover */}
        <div className="absolute inset-0 bg-black/0 group-hover:bg-black/5 transition-colors duration-200" />
      </div>
    </div>
  );
}
