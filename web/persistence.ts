/**
 * OPFS persistence helpers for VFS state.
 */

const OPFS_FILE = 'vfs_state.json';

export async function loadFromOPFS(): Promise<string> {
  try {
    const root = await navigator.storage.getDirectory();
    const fileHandle = await root.getFileHandle(OPFS_FILE);
    const file = await fileHandle.getFile();
    return await file.text();
  } catch {
    return '';
  }
}

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
