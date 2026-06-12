/**
 * Host filesystem adapter for mounting real directories into NexOS.
 *
 * Uses the File System Access API (`showDirectoryPicker`) to let users
 * select a real directory from their machine. The contents are pre-cached
 * so that the synchronous WASM side can read them without async calls.
 * Writes are queued and flushed after each command execution.
 */

/** A single directory entry returned by list_dir. */
interface HostEntry {
  name: string;
  is_dir: boolean;
  size: number;
}

/** Callback functions registered with WASM for a single mount. */
interface HostFsCallbacks {
  list_dir: (host_path: string) => string;
  read_file: (host_path: string) => string;
  read_file_lines: (host_path: string, start: number, count: number) => string;
  file_line_count: (host_path: string) => string;
  write_file: (host_path: string, content: string) => string;
  append_file: (host_path: string, content: string) => string;
  mkdir: (host_path: string) => string;
  touch: (host_path: string) => string;
  rm: (host_path: string) => string;
  rm_recursive: (host_path: string) => string;
  file_size: (host_path: string) => string;
  exists: (host_path: string) => string;
  is_dir: (host_path: string) => string;
}

/** Internal state for a single mounted directory. */
interface MountEntry {
  handle: FileSystemDirectoryHandle;
  hostRoot: string;
  vfsMountPath: string;
  mountId: string;
  /** Cache keyed by "type:path" (e.g. "dir:src", "file:src/main.rs"). */
  cache: Map<string, string>;
}

/**
 * Manages mounted host directories and provides synchronous callbacks
 * for the WASM side to access them.
 */
export class HostFsManager {
  private mounts: Map<string, MountEntry> = new Map();
  private pendingWrites: Array<Promise<void>> = [];
  private nextMountId = 0;

  /**
   * Mount a host directory selected via `showDirectoryPicker`.
   *
   * Recursively reads the directory tree and populates a cache so that
   * WASM can read files synchronously.
   */
  async mount(
    handle: FileSystemDirectoryHandle,
    vfsMountPath: string,
  ): Promise<string> {
    const mountId = `host_${this.nextMountId++}`;
    const entry: MountEntry = {
      handle,
      hostRoot: handle.name,
      vfsMountPath,
      mountId,
      cache: new Map(),
    };
    await this.populateCache(entry, '');
    this.mounts.set(mountId, entry);
    return mountId;
  }

  /**
   * Re-mount using a previously persisted mount path.
   *
   * Opens the directory picker for the user to re-authorize access.
   * Returns the mount ID on success.
   */
  async remount(
    handle: FileSystemDirectoryHandle,
    vfsMountPath: string,
  ): Promise<string> {
    return this.mount(handle, vfsMountPath);
  }

  /** Get the set of currently registered mount IDs. */
  getRegisteredMountIds(): Set<string> {
    return new Set(this.mounts.keys());
  }

  /** Get the mount entry for a given ID. */
  getMount(mountId: string): MountEntry | undefined {
    return this.mounts.get(mountId);
  }

  /** Get all mount entries. */
  getAllMounts(): Map<string, MountEntry> {
    return this.mounts;
  }

