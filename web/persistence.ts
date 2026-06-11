/**
 * OPFS persistence helpers for VFS state.
 *
 * The Origin Private File System (OPFS) is a storage API built into modern
 * browsers that provides per-origin file access without user-visible file
 * pickers.  NexOS uses it to save and restore the virtual file system
 * across page reloads.
 *
 * @module persistence
 */

/** Filename used inside OPFS to store the serialised VFS JSON. */
const OPFS_FILE = 'vfs_state.json';

/**
 * Load the VFS state from OPFS.
 *
 * Returns the raw JSON string if the file exists, or an empty string if
 * the file is missing or OPFS is unavailable (e.g. first visit).
 */
export async function loadFromOPFS(): Promise<string> {
  try {
    const root = await navigator.storage.getDirectory();
    const fileHandle = await root.getFileHandle(OPFS_FILE);
    const file = await fileHandle.getFile();
    return await file.text();
  } catch {
    // File does not exist yet — return empty so the shell creates a fresh VFS.
    return '';
  }
}

/**
 * Persist the VFS state to OPFS.
 *
 * Creates the file if it does not exist; overwrites it otherwise.
 * Errors are logged to the console but do not propagate — a failed save
 * should not interrupt the user's workflow.
 *
 * @param data - The JSON string produced by `get_state_json()` in WASM.
 */
export async function saveToOPFS(data: string): Promise<void> {
  try {
    const root = await navigator.storage.getDirectory();
    const fileHandle = await root.getFileHandle(OPFS_FILE, { create: true });
    const writable = await fileHandle.createWritable();
    await writable.write(data);
    await writable.close();
  } catch (e) {
    console.warn('[NexOS] OPFS save failed:', e);
  }
}
