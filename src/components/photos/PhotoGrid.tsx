import { useRef, useMemo, useState, useEffect } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import type { PhotoEntry, PhotoGroup } from '../../types/photo';
import { PhotoThumbnail } from './PhotoThumbnail';
import { DateHeader } from './DateHeader';

interface PhotoGridProps {
  groups: PhotoGroup[];
  onPhotoClick: (photo: PhotoEntry, globalIndex: number) => void;
}

type GridRow =
  | { type: 'header'; id: string; label: string; photoCount: number }
  | { type: 'photos'; id: string; photos: PhotoEntry[]; startIndex: number };

const GAP = 8;
const PADDING = 24;
const MIN_THUMBNAIL_SIZE = 140;
const MAX_THUMBNAIL_SIZE = 200;

export function PhotoGrid({ groups, onPhotoClick }: PhotoGridProps) {
  const parentRef = useRef<HTMLDivElement>(null);
  const [containerWidth, setContainerWidth] = useState(800);

  // Calculate responsive thumbnail size and photos per row
  const { thumbnailSize, photosPerRow } = useMemo(() => {
    const availableWidth = containerWidth - PADDING * 2;
    // Calculate how many photos fit at minimum size
    const maxPhotos = Math.floor((availableWidth + GAP) / (MIN_THUMBNAIL_SIZE + GAP));
    const photosPerRow = Math.max(4, Math.min(maxPhotos, 8));
    // Calculate actual size to fill the width
    const thumbnailSize = Math.min(
      MAX_THUMBNAIL_SIZE,
      Math.floor((availableWidth - (photosPerRow - 1) * GAP) / photosPerRow)
    );
    return { thumbnailSize, photosPerRow };
  }, [containerWidth]);

  // Track container width
  useEffect(() => {
    const container = parentRef.current;
    if (!container) return;

    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        setContainerWidth(entry.contentRect.width);
      }
    });

    observer.observe(container);
    setContainerWidth(container.clientWidth);

    return () => observer.disconnect();
  }, []);

  // Flatten groups into rows for virtualization
  const rows = useMemo(() => {
    const rows: GridRow[] = [];
    let globalIndex = 0;

    for (const group of groups) {
      // Add header row
      rows.push({
        type: 'header',
        id: `header-${group.id}`,
        label: group.label,
        photoCount: group.photos.length,
      });

      // Split photos into rows
      for (let i = 0; i < group.photos.length; i += photosPerRow) {
        const rowPhotos = group.photos.slice(i, i + photosPerRow);
        const startIndex = globalIndex;
        globalIndex += rowPhotos.length;

        rows.push({
          type: 'photos',
          id: `photos-${group.id}-${i}`,
          photos: rowPhotos,
          startIndex,
        });
      }
    }

    return rows;
  }, [groups, photosPerRow]);

  const virtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => parentRef.current,
    estimateSize: (index) => (rows[index].type === 'header' ? 44 : thumbnailSize + GAP),
    overscan: 3,
  });

  return (
    <div ref={parentRef} className="flex-1 overflow-auto">
      <div
        style={{
          height: virtualizer.getTotalSize(),
          width: '100%',
          position: 'relative',
        }}
      >
        {virtualizer.getVirtualItems().map((virtualRow) => {
          const row = rows[virtualRow.index];

          if (row.type === 'header') {
            return (
              <div
                key={row.id}
                style={{
                  position: 'absolute',
                  top: 0,
                  left: 0,
                  width: '100%',
                  height: virtualRow.size,
                  transform: `translateY(${virtualRow.start}px)`,
                }}
              >
                <DateHeader label={row.label} photoCount={row.photoCount} />
              </div>
            );
          }

          return (
            <div
              key={row.id}
              style={{
                position: 'absolute',
                top: 0,
                left: 0,
                width: '100%',
                height: virtualRow.size,
                transform: `translateY(${virtualRow.start}px)`,
                paddingLeft: PADDING,
                paddingRight: PADDING,
                gap: GAP,
              }}
              className="flex"
            >
              {row.photos.map((photo, i) => (
                <PhotoThumbnail
                  key={photo.path}
                  photo={photo}
                  size={thumbnailSize}
                  onClick={() => onPhotoClick(photo, row.startIndex + i)}
                />
              ))}
            </div>
          );
        })}
      </div>
    </div>
  );
}
