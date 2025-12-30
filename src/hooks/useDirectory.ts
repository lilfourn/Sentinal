import { useQuery } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import type { DirectoryContents, UserDirectory } from '../types/file';
import type { DirectoryError } from '../types/permissions';

/** Fetch directory contents */
async function fetchDirectory(path: string, showHidden: boolean): Promise<DirectoryContents> {
  try {
    return await invoke<DirectoryContents>('read_directory', {
      path,
      showHidden,
    });
  } catch (error) {
    // Check if it's our structured error
    if (typeof error === 'object' && error !== null && 'isPermissionError' in error) {
      throw error as DirectoryError;
    }
    // Legacy string error - check for permission issues
    const errorStr = String(error);
    if (errorStr.includes('Operation not permitted') || errorStr.includes('os error 1')) {
      throw {
        code: 'PERMISSION_DENIED',
        message: 'Access denied. Please grant Full Disk Access permission.',
        path,
        isPermissionError: true,
      } as DirectoryError;
    }
    throw error;
  }
}

/** Hook to get directory contents */
export function useDirectory(path: string, showHidden: boolean = false) {
  return useQuery<DirectoryContents, DirectoryError>({
    queryKey: ['directory', path, showHidden],
    queryFn: () => fetchDirectory(path, showHidden),
    enabled: !!path,
    staleTime: 5000, // Cache for 5 seconds
    refetchOnWindowFocus: true,
    retry: (failureCount, error) => {
      // Don't retry permission errors
      if (error?.isPermissionError) return false;
      return failureCount < 3;
    },
  });
}

/** Fetch user directories (Home, Documents, Downloads, etc.) */
async function fetchUserDirectories(): Promise<UserDirectory[]> {
  const dirs = await invoke<[string, string][]>('get_user_directories');
  return dirs.map(([name, path]) => ({ name, path }));
}

/** Hook to get user directories */
export function useUserDirectories() {
  return useQuery({
    queryKey: ['userDirectories'],
    queryFn: fetchUserDirectories,
    staleTime: 60000, // Cache for 1 minute
  });
}

/** Get home directory */
export async function getHomeDirectory(): Promise<string> {
  return invoke<string>('get_home_directory');
}

/** Get downloads directory */
export async function getDownloadsDirectory(): Promise<string> {
  return invoke<string>('get_downloads_directory');
}