  /**
   * Build the synchronous callbacks object for a mount.
   *
   * These functions read from the pre-populated cache. Writes queue
   * async operations and update the cache immediately.
   */
  getCallbacks(mountId: string): HostFsCallbacks | null {
    const entry = this.mounts.get(mountId);
    if (!entry) return null;

    const resolve = (path: string): FileSystemDirectoryHandle | null => {
      if (!path) return entry.handle;
      const parts = path.split('/').filter(Boolean);
      // We can't synchronously resolve subdirectories, but the cache
      // should already have everything pre-populated.
      return null;
    };

    return {
      list_dir: (path: string): string => {
        const cached = entry.cache.get(`dir:${path}`);
        if (cached !== undefined) return cached;
        // Return empty list if not cached
        return '[]';
      },

      read_file: (path: string): string => {
        const cached = entry.cache.get(`file:${path}`);
        if (cached !== undefined) return cached;
        return '';
      },

      read_file_lines: (path: string, start: number, count: number): string => {
        const content = entry.cache.get(`file:${path}`);
        if (content === undefined) return '';
        const lines = content.split('\n');
        const end = Math.min(start + count, lines.length);
        if (start >= lines.length) return '';
        return lines.slice(start, end).join('\n');
      },

      file_line_count: (path: string): string => {
        const content = entry.cache.get(`file:${path}`);
        if (content === undefined) return '0';
        if (content === '') return '0';
        return String(content.split('\n').length);
      },

      write_file: (path: string, content: string): string => {
        // Update cache immediately
        entry.cache.set(`file:${path}`, content);
        // Ensure parent directory cache is updated
        this.ensureDirCacheEntry(entry, path);
        // Queue async write
        this.pendingWrites.push(this.doWrite(entry, path, content));
        return '';
      },

      append_file: (path: string, content: string): string => {
        const existing = entry.cache.get(`file:${path}`) || '';
        const newContent = existing + content;
        entry.cache.set(`file:${path}`, newContent);
        this.ensureDirCacheEntry(entry, path);
        this.pendingWrites.push(this.doWrite(entry, path, newContent));
        return '';
      },

      mkdir: (path: string): string => {
        // Add to dir cache
        entry.cache.set(`dir:${path}`, '[]');
        // Also add to parent dir listing
        this.ensureDirCacheEntry(entry, path + '/placeholder');
        this.pendingWrites.push(this.doMkdir(entry, path));
        return '';
      },

      touch: (path: string): string => {
        if (!entry.cache.has(`file:${path}`)) {
          entry.cache.set(`file:${path}`, '');
          this.ensureDirCacheEntry(entry, path);
          this.pendingWrites.push(this.doWrite(entry, path, ''));
        }
        return '';
      },

      rm: (path: string): string => {
        entry.cache.delete(`file:${path}`);
        entry.cache.delete(`dir:${path}`);
        this.removeFromDirCache(entry, path);
        this.pendingWrites.push(this.doRm(entry, path));
        return '';
      },

      rm_recursive: (path: string): string => {
        // Remove from cache
        const prefix = path ? `${path}/` : '';
        const keysToDelete: string[] = [];
        for (const key of entry.cache.keys()) {
          const k = key.substring(key.indexOf(':') + 1);
          if (k === path || k.startsWith(prefix)) {
            keysToDelete.push(key);
          }
        }
        keysToDelete.forEach((k) => entry.cache.delete(k));
        this.removeFromDirCache(entry, path);
        this.pendingWrites.push(this.doRmRecursive(entry, path));
        return '';
      },

      file_size: (path: string): string => {
        const content = entry.cache.get(`file:${path}`);
        if (content === undefined) return '0';
        return String(new TextEncoder().encode(content).length);
      },

      exists: (path: string): string => {
        return String(
          entry.cache.has(`file:${path}`) || entry.cache.has(`dir:${path}`),
        );
      },

      is_dir: (path: string): string => {
        return String(entry.cache.has(`dir:${path}`));
      },
    };
  }

  /** Flush all pending async writes. Call after each command execution. */
  async flushWrites(): Promise<void> {
    const writes = this.pendingWrites;
    this.pendingWrites = [];
    await Promise.all(writes);
  }

  /** Unmount a directory and remove its cache. */
  unmount(mountId: string): void {
    this.mounts.delete(mountId);
  }

  // ---- Private helpers ---------------------------------------------------

  /** Recursively populate the cache for a host directory. */
  private async populateCache(
    entry: MountEntry,
    relativePath: string,
  ): Promise<void> {
    let dirHandle = entry.handle;
    if (relativePath) {
      const parts = relativePath.split('/').filter(Boolean);
      for (const part of parts) {
        try {
          dirHandle = await dirHandle.getDirectoryHandle(part);
        } catch {
          return; // Path doesn't exist
        }
      }
    }

    const entries: HostEntry[] = [];
    // @ts-expect-error: entries() is async iterable
    for await (const [name, childHandle] of dirHandle.entries()) {
      const childPath = relativePath ? `${relativePath}/${name}` : name;
      if (childHandle.kind === 'file') {
        try {
          const file = await (childHandle as FileSystemFileHandle).getFile();
          const text = await file.text();
          entry.cache.set(`file:${childPath}`, text);
          entries.push({ name, is_dir: false, size: file.size });
        } catch {
          // Skip unreadable files
          entries.push({ name, is_dir: false, size: 0 });
        }
      } else {
        entries.push({ name, is_dir: true, size: 0 });
        entry.cache.set(`dir:${childPath}`, '[]');
        await this.populateCache(entry, childPath);
      }
    }
    entry.cache.set(`dir:${relativePath}`, JSON.stringify(entries));
  }

