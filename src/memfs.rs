use std::{cell::UnsafeCell, collections::{hash_map::Entry, HashMap}, ffi::OsStr, iter::Peekable, path::{Iter, Path}, sync::{Arc, Mutex, RwLock}};
use crate::utils::{Result, OpenFlag, MemFSErr};

pub struct MemFS {
    root: MemFSDirNode,
    file_descriptiors: Arc<RwLock<HashMap<u32, MemFSFileDescriptor>>>,
    file_descriptor_count: Arc<Mutex<u32>>,
}

impl MemFS {
    pub fn new() -> Self {
        Self {
            root: MemFSDirNode::root(),
            file_descriptiors: Arc::new(RwLock::new(HashMap::new())),
            file_descriptor_count: Arc::new(Mutex::new(0)),
        }
    }

    pub fn open(&self, path: &str, flag: Option<OpenFlag>) -> Result<u32> {
        if let Some(inner_flag) = flag {

        }

        let iter = Self::path_str_to_iter(path);
        let item_node = self.root.search_entry_with_path(iter)?;

        let item_guard = item_node.read().or_else(|_| {
            Err(MemFSErr::poisoned_lock())
        })?;

        match &*item_guard {
            MemFSEntry::Directory(_) => return Err(MemFSErr::with_message("Cannot open directory")),
            MemFSEntry::File(_) => {
                let fd = self.allocate_file_descriptor()?;

                let mut guard = self.file_descriptiors.write().or_else(|_| {
                    Err(MemFSErr::poisoned_lock())
                })?;
        
                guard.insert(fd, MemFSFileDescriptor::new(fd, item_node.clone()));

                Ok(fd)
            }
        }
    }

    pub fn close(&self, fd: u32) -> Result<()> {
        let mut guard = self.file_descriptiors.write().or_else(|_| { Err(MemFSErr::poisoned_lock()) })?;

        guard.remove(&fd);

        Ok(())
    }

    pub fn create(&self, path: &str) -> Result<()> {
        let dir_path = Self::get_directory_names_excluding_last_one(path).peekable();
        let dir_node = self.root.search_entry_with_path(dir_path)?;
        let dir_guard = dir_node.write().or_else(|_| { Err(MemFSErr::poisoned_lock() )})?;

        match &*dir_guard {
            MemFSEntry::Directory(dir) => {
                dir.create_new_file(path.split("/").last().expect("Path is unspecified"))
            },
            MemFSEntry::File(_) => Err(MemFSErr::no_such_file_or_directory())
        }
    }

    pub fn remove(&self, path: &str) -> Result<()> {
        let dir_path = Self::get_directory_names_excluding_last_one(path).peekable();
        let dir_node = self.root.search_entry_with_path(dir_path)?;
        let dir_guard = dir_node.write().or_else(|_| {
            Err(MemFSErr::poisoned_lock())
        })?;

        match &*dir_guard {
            MemFSEntry::Directory(dir) => dir.remove_file(path.split("/").last().expect("Path is unspecified")),
            MemFSEntry::File(_) => Err(MemFSErr::no_such_file_or_directory())
        }
    }

    pub fn read(&self, fd: u32, path: &str, size: u32) -> Result<()> {
        Ok(())
    }

    pub fn write(&self, fd: u32, path: &str, size: u32) -> Result<()> {
        Ok(())
    }

    pub fn mkdir(&self, path: &str) -> Result<()> {
        let dir_path = Self::get_directory_names_excluding_last_one(path).peekable();
        let dir_node = self.root.search_entry_with_path(dir_path)?;
        let dir_guard = dir_node.write().or_else(|_| { Err(MemFSErr::poisoned_lock() )})?;

        match &*dir_guard {
            MemFSEntry::Directory(dir) => {
                dir.create_new_directory(path.split("/").last().expect("Path is unspecified"))
            },
            MemFSEntry::File(_) => Err(MemFSErr::no_such_file_or_directory())
        }
    }

    pub fn rmdir(&self, path: &str) -> Result<()> {
        Ok(())
    }

    fn path_str_to_iter(path: &str) -> Peekable<impl Iterator<Item = &OsStr>> {
        Path::new(path).iter().peekable()
    }

    fn get_directory_names_excluding_last_one(path: &str) -> impl Iterator<Item = &OsStr> {
        let path_iter = Path::new(path).iter();
        let iter_count = path.split("/").count();
        
        path_iter.take(iter_count.saturating_sub(1))
    }

