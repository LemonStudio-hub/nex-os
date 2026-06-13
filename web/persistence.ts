/**
 * OPFS persistence helpers for VFS state — incremental storage.
 *
 * The Origin Private File System (OPFS) is a storage API built into modern
 * browsers that provides per-origin file access without user-visible file
 * pickers.  NexOS uses it to save and restore the virtual file system
 * across page reloads.
 *
 * ## Storage layout
 *
 * ```
 * OPFS root/
 *   nexos_tree.json          — VFS tree structure (directories + empty files)
 *   nexos_files/
 *     <base64url_encoded_path> — individual file content (plain text)
 *   vfs_state.json           — LEGACY (read as fallback, not actively written)
 *   user_config.json         — auth config (unchanged)
 * ```
 *
 * On each save only **dirty** files are written.  Deleted files are removed
 * from `nexos_files/`.  The tree skeleton is saved when the directory
 * structure changes.
 *
 * @module persistence
 */

/** Filename for the tree skeleton (directories + file metadata). */
const TREE_FILE = 'nexos_tree.json';
/** Directory name for individual file contents. */
const FILES_DIR = 'nexos_files';
/** Legacy monolithic state file (read-only fallback). */
const LEGACY_FILE = 'vfs_state.json';
/** Filename for non-VFS shell state (history, env_vars, hostname). */
const STATE_FILE = 'nexos_state.json';

/**
 * Encode an absolute VFS path into a safe OPFS filename.
 *
 * Uses `encodeURIComponent` + base64, with `+/=` replaced to be
 * filesystem-safe.
 */
