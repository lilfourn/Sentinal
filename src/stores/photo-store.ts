import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import type { PhotoEntry, PhotoGroup, PhotoScanResult } from '../types/photo';

interface PhotoState {
  photos: PhotoEntry[];
  groupedPhotos: PhotoGroup[];
  totalCount: number;
  isLoading: boolean;
  isScanning: boolean;
  scanDurationMs: number;
  error: string | null;
  sortBy: 'date' | 'name' | 'size';
  sortDirection: 'asc' | 'desc';
  selectedPhotos: Set<string>;
  lightboxOpen: boolean;
  lightboxIndex: number;
}

interface PhotoActions {
  scanPhotos: () => Promise<void>;
  refreshPhotos: () => Promise<void>;
  setSortBy: (field: PhotoState['sortBy']) => void;
  toggleSortDirection: () => void;
  selectPhoto: (path: string, additive: boolean) => void;
  clearSelection: () => void;
  openLightbox: (index: number) => void;
  closeLightbox: () => void;
  nextPhoto: () => void;
  prevPhoto: () => void;
}

function formatDateLabel(dateKey: string): string {
  const date = new Date(dateKey);
  const today = new Date();
  const yesterday = new Date(today);
  yesterday.setDate(yesterday.getDate() - 1);

  const dateStr = date.toDateString();
  if (dateStr === today.toDateString()) {
    return 'Today';
  }
  if (dateStr === yesterday.toDateString()) {
    return 'Yesterday';
  }

  return date.toLocaleDateString('en-US', {
    weekday: 'long',
    year: 'numeric',
    month: 'long',
    day: 'numeric',
  });
}

function groupPhotosByDate(photos: PhotoEntry[]): PhotoGroup[] {
  const groups = new Map<string, PhotoEntry[]>();

  for (const photo of photos) {
    const timestamp = photo.createdAt ?? photo.modifiedAt;
    if (!timestamp) continue;

    const dateKey = new Date(timestamp).toISOString().split('T')[0];
    if (!groups.has(dateKey)) {
      groups.set(dateKey, []);
    }
    groups.get(dateKey)!.push(photo);
  }

  return Array.from(groups.entries())
    .sort(([a], [b]) => b.localeCompare(a))
    .map(([dateKey, photos]) => ({
      id: dateKey,
      label: formatDateLabel(dateKey),
      photos,
    }));
}

export const usePhotoStore = create<PhotoState & PhotoActions>((set, get) => ({
  photos: [],
  groupedPhotos: [],
  totalCount: 0,
  isLoading: false,
  isScanning: false,
  scanDurationMs: 0,
  error: null,
  sortBy: 'date',
  sortDirection: 'desc',
  selectedPhotos: new Set(),
  lightboxOpen: false,
  lightboxIndex: 0,

  scanPhotos: async () => {
    set({ isLoading: true, isScanning: true, error: null });
    try {
      // Get default photo directories
      const dirs = await invoke<[string, string][]>('get_photo_directories');
      const paths = dirs.map(([, path]) => path);

      // Scan for photos
      const result = await invoke<PhotoScanResult>('scan_photos', {
        directories: paths,
      });

      const grouped = groupPhotosByDate(result.photos);

      set({
        photos: result.photos,
        groupedPhotos: grouped,
        totalCount: result.totalCount,
        scanDurationMs: result.scanDurationMs,
        isLoading: false,
        isScanning: false,
      });
    } catch (error) {
      set({
        error: String(error),
        isLoading: false,
        isScanning: false,
      });
    }
  },

  refreshPhotos: async () => {
    await get().scanPhotos();
  },

  setSortBy: (field) => {
    set({ sortBy: field });
    // Re-sort photos
    const { photos, sortDirection } = get();
    const sorted = [...photos].sort((a, b) => {
      let cmp = 0;
      switch (field) {
        case 'date':
          cmp = (a.createdAt ?? 0) - (b.createdAt ?? 0);
          break;
        case 'name':
          cmp = a.name.localeCompare(b.name);
          break;
        case 'size':
          cmp = a.size - b.size;
          break;
      }
      return sortDirection === 'desc' ? -cmp : cmp;
    });
    set({ photos: sorted, groupedPhotos: groupPhotosByDate(sorted) });
  },

  toggleSortDirection: () => {
    const { sortDirection, sortBy } = get();
    const newDirection = sortDirection === 'asc' ? 'desc' : 'asc';
    set({ sortDirection: newDirection });
    get().setSortBy(sortBy);
  },

  selectPhoto: (path, additive) => {
    const { selectedPhotos } = get();
    const newSelection = new Set(additive ? selectedPhotos : []);
    if (newSelection.has(path)) {
      newSelection.delete(path);
    } else {
      newSelection.add(path);
    }
    set({ selectedPhotos: newSelection });
  },

  clearSelection: () => {
    set({ selectedPhotos: new Set() });
  },

  openLightbox: (index) => {
    set({ lightboxOpen: true, lightboxIndex: index });
  },

  closeLightbox: () => {
    set({ lightboxOpen: false });
  },

  nextPhoto: () => {
    const { lightboxIndex, photos } = get();
    if (lightboxIndex < photos.length - 1) {
      set({ lightboxIndex: lightboxIndex + 1 });
    }
  },

  prevPhoto: () => {
    const { lightboxIndex } = get();
    if (lightboxIndex > 0) {
      set({ lightboxIndex: lightboxIndex - 1 });
    }
  },
}));
