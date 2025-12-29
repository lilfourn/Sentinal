/** Represents a single file or directory entry */
export interface FileEntry {
  /** File or directory name */
  name: string;
  /** Absolute path */
  path: string;
  /** Whether this is a directory */
  isDirectory: boolean;
  /** Whether this is a file */
  isFile: boolean;
  /** Whether this is a symbolic link */
  isSymlink: boolean;
  /** File size in bytes (0 for directories) */
  size: number;
  /** Last modified timestamp (milliseconds since epoch) */
  modifiedAt: number | null;
  /** Created timestamp (milliseconds since epoch) */
  createdAt: number | null;
  /** File extension (without dot), null for directories */
  extension: string | null;
  /** MIME type guess based on extension */
  mimeType: string | null;
  /** Whether file is hidden */
  isHidden: boolean;
}

/** Represents a directory with its contents */
export interface DirectoryContents {
  /** Path of this directory */
  path: string;
  /** Directory name */
  name: string;
  /** Parent directory path (null for root) */
  parentPath: string | null;
  /** List of entries in this directory */
  entries: FileEntry[];
  /** Total count of entries */
  totalCount: number;
}

/** File metadata for detailed info */
export interface FileMetadata {
  path: string;
  name: string;
  size: number;
  isDirectory: boolean;
  isFile: boolean;
  isSymlink: boolean;
  isReadonly: boolean;
  modifiedAt: number | null;
  createdAt: number | null;
  accessedAt: number | null;
  extension: string | null;
  mimeType: string | null;
}

/** View mode for file list */
export type ViewMode = 'list' | 'grid' | 'columns';

/** Sort field options */
export type SortField = 'name' | 'size' | 'modified' | 'kind';

/** Sort direction */
export type SortDirection = 'asc' | 'desc';

/** User directory info */
export interface UserDirectory {
  name: string;
  path: string;
}
