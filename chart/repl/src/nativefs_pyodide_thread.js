export function addNativeFS(pyodide, mainThreadNativeFSHelpers) {
    // We need some help from the main thread via synclink.
    const nativeFSHelpers = {
        readdir(x) {
            return mainThreadNativeFSHelpers.readdir(x).syncify();
        },
        getFileContents(f) {
            let size = mainThreadNativeFSHelpers.getFileSize(f).syncify();
            let buf = new Uint8Array(new SharedArrayBuffer(size));
            mainThreadNativeFSHelpers.getFileContents(f, buf).syncify();
            return buf;
        },
        getFileTimestamp(f) {
            return mainThreadNativeFSHelpers.getFileTimestamp(f).syncify();
        },
        writeToFile(fileHandle, position, data) {
            return mainThreadNativeFSHelpers
                .writeToFile(fileHandle, position, data)
                .syncify();
        },
        truncate(fileHandle, length) {
            return mainThreadNativeFSHelpers
                .truncate(fileHandle, length)
                .syncify();
        },
        touch(fileHandle) {
            return mainThreadNativeFSHelpers.touch(fileHandle).syncify();
        },
        lookup(dirHandle, name) {
            const isDirBuff = new Uint8Array(new SharedArrayBuffer(1));
            const handle = mainThreadNativeFSHelpers.lookup(dirHandle, name, isDirBuff).syncify();
            const isDir = !!isDirBuff[0];
            return [handle, isDir];
        },
    };
    const FS = pyodide.FS;
    const MEMFS = pyodide.FS.filesystems.MEMFS;
    const NATIVEFS = {
        ops_table: null,
        mount: function (mount) {
            console.log('mount??');
            const node = NATIVEFS.createNode(
                null,
                '/',
                0o040000 /* S_IFDIR */ | 0o777 /* permissions */,
                0,
            );
            if (!mount.opts.handle) {
                throw new Error('No handle!');
            }
            node.handle = mount.opts.handle;
            return node;
        },
        createNode: function (parent, name, mode, dev) {
            if (FS.isBlkdev(mode) || FS.isFIFO(mode)) {
                // not supported
                throw new FS.ErrnoError(63 /* EPERM */);
            }
            if (!NATIVEFS.ops_table) {
                NATIVEFS.ops_table = {
                    dir: {
                        node: {
                            getattr: NATIVEFS.node_ops.getattr,
                            setattr: NATIVEFS.node_ops.setattr,
                            lookup: NATIVEFS.node_ops.lookup,
                            mknod: NATIVEFS.node_ops.mknod,
                            rename: NATIVEFS.node_ops.rename,
                            unlink: NATIVEFS.node_ops.unlink,
                            rmdir: NATIVEFS.node_ops.rmdir,
                        },
                        stream: {
                            llseek: NATIVEFS.stream_ops.llseek,
                        },
                    },
                    file: {
                        node: {
                            getattr: NATIVEFS.node_ops.getattr,
                            setattr: NATIVEFS.node_ops.setattr,
                        },
                        stream: {
                            open: NATIVEFS.stream_ops.open,
                            close: NATIVEFS.stream_ops.close,
                            llseek: NATIVEFS.stream_ops.llseek,
                            read: NATIVEFS.stream_ops.read,
                            write: NATIVEFS.stream_ops.write,
                            allocate: NATIVEFS.stream_ops.allocate,
                            mmap: NATIVEFS.stream_ops.mmap,
                            msync: NATIVEFS.stream_ops.msync,
                        },
                    },
                };
            }
            var node = FS.createNode(parent, name, mode, dev);
            if (FS.isDir(node.mode)) {
                node.node_ops = NATIVEFS.ops_table.dir.node;
                node.stream_ops = NATIVEFS.ops_table.dir.stream;
            } else if (FS.isFile(node.mode)) {
                node.node_ops = NATIVEFS.ops_table.file.node;
                node.stream_ops = NATIVEFS.ops_table.file.stream;
                node.usedBytes = 0; // The actual number of bytes used in the typed array, as opposed to contents.length which gives the whole capacity.
                // When the byte data of the file is populated, this will point to either a typed array, or a normal JS array. Typed arrays are preferred
                // for performance, and used by default. However, typed arrays are not resizable like normal JS arrays are, so there is a small disk size
                // penalty involved for appending file writes that continuously grow a file similar to std::vector capacity vs used -scheme.
                node.contents = null;
            } else if (FS.isLink(node.mode)) {
                // not implemented
                throw new FS.ErrnoError(52 /* ENOSYS */);
            } else if (FS.isChrdev(node.mode)) {
                // not implemented
                throw new FS.ErrnoError(52 /* ENOSYS */);
            } else {
                console.warn(node.mode);
                throw new FS.ErrnoError(52 /* ENOSYS */);
            }
            node.timestamp = Date.now();
            // add the new node to the parent
            return node;
        },
        node_ops: {
            getattr: function (node) {
                var attr = {};
                // device numbers reuse inode numbers.
                attr.dev = FS.isChrdev(node.mode) ? node.id : 1;
                attr.ino = node.id;
                attr.mode = node.mode;
                attr.nlink = 1;
                attr.uid = 0;
                attr.gid = 0;
                attr.rdev = node.rdev;
                if (FS.isDir(node.mode)) {
                    attr.size = 4096;
                } else if (FS.isFile(node.mode)) {
                    attr.size = node.usedBytes;
                } else {
                    attr.size = 0;
                }
                attr.atime = new Date(node.timestamp);
                attr.mtime = new Date(node.timestamp);
                attr.ctime = new Date(node.timestamp);
                // NOTE: In our implementation, st_blocks = Math.ceil(st_size/st_blksize),
                //       but this is not required by the standard.
                attr.blksize = 4096;
                attr.blocks = Math.ceil(attr.size / attr.blksize);
                return attr;
            },
            setattr: function (node, attr) {
                if (attr.mode !== undefined) {
                    node.mode = attr.mode;
                }
                if (
                    attr.timestamp !== undefined &&
                    attr.timestamp > node.timestamp
                ) {
                    // We can't arbitrarily modify the timestamp, but we can set
                    // it to now. If the user tries to move the modified time
                    // backwards, leave it alone I guess?
                    node.timestamp = new Date(
                        nativeFSHelpers.touch(node.handle),
                    );
                }
                if (attr.size !== undefined) {
                    MEMFS.resizeFileStorage(node, attr.size);
                    nativeFSHelpers.truncate(node.handle, attr.size);
                }
            },
            lookup: function (parent, name) {
                const [handle, isDir] = nativeFSHelpers.lookup(parent.handle, name);
                if (!handle) {
                    throw FS.genericErrors[44 /* ENOENT */];
                }
                let mode;
                if (isDir) {
                    mode = 0o040000 /*  S_IFDIR */ | 0o777 /* permissions */;
                } else {
                    mode = 0o100000 /* S_IFREG */ | 0o666 /* permissions */;
                }
                const node = NATIVEFS.createNode(parent, name, mode);
                node.handle = handle;
                node.timestamp = new Date(
                    nativeFSHelpers.getFileTimestamp(handle),
                );
                return node;
            },
            mknod: function (parent, name, mode, dev) {
                let node = NATIVEFS.createNode(parent, name, mode, dev);
                if (FS.isDir(mode)) {
                    node.handle = parent.handle
                        .getDirectoryHandle(name, { create: true })
                        .syncify();
                } else {
                    node.handle = parent.handle
                        .getFileHandle(name, { create: true })
                        .syncify();
                }
                return node;
            },
            rename: function (old_node, new_dir, new_name) {
                // not implemented
                throw new FS.ErrnoError(52 /* ENOSYS */);
            },
            unlink: function (parent, name) {
                parent.handle.removeEntry(name, { recursive: true }).syncify();
                parent.timestamp = Date.now();
            },
            rmdir: function (parent, name) {
                parent.handle.removeEntry(name).syncify();
                parent.timestamp = Date.now();
            },
            readdir: function (node) {
                return nativeFSHelpers.readdir(node.handle);
            },
        },
        stream_ops: {
            open: function (stream) {
                stream.node.contents = nativeFSHelpers.getFileContents(
                    stream.node.handle,
                );
                stream.node.usedBytes = stream.node.contents.length;
            },
            read: MEMFS.stream_ops.read,
            write: function (stream, buffer, offset, length, position, canOwn) {
                let data = buffer.subarray(offset, offset + length);
                if (buffer.buffer === pyodide._module.HEAP8.buffer) {
                    data = data.slice();
                }
                try {
                    nativeFSHelpers.writeToFile(
                        stream.node.handle,
                        position,
                        data,
                    );
                } catch (e) {
                    throw new FS.ErrnoError(63 /* EPERM */);
                }
                MEMFS.stream_ops.write(
                    stream,
                    buffer,
                    offset,
                    length,
                    position,
                    canOwn,
                );
                return length;
            },
            llseek: MEMFS.stream_ops.llseek,
            allocate: function (stream, offset, length) {
                MEMFS.expandFileStorage(stream.node, offset + length);
                stream.node.usedBytes = Math.max(
                    stream.node.usedBytes,
                    offset + length,
                );
            },
            mmap: function (stream, address, length, position, prot, flags) {
                if (address !== 0) {
                    // We don't currently support location hints for the address of the mapping
                    throw new FS.ErrnoError(28 /* EINVAL */);
                }
                if (!FS.isFile(stream.node.mode)) {
                    throw new FS.ErrnoError(43 /* ENODEV */);
                }
                const ptr = mmapAlloc(length);
                NATIVEFS.stream_ops.read(stream, HEAP8, ptr, length, position);
                return { ptr, allocated: true };
            },
            msync: function (stream, buffer, offset, length, mmapFlags) {
                if (!FS.isFile(stream.node.mode)) {
                    throw new FS.ErrnoError(43 /* ENODEV */);
                }
                if (mmapFlags & 0x02 /* MAP_PRIVATE */) {
                    // MAP_PRIVATE calls need not to be synced back to underlying fs
                    return 0;
                }
                NATIVEFS.stream_ops.write(
                    stream,
                    buffer,
                    0,
                    length,
                    offset,
                    false,
                );
                return 0;
            },
        },
    };
    pyodide.FS.filesystems.NATIVEFS = NATIVEFS;
}
