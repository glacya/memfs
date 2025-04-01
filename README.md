# MemFS
Implementation of simple in-memory file system in Rust

## Structure
MemFS supports the following system calls.
```
open, close, unlink, read, write, lseek, mkdir, rmdir, chdir
```

Directory structure of MemFS is implemented using tree data structure.
Every directory or file is a node, and a directory can have its children.

Since MemFS is aimed to support thread-safety, every pointer on MemFS tree structure is wrapped with `Arc<T>` and `RwLock<T>`.