function encodePath(path: string): string {
  // encodeURIComponent handles all special chars, then base64-encode.
  // Replace characters that are problematic in filenames.
  return btoa(encodeURIComponent(path))
    .replace(/\+/g, '-')
    .replace(/\//g, '_')
    .replace(/=+$/, '');
}

/**
 * Decode an OPFS filename back to the original VFS path.
 */
function decodePath(encoded: string): string {
  // Restore base64 characters.
  let b64 = encoded.replace(/-/g, '+').replace(/_/g, '/');
  // Add padding if needed.
  while (b64.length % 4 !== 0) {
    b64 += '=';
  }
  return decodeURIComponent(atob(b64));
}

/**
 * Walk a parsed VFS tree object and locate the FileNode matching `path`.
 *
 * `path` must be an absolute path like `/home/user/file.txt`.
 * Returns the node object (with `name` and `content` fields) or `null`.
 */
function findFileNode(
  tree: Record<string, unknown>,
  path: string,
): Record<string, unknown> | null {
  const root = tree.root as Record<string, unknown> | undefined;
  if (!root) return null;

  const parts = path.split('/').filter(Boolean);
  let current: Record<string, unknown> = root;

  for (let i = 0; i < parts.length; i++) {
    const children = current.children as
      | Record<string, Record<string, unknown>>
      | undefined;
    if (!children) return null;

    const child = children[parts[i]];
    if (!child) return null;

    // The serde enum serialises as `{"File": {...}}` or `{"Directory": {...}}`.
    if ('File' in child) {
      const fileNode = child.File as Record<string, unknown>;
      if (i === parts.length - 1) {
        return fileNode;
      }
      // Cannot traverse through a file.
      return null;
    } else if ('Directory' in child) {
      current = child.Directory as Record<string, unknown>;
    } else {
      return null;
    }
  }

  return null;
}

/**
 * Minimal WASM API surface needed by the persistence layer.
 */
interface PersistenceWasmApi {
  get_dirty_files_json(state: string): string;
  get_deleted_files_json(state: string): string;
  get_file_content(state: string, path: string): string;
  get_tree_json(state: string): string;
  mark_all_dirty(state: string): string;
  get_state_dirty_flags(state: string): string;
  mark_state_clean(state: string): string;
  get_non_vfs_state_json(state: string): string;
  apply_saved_state(state: string, savedState: string): string;
}

/**
 * Load the non-VFS state file from OPFS.
 * Returns an empty string if the file does not exist.
 */
async function loadStateFile(
  root: FileSystemDirectoryHandle,
): Promise<string> {
  try {
    const handle = await root.getFileHandle(STATE_FILE);
    const file = await handle.getFile();
    return await file.text();
  } catch {
    return '';
  }
}

/**
 * Load the VFS state from OPFS.
 *
 * Tries the new incremental format first (`nexos_tree.json` +
 * `nexos_files/`).  Falls back to the legacy `vfs_state.json` if the
 * new format is not found.  Also loads non-VFS state from
 * `nexos_state.json` if available.
 *
 * @returns `{ json, stateJson, isNewFormat }` — the VFS JSON string,
 *   the non-VFS state JSON string, and whether it came from the new
 *   format (used for migration detection).
 */
export async function loadFromOPFS(): Promise<{
  json: string;
  stateJson: string;
  isNewFormat: boolean;
}> {
  try {
    const root = await navigator.storage.getDirectory();

    // --- Try new incremental format ---
    try {
      const treeHandle = await root.getFileHandle(TREE_FILE);
      const treeFile = await treeHandle.getFile();
      const treeJson = await treeFile.text();

      // Merge individual file contents back into the tree.
      const tree = JSON.parse(treeJson);
      const filesDir = await root.getDirectoryHandle(FILES_DIR);

      // Iterate over stored files and inject their content into the tree.
      // @ts-expect-error — entries() is available on FileSystemDirectoryHandle
      for await (const [name, handle] of filesDir.entries()) {
        if (handle.kind !== 'file') continue;
        try {
          const file = await (handle as FileSystemFileHandle).getFile();
          const content = await file.text();
          const path = decodePath(name);
          const node = findFileNode(tree, path);
          if (node) {
            node.content = content;
          }
        } catch {
          // Skip unreadable files.
        }
      }

      // Load non-VFS state (may not exist yet).
      const stateJson = await loadStateFile(root);

      return { json: JSON.stringify(tree), stateJson, isNewFormat: true };
    } catch {
      // New format not found — fall through to legacy.
    }

    // --- Legacy fallback ---
    const fileHandle = await root.getFileHandle(LEGACY_FILE);
    const file = await fileHandle.getFile();
    const json = await file.text();

    // Try loading non-VFS state even with legacy VFS format.
    const stateJson = await loadStateFile(root);

    return { json, stateJson, isNewFormat: false };
  } catch {
    // OPFS unavailable or nothing stored.
    return { json: '', stateJson: '', isNewFormat: false };
  }
}

/**
 * Persist the shell state to OPFS using incremental storage.
 *
 * Only dirty (modified/created) files are written.  Deleted files are
 * removed from `nexos_files/`.  The tree skeleton is saved when there
 * are any changes.  Non-VFS state (history, env_vars, hostname) is
 * saved to `nexos_state.json` when dirty.
 *
 * After a successful save all dirty flags are cleared via
 * `mark_state_clean`.
 *
 * @param stateJson - The full shell state JSON (not just VFS).
 * @param wasm      - The WASM API bindings.
 * @returns The updated state JSON with all dirty flags cleared.
 */
export async function saveToOPFS(
  stateJson: string,
  wasm: PersistenceWasmApi,
): Promise<string> {
  try {
    const root = await navigator.storage.getDirectory();

    // Get dirty and deleted file lists.
    const dirtyFiles: string[] = JSON.parse(
      wasm.get_dirty_files_json(stateJson),
    );
    const deletedFiles: string[] = JSON.parse(
      wasm.get_deleted_files_json(stateJson),
    );

    // Get non-VFS dirty flags.
    const stateDirty: { history?: boolean; env_vars?: boolean } = JSON.parse(
      wasm.get_state_dirty_flags(stateJson),
    );
    const hasStateChanges =
      stateDirty.history === true || stateDirty.env_vars === true;

    // If no changes at all, skip save entirely.
    if (
      dirtyFiles.length === 0 &&
      deletedFiles.length === 0 &&
      !hasStateChanges
    ) {
      return stateJson;
    }

    // Save each dirty file individually.
    if (dirtyFiles.length > 0) {
      const filesDir = await root.getDirectoryHandle(FILES_DIR, {
        create: true,
      });
      for (const path of dirtyFiles) {
        const content = wasm.get_file_content(stateJson, path);
        const encoded = encodePath(path);
        const fh = await filesDir.getFileHandle(encoded, { create: true });
        const writable = await fh.createWritable();
        await writable.write(content);
        await writable.close();
      }
    }

    // Delete removed files.
    for (const path of deletedFiles) {
      try {
        const filesDir = await root.getDirectoryHandle(FILES_DIR);
        const encoded = encodePath(path);
        await filesDir.removeEntry(encoded);
      } catch {
        // File may not exist in OPFS yet — ignore.
      }
    }

    // Save the tree skeleton.
    const treeJson = wasm.get_tree_json(stateJson);
    if (treeJson) {
      const treeFh = await root.getFileHandle(TREE_FILE, { create: true });
      const treeWritable = await treeFh.createWritable();
      await treeWritable.write(treeJson);
      await treeWritable.close();
    }

    // Save non-VFS state if dirty.
    if (hasStateChanges) {
      const nonVfsJson = wasm.get_non_vfs_state_json(stateJson);
      if (nonVfsJson && nonVfsJson !== '{}') {
        const stateFh = await root.getFileHandle(STATE_FILE, {
          create: true,
        });
        const stateWritable = await stateFh.createWritable();
        await stateWritable.write(nonVfsJson);
        await stateWritable.close();
      }
    }

    // Clear all dirty flags after successful save.
    const cleanState = wasm.mark_state_clean(stateJson);
    return cleanState;
  } catch (e) {
    console.warn('[NexOS] OPFS save failed:', e);
    return stateJson;
  }
}