  /** Ensure a directory cache entry includes a child name. */
  private ensureDirCacheEntry(entry: MountEntry, childPath: string): void {
    const lastSlash = childPath.lastIndexOf('/');
    const dirPath = lastSlash > 0 ? childPath.substring(0, lastSlash) : '';
    const childName = lastSlash >= 0 ? childPath.substring(lastSlash + 1) : childPath;

    const cached = entry.cache.get(`dir:${dirPath}`);
    let entries: HostEntry[];
    if (cached !== undefined) {
      try {
        entries = JSON.parse(cached);
      } catch {
        entries = [];
      }
    } else {
      entries = [];
    }

    if (!entries.some((e) => e.name === childName)) {
      entries.push({ name: childName, is_dir: false, size: 0 });
      entry.cache.set(`dir:${dirPath}`, JSON.stringify(entries));
    }
  }

  /** Remove a child name from its parent's directory cache entry. */
  private removeFromDirCache(entry: MountEntry, childPath: string): void {
    const lastSlash = childPath.lastIndexOf('/');
    const dirPath = lastSlash > 0 ? childPath.substring(0, lastSlash) : '';
    const childName = lastSlash >= 0 ? childPath.substring(lastSlash + 1) : childPath;

    const cached = entry.cache.get(`dir:${dirPath}`);
    if (cached === undefined) return;
    try {
      const entries: HostEntry[] = JSON.parse(cached);
      const filtered = entries.filter((e) => e.name !== childName);
      entry.cache.set(`dir:${dirPath}`, JSON.stringify(filtered));
    } catch {
      // Ignore parse errors
    }
  }

  /** Write file content to the real host filesystem. */
  private async doWrite(
    entry: MountEntry,
    path: string,
    content: string,
  ): Promise<void> {
    const parts = path.split('/').filter(Boolean);
    const fileName = parts.pop()!;
    let dir = entry.handle;
    for (const part of parts) {
      dir = await dir.getDirectoryHandle(part, { create: true });
    }
    const fh = await dir.getFileHandle(fileName, { create: true });
    const writable = await fh.createWritable();
    await writable.write(content);
    await writable.close();
  }

  /** Create a directory on the real host filesystem. */
  private async doMkdir(
    entry: MountEntry,
    path: string,
  ): Promise<void> {
    const parts = path.split('/').filter(Boolean);
    let dir = entry.handle;
    for (const part of parts) {
      dir = await dir.getDirectoryHandle(part, { create: true });
    }
  }

  /** Remove a file or empty directory from the real host filesystem. */
  private async doRm(entry: MountEntry, path: string): Promise<void> {
    const parts = path.split('/').filter(Boolean);
    const name = parts.pop()!;
    let dir = entry.handle;
    for (const part of parts) {
      try {
        dir = await dir.getDirectoryHandle(part);
      } catch {
        return;
      }
    }
    try {
      await dir.removeEntry(name);
    } catch {
      // May fail if directory is non-empty or entry doesn't exist
    }
  }

  /** Remove a directory recursively from the real host filesystem. */
  private async doRmRecursive(entry: MountEntry, path: string): Promise<void> {
    const parts = path.split('/').filter(Boolean);
    const name = parts.pop()!;
    let dir = entry.handle;
    for (const part of parts) {
      try {
        dir = await dir.getDirectoryHandle(part);
      } catch {
        return;
      }
    }
    try {
      await dir.removeEntry(name, { recursive: true });
    } catch {
      // Ignore errors
    }
  }
}
