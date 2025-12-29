# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Sentinel is a Tauri v2 desktop file manager application with AI-powered file organization capabilities. It uses React 19 for the frontend and Rust for the backend.

## Common Commands

```bash
# Development (starts both Vite dev server and Tauri)
npm run tauri dev

# Build production app
npm run tauri build

# Frontend only (no Tauri shell)
npm run dev

# Type check
npm run build  # runs tsc && vite build

# Rust checks (from src-tauri/)
cargo check
cargo build
```

## Architecture

### Frontend (src/)
- **React 19 + TypeScript + Vite 7** with TailwindCSS v4
- **State Management**: Zustand stores in `src/stores/`
  - `navigation-store.ts` - Directory navigation, history, view mode, Quick Look
  - `selection-store.ts` - File/folder selection state
  - `organize-store.ts` - AI organization workflow state machine
  - `toast-store.ts` - Toast notifications
- **Data Fetching**: TanStack Query for directory contents
- **Tauri IPC**: Uses `@tauri-apps/api/core` `invoke()` to call Rust commands

### Backend (src-tauri/)
- **Tauri v2** with Rust
- **Module Structure**:
  - `commands/` - Tauri command handlers (filesystem, watcher, ai)
  - `models/` - Data structures shared with frontend (FileEntry, DirectoryContents)
  - `services/` - Background services (file watcher using notify crate)
  - `ai/` - Anthropic API client, credential storage (keyring), prompts
  - `security/` - Path validation to prevent dangerous operations

### Key Tauri Commands
- Filesystem: `read_directory`, `rename_file`, `delete_to_trash`, `move_file`, `copy_file`
- Watcher: `start_downloads_watcher`, `stop_downloads_watcher`, `get_watcher_status`
- AI: `set_api_key`, `get_rename_suggestion`, `generate_organize_plan`, `build_folder_context`

### AI Integration
- Uses Anthropic Claude API for file rename suggestions and folder organization plans
- Credentials stored securely via the `keyring` crate (OS keychain)
- Two-phase organization: Haiku for fast context analysis, Sonnet for plan generation

### Type Sharing
Frontend types in `src/types/file.ts` mirror Rust structs in `src-tauri/src/models/`. When modifying data structures, update both.
