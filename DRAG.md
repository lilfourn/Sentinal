# Drag and Drop System Documentation

Comprehensive documentation of Sentinel's drag and drop implementation for moving files/folders into different folders.

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Core Types & Interfaces](#core-types--interfaces)
3. [Frontend Implementation](#frontend-implementation)
4. [Backend Implementation](#backend-implementation)
5. [Visual Feedback System](#visual-feedback-system)
6. [Data Flow & Event Handling](#data-flow--event-handling)
7. [Validation & Error Handling](#validation--error-handling)
8. [Key Files Reference](#key-files-reference)

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           USER INTERACTION                                  │
│  Click + Drag (5px threshold) → Drag Preview → Hover Folder → Release       │
└────────────────────────────────────┬────────────────────────────────────────┘
                                     │
┌────────────────────────────────────▼────────────────────────────────────────┐
│                        FRONTEND (React + TypeScript)                         │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐          │
│  │ DragDropProvider│◄───│  useDragDrop    │◄───│ FileGridView    │          │
│  │   (Context)     │    │    (Hook)       │    │ FileListView    │          │
│  │                 │    │                 │    │ FileColumnsView │          │
│  │ - dragState     │    │ - startDrag()   │    │                 │          │
│  │ - dropTarget    │    │ - setDropTarget │    │ - handleDragStart│          │
│  │ - executeDrop() │    │ - executeDrop() │    │ - handleDragEnter│          │
│  └────────┬────────┘    └─────────────────┘    │ - handleDrop    │          │
│           │                                     └─────────────────┘          │
│           │                                                                  │
│  ┌────────▼────────┐    ┌─────────────────┐    ┌─────────────────┐          │
│  │  DragPreview    │    │ selection-store │    │ cycle-detection │          │
│  │  (Component)    │    │   (Zustand)     │    │    (lib)        │          │
│  │                 │    │                 │    │                 │          │
│  │ - Follows cursor│    │ - selectedPaths │    │ - wouldCreateCycle        │
│  │ - Shows count   │    │ - selectMultiple│    │ - getCycleReason│          │
│  │ - Copy indicator│    └─────────────────┘    └─────────────────┘          │
│  └─────────────────┘                                                        │
│                                                                             │
└────────────────────────────────────┬────────────────────────────────────────┘
                                     │ invoke()
                                     │
┌────────────────────────────────────▼────────────────────────────────────────┐
│                          BACKEND (Rust + Tauri)                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────┐            │
│  │                    commands/filesystem.rs                    │            │
│  │                                                              │            │
│  │  validate_drag_drop()    move_files_batch()   copy_files_batch()         │
│  │  - Target exists?        - Pre-validate       - Pre-validate │            │
│  │  - Is directory?         - fs::rename()       - fs::copy()   │            │
│  │  - Cycle check           - Copy+delete fallback              │            │
│  │  - Name collision?       - Return new paths   - Return new paths         │
│  │  - Protected path?                                           │            │
│  └──────────────────────────────┬──────────────────────────────┘            │
│                                 │                                            │
│  ┌──────────────────────────────▼──────────────────────────────┐            │
│  │                    security/cycle_detection.rs               │            │
│  │                                                              │            │
│  │  would_create_cycle()         validate_multi_drop()          │            │
│  │  - Canonicalize paths         - Check target not in sources  │            │
│  │  - Same directory check       - Validate each source         │            │
│  │  - Descendant check           - Uses canonical paths         │            │
│  └──────────────────────────────────────────────────────────────┘            │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Core Types & Interfaces

### DragState (`src/types/drag-drop.ts:4-13`)

```typescript
export interface DragState {
  /** Items being dragged (can be multiple with selection) */
  items: FileEntry[];
  /** Source directory path where items originated */
  sourceDirectory: string;
  /** Whether Alt/Option is held (copy mode vs move) */
  isCopy: boolean;
  /** Current mouse position for preview rendering */
  position: { x: number; y: number };
}
```

### DropTarget (`src/types/drag-drop.ts:15-20`)

```typescript
export interface DropTarget {
  /** Target folder path */
  path: string;
  /** Whether drop is allowed */
  isValid: boolean;
  /** Reason if invalid */
  reason?: DropInvalidReason;
}
```

### DropInvalidReason (`src/types/drag-drop.ts:23-30`)

```typescript
export type DropInvalidReason =
  | 'cycle_self'           // Dropping into itself
  | 'cycle_descendant'     // Dropping into a descendant folder
  | 'target_selected'      // Target is one of the dragged items
  | 'name_collision'       // File already exists at destination
  | 'permission_denied'    // No write permission
  | 'not_directory'        // Target is not a directory
  | 'protected_path';      // System protected path
```

### DragDropError (Rust - `src-tauri/src/commands/filesystem.rs:22-45`)

```rust
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DragDropError {
    CycleDetectedSelf { path: String },
    CycleDetectedDescendant { source: String, target: String },
    TargetIsSelected { target: String },
    NameCollision { name: String, destination: String },
    PermissionDenied { path: String, message: String },
    SourceNotFound { path: String },
    TargetNotDirectory { path: String },
    ProtectedPath { path: String },
    IoError { message: String },
}
```

---

## Frontend Implementation

### Core Hook: `useDragDrop` (`src/hooks/useDragDrop.ts`)

The central hook managing all drag-drop state and operations.

#### Starting a Drag (Lines 46-56)

```typescript
const startDrag = useCallback(
  (items: FileEntry[], sourceDirectory: string) => {
    setDragState({
      items,
      sourceDirectory,
      isCopy: false,
      position: { x: 0, y: 0 },
    });
  },
  []
);
```

#### Setting Drop Target with Validation (Lines 70-113)

Two-stage validation: frontend synchronous + backend asynchronous.

```typescript
const setDropTarget = useCallback(
  (path: string | null, isDirectory: boolean) => {
    if (!path || !dragState) {
      setDropTargetState(null);
      return;
    }

    const sourcePaths = dragState.items.map((item) => item.path);

    // STAGE 1: Frontend synchronous validation (immediate feedback)
    if (!isDirectory) {
      setDropTargetState({ path, isValid: false, reason: 'not_directory' });
      return;
    }

    if (wouldCreateCycleMulti(sourcePaths, path)) {
      const reason = getCycleReason(sourcePaths, path) as DropInvalidReason;
      setDropTargetState({ path, isValid: false, reason });
      return;
    }

    // Optimistically show as valid
    setDropTargetState({ path, isValid: true });

    // STAGE 2: Backend async validation (permissions, symlinks, collisions)
    invoke('validate_drag_drop', {
      sources: sourcePaths,
      target: path,
    })
      .then(() => {/* Already valid */})
      .catch((error: unknown) => {
        const errorObj = error as { type?: string };
        const reason = mapErrorToReason(errorObj?.type);
        setDropTargetState((prev) =>
          prev?.path === path ? { path, isValid: false, reason } : prev
        );
      });
  },
  [dragState]
);
```

#### Executing the Drop (Lines 115-145)

```typescript
const executeDrop = useCallback(async (): Promise<boolean> => {
  if (!dragState || !dropTarget?.isValid) {
    return false;
  }

  const sourcePaths = dragState.items.map((item) => item.path);
  const command = dragState.isCopy ? 'copy_files_batch' : 'move_files_batch';

  try {
    const newPaths = await invoke<string[]>(command, {
      sources: sourcePaths,
      targetDirectory: dropTarget.path,
    });

    onDropComplete?.(newPaths, dragState.isCopy);
    setDragState(null);
    setDropTargetState(null);
    return true;
  } catch (error) {
    onDropError?.(errorMessage);
    setDragState(null);
    setDropTargetState(null);
    return false;
  }
}, [dragState, dropTarget, onDropComplete, onDropError]);
```

#### Global Event Handlers (Lines 154-213)

```typescript
useEffect(() => {
  if (!dragState) return;

  const handleMouseMove = (e: MouseEvent) => {
    updateDragPosition(e.clientX, e.clientY, e.altKey);
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === 'Escape') cancelDrag();
    if (e.key === 'Alt') setDragState((prev) => prev ? { ...prev, isCopy: true } : null);
  };

  const handleKeyUp = (e: KeyboardEvent) => {
    if (e.key === 'Alt') setDragState((prev) => prev ? { ...prev, isCopy: false } : null);
  };

  const handleMouseUp = () => {
    // Cancel if dropped on blank space (no folder caught it)
    requestAnimationFrame(() => {
      setDragState((currentState) => {
        if (currentState) {
          setDropTargetState(null);
          onDragCancel?.();
          return null;
        }
        return currentState;
      });
    });
  };

  document.addEventListener('mousemove', handleMouseMove);
  document.addEventListener('mouseup', handleMouseUp);
  document.addEventListener('keydown', handleKeyDown);
  document.addEventListener('keyup', handleKeyUp);
  window.addEventListener('blur', handleBlur);

  return () => { /* cleanup */ };
}, [dragState, updateDragPosition, cancelDrag, onDragCancel]);
```

### File Views: Drag Handlers

#### FileGridView/FileListView - Multi-Selection Drag (`src/components/file-list/FileGridView.tsx:478-515`)

```typescript
// If clicked item is selected, drag ALL selected items; otherwise just the clicked one
const handleDragStart = useCallback(
  (entry: FileEntry) => {
    const itemsToDrag = selectedPaths.has(entry.path)
      ? entries.filter((e) => selectedPaths.has(e.path))
      : [entry];

    startFileDrag(itemsToDrag, currentPath);
  },
  [selectedPaths, entries, currentPath, startFileDrag]
);

// Only directories can be drop targets
const handleDragEnter = useCallback(
  (entry: FileEntry) => {
    if (entry.isDirectory && isDragDropActive) {
      setDropTarget(entry.path, true);
    }
  },
  [isDragDropActive, setDropTarget]
);

// Execute drop when mouse released over valid folder
const handleDrop = useCallback(
  async (entry: FileEntry) => {
    if (entry.isDirectory && isDragDropActive) {
      setDropTarget(entry.path, true);
      await executeDrop();
    }
  },
  [isDragDropActive, setDropTarget, executeDrop]
);

// Clear drop target when mouse leaves
const handleMouseLeave = useCallback(() => {
  if (isDragDropActive) {
    setDropTarget(null, false);
  }
}, [isDragDropActive, setDropTarget]);
```

#### FileRow - Drag Threshold Detection (`src/components/file-list/FileRow.tsx:87-116`)

```typescript
const handleMouseDown = (e: React.MouseEvent) => {
  if (e.button !== 0 || isEditing) return;

  const startX = e.clientX;
  const startY = e.clientY;
  const threshold = 5; // 5px movement before considering it a drag

  const handleMouseMove = (moveEvent: MouseEvent) => {
    const deltaX = Math.abs(moveEvent.clientX - startX);
    const deltaY = Math.abs(moveEvent.clientY - startY);

    if (deltaX > threshold || deltaY > threshold) {
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
      onDragStart?.(e); // Trigger drag
    }
  };

  const handleMouseUp = () => {
    document.removeEventListener('mousemove', handleMouseMove);
    document.removeEventListener('mouseup', handleMouseUp);
  };

  document.addEventListener('mousemove', handleMouseMove);
  document.addEventListener('mouseup', handleMouseUp);
};
```

#### HTML5 Drag for Chat Panel Context (`src/components/file-list/FileRow.tsx:118-129`)

```typescript
// Custom MIME types for internal drag data
const handleDragStartHTML5 = (e: React.DragEvent) => {
  e.dataTransfer.setData('sentinel/path', entry.path);
  e.dataTransfer.setData('sentinel/type', entry.isDirectory ? 'folder' : 'file');
  e.dataTransfer.setData('sentinel/name', entry.name);
  e.dataTransfer.setData('sentinel/size', String(entry.size || 0));
  if (entry.mimeType) {
    e.dataTransfer.setData('sentinel/mime', entry.mimeType);
  }
  e.dataTransfer.effectAllowed = 'copyLink';
};
```

### Cycle Detection (`src/lib/cycle-detection.ts`)

#### Multi-Item Cycle Detection (Lines 56-77)

```typescript
export function wouldCreateCycleMulti(
  sourcePaths: string[],
  targetPath: string
): boolean {
  const target = normalizePath(targetPath);

  for (const source of sourcePaths) {
    const normalizedSource = normalizePath(source);

    // Target is one of the dragged items
    if (normalizedSource === target) {
      return true;
    }

    // Target is descendant of source (e.g., moving /a/b into /a/b/c)
    if (target.startsWith(normalizedSource + '/')) {
      return true;
    }
  }

  return false;
}
```

#### Get Specific Cycle Reason (Lines 86-112)

```typescript
export function getCycleReason(
  sourcePaths: string[],
  targetPath: string
): 'cycle_self' | 'cycle_descendant' | 'target_selected' | undefined {
  const target = normalizePath(targetPath);

  for (const source of sourcePaths) {
    const normalizedSource = normalizePath(source);

    if (normalizedSource === target) {
      return 'target_selected';
    }

    if (target.startsWith(normalizedSource + '/')) {
      return 'cycle_descendant';
    }
  }

  return undefined;
}
```

---

## Backend Implementation

### Drag-Drop Validation (`src-tauri/src/commands/filesystem.rs:492-552`)

```rust
#[tauri::command]
pub async fn validate_drag_drop(
    sources: Vec<String>,
    target: String,
) -> Result<(), DragDropError> {
    let target_path = Path::new(&target);

    // 1. Target must exist
    if !target_path.exists() {
        return Err(DragDropError::SourceNotFound { path: target.clone() });
    }

    // 2. Target must be a directory
    if !target_path.is_dir() {
        return Err(DragDropError::TargetNotDirectory { path: target.clone() });
    }

    // 3. Cycle detection (uses canonical paths to resolve symlinks)
    let source_paths: Vec<PathBuf> = sources.iter().map(|s| PathBuf::from(s)).collect();
    let source_refs: Vec<&Path> = source_paths.iter().map(|p| p.as_path()).collect();
    cycle_detection::validate_multi_drop(&source_refs, target_path)?;

    // 4. Validate each source
    for source_path in &source_paths {
        // Source must exist
        if !source_path.exists() {
            return Err(DragDropError::SourceNotFound {
                path: source_path.to_string_lossy().to_string(),
            });
        }

        // Protected path check
        if PathValidator::is_protected_path(source_path) {
            return Err(DragDropError::ProtectedPath {
                path: source_path.to_string_lossy().to_string(),
            });
        }

        // Name collision check
        if let Some(name) = source_path.file_name() {
            let destination = target_path.join(name);
            if destination.exists() {
                return Err(DragDropError::NameCollision {
                    name: name.to_string_lossy().to_string(),
                    destination: target.clone(),
                });
            }
        }
    }

    Ok(())
}
```

### Batch Move Operation (`src-tauri/src/commands/filesystem.rs:554-598`)

```rust
#[tauri::command]
pub async fn move_files_batch(
    sources: Vec<String>,
    target_directory: String,
) -> Result<Vec<String>, DragDropError> {
    // Pre-validate ALL operations before executing ANY
    validate_drag_drop(sources.clone(), target_directory.clone()).await?;

    let target_path = Path::new(&target_directory);
    let mut new_paths = Vec::new();

    for source in &sources {
        let src_path = Path::new(source);
        let file_name = src_path.file_name().ok_or_else(|| DragDropError::IoError {
            message: format!("Invalid source path: {}", source),
        })?;

        let dst_path = target_path.join(file_name);

        // Try rename first (same filesystem), fall back to copy+delete
        if std::fs::rename(src_path, &dst_path).is_err() {
            if src_path.is_dir() {
                copy_dir_all(src_path, &dst_path)?;
                std::fs::remove_dir_all(src_path)?;
            } else {
                std::fs::copy(src_path, &dst_path)?;
                std::fs::remove_file(src_path)?;
            }
        }

        new_paths.push(dst_path.to_string_lossy().to_string());
    }

    Ok(new_paths)
}
```

### Batch Copy Operation (`src-tauri/src/commands/filesystem.rs:600-635`)

```rust
#[tauri::command]
pub async fn copy_files_batch(
    sources: Vec<String>,
    target_directory: String,
) -> Result<Vec<String>, DragDropError> {
    // Same validation as move
    validate_drag_drop(sources.clone(), target_directory.clone()).await?;

    let target_path = Path::new(&target_directory);
    let mut new_paths = Vec::new();

    for source in &sources {
        let src_path = Path::new(source);
        let file_name = src_path.file_name()?;
        let dst_path = target_path.join(file_name);

        if src_path.is_dir() {
            copy_dir_all(src_path, &dst_path)?;
        } else {
            std::fs::copy(src_path, &dst_path)?;
        }

        new_paths.push(dst_path.to_string_lossy().to_string());
    }

    Ok(new_paths)
}
```

### Backend Cycle Detection (`src-tauri/src/security/cycle_detection.rs`)

#### With Canonical Paths (Lines 65-89)

```rust
pub fn would_create_cycle(source: &Path, target: &Path) -> Result<(), CycleError> {
    // Canonicalize to resolve symlinks and normalize paths
    let source_canonical = source.canonicalize()
        .map_err(|_| CycleError::SourceNotFound(source.to_path_buf()))?;
    let target_canonical = target.canonicalize()
        .map_err(|_| CycleError::TargetNotFound(target.to_path_buf()))?;

    // Same path (dropping into itself)
    if source_canonical == target_canonical {
        return Err(CycleError::SameDirectory(source_canonical));
    }

    // Target is descendant of source
    if target_canonical.starts_with(&source_canonical) {
        return Err(CycleError::TargetIsDescendant {
            source: source_canonical,
            target: target_canonical,
        });
    }

    Ok(())
}
```

### Protected Paths (`src-tauri/src/security/mod.rs:25-72`)

```rust
pub fn is_protected_path(path: &Path) -> bool {
    let protected_paths = vec![
        // macOS/Linux
        PathBuf::from("/"),
        PathBuf::from("/System"),
        PathBuf::from("/usr"),
        PathBuf::from("/bin"),
        PathBuf::from("/sbin"),
        PathBuf::from("/Library"),
        PathBuf::from("/Applications"),
        PathBuf::from("/private"),
        PathBuf::from("/var"),
        // Windows
        PathBuf::from("C:\\Windows"),
        PathBuf::from("C:\\Program Files"),
        PathBuf::from("C:\\Program Files (x86)"),
    ];

    let check_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

    for protected in &protected_paths {
        if check_path == *protected {
            return true;
        }
        if check_path.starts_with(protected) {
            // Allow user directories within home
            if let Some(home) = dirs::home_dir() {
                if check_path.starts_with(&home) {
                    return false;
                }
            }
            // Block direct children of protected paths
            if check_path.parent() == Some(protected) {
                return true;
            }
        }
    }

    // Block home directory itself (but not subdirectories)
    if let Some(home) = dirs::home_dir() {
        if check_path == home {
            return true;
        }
    }

    false
}
```

---

## Visual Feedback System

### Drag Preview Component (`src/components/drag-drop/DragPreview.tsx:9-52`)

```typescript
export function DragPreview({ dragState }: DragPreviewProps) {
  const { items, position, isCopy } = dragState;
  const count = items.length;
  const firstItem = items[0];

  return (
    <div
      className="fixed pointer-events-none z-50 flex items-center gap-2
                 px-3 py-2 rounded-lg shadow-lg
                 bg-white/95 dark:bg-gray-800/95 backdrop-blur-sm
                 border border-gray-200 dark:border-gray-700"
      style={{
        left: position.x + 16,
        top: position.y + 16,
      }}
    >
      {/* File/Folder Icon */}
      {firstItem.isDirectory ? (
        <FolderIcon size={20} />
      ) : (
        <File size={20} className="text-gray-400" />
      )}

      {/* Name or Count */}
      <span className="text-sm max-w-48 truncate">
        {count === 1 ? firstItem.name : `${count} items`}
      </span>

      {/* Count Badge (multi-select) */}
      {count > 1 && (
        <span className="flex items-center justify-center min-w-5 h-5 px-1.5
                        rounded-full bg-orange-500 text-white text-xs font-medium">
          {count}
        </span>
      )}

      {/* Copy Mode Indicator */}
      {isCopy && (
        <span className="text-xs text-green-600 dark:text-green-400 font-medium ml-1">
          + Copy
        </span>
      )}
    </div>
  );
}
```

### Drop Target Highlighting (Tailwind Classes)

```typescript
// FileRow.tsx:156-167, FileGridView.tsx:165-172
className={cn(
  // Base styles
  'group relative cursor-default select-none transition-colors duration-75',

  // Selection state
  isSelected && 'bg-orange-500/20',

  // Hover state (when not dragging)
  !isSelected && !isDragTarget && 'hover:bg-gray-500/10',

  // VALID drop target: orange ring + background
  isDragTarget && isValidDropTarget && 'ring-2 ring-orange-500 bg-orange-500/10',

  // INVALID drop target: red ring + background
  isDragTarget && !isValidDropTarget && 'ring-2 ring-red-500 bg-red-500/10',
)}
```

### Ghost Animations (`src/components/ghost/GhostAnimations.css`)

#### Source File (Being Moved Away)

```css
/* Lines 30-61 */
.ghost-source {
  position: relative;
}

.ghost-source .truncate {
  text-decoration: line-through;
  text-decoration-color: rgba(251, 191, 36, 0.6);
}

.ghost-source::after {
  content: '';
  position: absolute;
  inset: 0;
  background: linear-gradient(90deg, transparent 0%, rgba(251, 191, 36, 0.05) 50%, transparent 100%);
  animation: ghost-source-pulse 2s ease-in-out infinite;
}

@keyframes ghost-source-pulse {
  0%, 100% { opacity: 0.3; }
  50% { opacity: 0.7; }
}
```

#### Destination (Ghost at Target)

```css
/* Lines 6-25 */
.ghost-destination {
  opacity: 0.85;
  background: linear-gradient(135deg, rgba(34, 197, 94, 0.08) 0%, rgba(34, 197, 94, 0.04) 100%);
  border: 1px dashed rgba(34, 197, 94, 0.4);
  border-radius: 6px;
}

.ghost-destination::before {
  content: '';
  position: absolute;
  inset: 0;
  box-shadow: inset 0 0 12px rgba(34, 197, 94, 0.15);
}
```

#### Completion Animation (Materializing)

```css
/* Lines 118-138 */
.ghost-complete-animation {
  animation: ghost-materialize 0.4s ease-out forwards;
}

@keyframes ghost-materialize {
  0% {
    opacity: 0.7;
    transform: scale(0.98);
    box-shadow: 0 0 16px rgba(34, 197, 94, 0.5);
    background: rgba(34, 197, 94, 0.1);
  }
  100% {
    opacity: 1;
    transform: scale(1);
    box-shadow: none;
    background: transparent;
  }
}
```

---

## Data Flow & Event Handling

### Complete Drag-Drop Sequence

```
1. USER CLICKS FILE
   └── FileRow.handleMouseDown() registers mousemove/mouseup listeners

2. USER MOVES MOUSE 5+ PIXELS
   └── Threshold exceeded → onDragStart callback fires
   └── FileGridView.handleDragStart() called
       └── Checks if item in selectedPaths
           ├── YES: Drag ALL selected items
           └── NO: Drag only clicked item
       └── startFileDrag(items, currentPath)

3. DRAG STATE ACTIVATED
   └── useDragDrop sets dragState
   └── DragDropProvider renders <DragPreview />
   └── Global event listeners attached

4. WHILE DRAGGING
   └── mousemove → updateDragPosition(x, y, altKey)
       └── Updates preview position
       └── Toggles isCopy based on Alt key
   └── Alt keydown/keyup → toggles copy mode
   └── Escape → cancelDrag()

5. USER HOVERS OVER FOLDER
   └── FileRow.handleMouseEnter() fires
   └── FileGridView.handleDragEnter(entry) called
       └── Checks entry.isDirectory && isDragDropActive
       └── setDropTarget(path, isDirectory=true)
           └── FRONTEND VALIDATION (sync)
               ├── Not directory? → invalid
               ├── wouldCreateCycleMulti? → invalid
               └── Valid → setDropTargetState({ path, isValid: true })
           └── BACKEND VALIDATION (async)
               └── invoke('validate_drag_drop', { sources, target })
                   └── On error → update dropTarget.reason

6. USER RELEASES MOUSE ON FOLDER
   └── FileRow.handleMouseUp() fires
   └── FileGridView.handleDrop(entry) called
       └── setDropTarget(path, true) // final confirmation
       └── executeDrop()
           └── Check dragState && dropTarget.isValid
           └── Choose command: move_files_batch or copy_files_batch
           └── invoke(command, { sources, targetDirectory })
               └── Backend validates, executes, returns new paths
           └── onDropComplete(newPaths, isCopy)
               └── invalidateQueries(['directory'])
           └── Clear dragState and dropTarget

7. USER RELEASES ON BLANK SPACE
   └── Global mouseup handler fires
   └── No folder caught the drop
   └── cancelDrag() → clears state
   └── onDragCancel() → clearSelection()
```

### Event Listener Summary

| Event | Handler | Purpose |
|-------|---------|---------|
| `mousedown` | FileRow/GridItem | Start tracking for drag threshold |
| `mousemove` (local) | FileRow/GridItem | Detect 5px threshold, initiate drag |
| `mousemove` (global) | useDragDrop | Update drag position, track Alt key |
| `mouseenter` | FileRow/GridItem | Detect folder hover, set drop target |
| `mouseup` (local) | FileRow/GridItem | Execute drop on valid folder |
| `mouseup` (global) | useDragDrop | Cancel drag if dropped on blank space |
| `keydown` | useDragDrop | Escape to cancel, Alt for copy mode |
| `keyup` | useDragDrop | Alt release exits copy mode |
| `blur` | useDragDrop | Cancel drag on window blur |

---

## Validation & Error Handling

### Frontend Validation (Synchronous)

| Check | Implementation | File |
|-------|----------------|------|
| Is directory | `!isDirectory` → 'not_directory' | useDragDrop.ts:79-82 |
| Self-drop | `source === target` | cycle-detection.ts:34-36 |
| Descendant drop | `target.startsWith(source + '/')` | cycle-detection.ts:38-40 |
| Target is selected | Check if target in dragged items | cycle-detection.ts:96-98 |

### Backend Validation (Asynchronous)

| Check | Error Type | File |
|-------|------------|------|
| Target exists | `SOURCE_NOT_FOUND` | filesystem.rs:501-504 |
| Target is directory | `TARGET_NOT_DIRECTORY` | filesystem.rs:507-510 |
| Cycle detection | `CYCLE_DETECTED_*` | cycle_detection.rs:65-89 |
| Source exists | `SOURCE_NOT_FOUND` | filesystem.rs:525-528 |
| Protected path | `PROTECTED_PATH` | filesystem.rs:532-536 |
| Name collision | `NAME_COLLISION` | filesystem.rs:539-547 |

### User-Facing Error Messages (`src/types/drag-drop.ts:32-40`)

```typescript
export const DROP_INVALID_MESSAGES: Record<DropInvalidReason, string> = {
  cycle_self: 'Cannot drop folder into itself',
  cycle_descendant: 'Cannot drop folder into its own subfolder',
  target_selected: 'Cannot drop into a selected item',
  name_collision: 'Item with same name already exists',
  permission_denied: 'Permission denied',
  not_directory: 'Can only drop into folders',
  protected_path: 'Cannot modify protected folder',
};
```

---

## Key Files Reference

### Frontend Files

| File | Purpose |
|------|---------|
| `src/types/drag-drop.ts` | Type definitions for drag state, drop target, errors |
| `src/hooks/useDragDrop.ts` | Core hook managing drag-drop state and operations |
| `src/components/drag-drop/DragDropProvider.tsx` | Context provider, renders DragPreview |
| `src/components/drag-drop/DragPreview.tsx` | Visual preview following cursor |
| `src/components/file-list/FileGridView.tsx` | Grid view with drag handlers |
| `src/components/file-list/FileListView.tsx` | List view with drag handlers |
| `src/components/file-list/FileRow.tsx` | Individual row with threshold detection |
| `src/lib/cycle-detection.ts` | Frontend path cycle detection |
| `src/stores/selection-store.ts` | Tracks multi-file selection |
| `src/hooks/useMarqueeSelection.ts` | Rectangle selection for multi-select |
| `src/components/ghost/GhostAnimations.css` | Visual animations for operations |

### Backend Files

| File | Purpose |
|------|---------|
| `src-tauri/src/commands/filesystem.rs` | Tauri commands for validate/move/copy |
| `src-tauri/src/security/cycle_detection.rs` | Backend canonical path cycle detection |
| `src-tauri/src/security/mod.rs` | Protected path validation |
| `src-tauri/src/execution/executor.rs` | WAL-based execution with conflict policies |

### External File Drop

| File | Purpose |
|------|---------|
| `src/hooks/useExternalFileDrop.ts` | Handles native file drops from Finder/Explorer |
| `src/hooks/useChatDropZone.ts` | Combined internal/external drop for chat context |
| `src/components/ChatPanel/DropZoneOverlay.tsx` | Visual overlay for chat drop zone |

---

## Key Design Decisions

1. **5px Drag Threshold**: Prevents accidental drags on clicks. User must move 5 pixels to initiate drag.

2. **Two-Stage Validation**: Frontend provides instant feedback (cycles); backend handles edge cases (symlinks, permissions).

3. **Multi-Selection Drag**: If dragged item is selected, ALL selected items are dragged together.

4. **Copy Mode (Alt Key)**: Dynamically toggle between move and copy during drag operation.

5. **Batch Operations**: Pre-validate ALL operations before executing ANY (atomic validation).

6. **Cross-Filesystem Support**: `fs::rename()` first, fall back to `copy+delete` if rename fails.

7. **Canonical Path Resolution**: Backend uses `canonicalize()` to resolve symlinks and normalize paths.

8. **Protected Paths**: System directories and home root are protected from all operations.

9. **Custom MIME Types**: `sentinel/*` prefix for internal drag data to distinguish from external drops.

10. **Ghost Animations**: Visual feedback showing source (strikethrough), destination (green dashed), and completion (materialize) states.