    fn allocate_file_descriptor(&self) -> Result<u32> {
        let mut guard = self.file_descriptor_count.lock().or_else(|_| {
            Err(MemFSErr::with_message("Mutex poison error"))
        })?;

        let fd = *guard;
        *guard += 1;

        Ok(fd)
    }
}

pub struct MemFSDirNode {
    pub children: Arc<RwLock<HashMap<String, Arc<RwLock<MemFSEntry>>>>>,
}

impl MemFSDirNode {
    pub fn new() -> Self {
        Self {
            children: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn root() -> Self {
        let inner = MemFSEntry::Directory(Self::new());
        let mut map = HashMap::new();

        map.insert("/".to_string(), Arc::new(RwLock::new(inner)));

        Self {
            children: Arc::new(RwLock::new(map))
        }
    }

    pub fn create_new_file(&self, file_name: &str) -> Result<()> {
        let mut guard = self.children.write().or_else(  |_| {
            Err(MemFSErr::poisoned_lock())
        })?;

        match guard.entry(file_name.to_string()) {
            Entry::Occupied(v) => Err(
                MemFSErr::with_message("A file name already exists")
            ),
            Entry::Vacant(v) => {


                v.insert(Arc::new(RwLock::new(MemFSEntry::File(MemFSFileNode::new()))));
                Ok(())
            }
        }
    }

    pub fn create_new_directory(&self, dir_name: &str) -> Result<()> {
        let mut guard = self.children.write().or_else(  |_| {
            Err(MemFSErr::poisoned_lock())
        })?;

        match guard.entry(dir_name.to_string()) {
            Entry::Occupied(_) => Err(
                MemFSErr::with_message("A directory name already exists")
            ),
            Entry::Vacant(v) => {
                v.insert(Arc::new(RwLock::new(MemFSEntry::Directory(MemFSDirNode::new()))));
                Ok(())
            }
        }
    }

    pub fn remove_file(&self, file_name: &str) -> Result<()> {
        let mut guard = self.children.write().or_else(|_| {
            Err(MemFSErr::poisoned_lock())
        })?;

        guard.remove(file_name);

        Ok(())
    }

    pub fn search_entry_with_path<'a>(&self, mut iter: Peekable<impl Iterator<Item = &'a OsStr>>) -> Result<Arc<RwLock<MemFSEntry>>> {
        let current_elem = iter.next();

        let current_path = current_elem.unwrap().to_str().unwrap();

        let next_elem = iter.peek();

        let guard = self.children.read().or_else(|_| {
            Err(MemFSErr::with_message("RwLock poison error"))
        })?;

        match next_elem {
            Some(_) => {
                match guard.get(current_path) {
                    Some(v) => {
                        let inner_guard = v.read().or_else(|_| {
                            Err(MemFSErr::with_message("RwLock poison error"))
                        })?;

                        match &*inner_guard {
                            MemFSEntry::Directory(dir) => dir.search_entry_with_path(iter),
                            MemFSEntry::File(_) => Err(MemFSErr::with_message("No such file or directory"))
                        }
                    },
                    None => Err(MemFSErr::with_message("No such file or directory"))
                }
            },
            None => {
                // Now at the end of path string. current_elem should be the one you looking for.
                match guard.get(current_path) {
                    Some(v) => Ok(v.clone()),
                    None => Err(MemFSErr::with_message("No such file or directory"))
                }
            }
        }
    }

    pub fn add_child(&self, name: &str, entry: MemFSEntry) -> Result<()> {
        let mut guard = self.children.write().or_else(|_| {
            Err(MemFSErr::with_message("RwLock poison error"))
        })?;

        match guard.entry(name.to_string()) {
            Entry::Occupied(_) => Err(MemFSErr::with_message("Entry already exists")),
            Entry::Vacant(e) => {
                e.insert(Arc::new(RwLock::new(entry)));
                Ok(())
            }
        }
    }
}


pub struct MemFSFileNode {
    pub value: UnsafeCell<Vec<u8>>
}

impl MemFSFileNode {
    pub fn new() -> Self {
        Self {
            value: UnsafeCell::new(vec![])
        }
    }
}

pub enum MemFSEntry {
    Directory(MemFSDirNode),
    File(MemFSFileNode),
}

struct MemFSFileDescriptor {
    number: u32,
    file_pointer: u32,
    entry: Arc<RwLock<MemFSEntry>>,
}

impl MemFSFileDescriptor {
    pub fn new(number: u32, entry: Arc<RwLock<MemFSEntry>>) -> Self {
        Self {
            number,
            file_pointer: 0,
            entry,
        }
    }
}