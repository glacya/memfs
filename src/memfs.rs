use std::{cell::UnsafeCell, collections::{hash_map::Entry, HashMap}, ffi::OsStr, iter::Peekable, path::Path, sync::{Arc, Mutex, RwLock}};
use crate::utils::{MemFSErr, OpenFlag, Result, SeekFlag};

pub struct MemFS {
    root: MemFSDirNode,
    file_descriptiors: Arc<RwLock<HashMap<usize, MemFSFileDescriptor>>>,
    file_descriptor_count: Arc<Mutex<usize>>,
}

unsafe impl Sync for MemFS {}

unsafe impl Send for MemFS {}

impl MemFS {
    pub fn new() -> Self {
        Self {
            root: MemFSDirNode::root(),
            file_descriptiors: Arc::new(RwLock::new(HashMap::new())),
            file_descriptor_count: Arc::new(Mutex::new(0)),
        }
    }

    pub fn open(&self, path: &str, flag: OpenFlag) -> Result<usize> {
        // Check flag. O_RDONLY, O_WRONLY, O_RDWR are the mutually exclusive ones.
        if !flag.check_mode_exclusiveness() {
            return Err(MemFSErr::invalid_value());
        }

        if flag.contains(OpenFlag::O_CREAT) {
            self.create(path)?;
        }

        let iter = Self::path_str_to_iter(path);
        let item_node = self.root.search_entry_with_path(iter)?;

        let item_guard = item_node.read().or_else(|_| {
            Err(MemFSErr::poisoned_lock())
        })?;

        match &*item_guard {
            MemFSEntry::Directory(_) => return Err(MemFSErr::is_directory()),
            MemFSEntry::File(_) => {
                let fd = self.allocate_file_descriptor()?;

                let mut guard = self.file_descriptiors.write().or_else(|_| {
                    Err(MemFSErr::poisoned_lock())
                })?;
        
                guard.insert(fd, MemFSFileDescriptor::new(fd, flag & !(OpenFlag::O_CREAT), item_node.clone()));

                Ok(fd)
            }
        }
    }

    pub fn close(&self, fd: usize) -> Result<()> {
        let mut guard = self.file_descriptiors.write().or_else(|_| { Err(MemFSErr::poisoned_lock()) })?;

        guard.remove(&fd);

        Ok(())
    }

    fn create(&self, path: &str) -> Result<()> {
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

    pub fn read(&self, fd: usize, buffer: &mut Vec<u8>, size: usize) -> Result<usize> {
        let fd_map = self.file_descriptiors.read().or_else(|_| { Err(MemFSErr::poisoned_lock())})?;

        if let Some(v) = fd_map.get(&fd) {
            unsafe {v.read_file(buffer, size)}
        }
        else {
            Err(MemFSErr::bad_file_descriptor())
        }
    }

    pub fn write(&self, fd: usize, buffer: &Vec<u8>, size: usize) -> Result<usize> {
        let fd_map = self.file_descriptiors.read().or_else(|_| { Err(MemFSErr::poisoned_lock())})?;

        if let Some(v) = fd_map.get(&fd) {
            unsafe {v.write_file(buffer, size)}
        }
        else {
            Err(MemFSErr::bad_file_descriptor())
        }
    }

    pub fn lseek(&self, fd: usize, offset: usize, flag: SeekFlag) -> Result<usize> {
        let fd_map = self.file_descriptiors.read().or_else(|_| { Err(MemFSErr::poisoned_lock())})?;

        if let Some(v) = fd_map.get(&fd) {
            unsafe {v.seek_file(offset, flag)}
        }
        else {
            Err(MemFSErr::bad_file_descriptor())
        }
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
        let dir_path = Self::get_directory_names_excluding_last_one(path).peekable();
        let dir_node = self.root.search_entry_with_path(dir_path)?;
        let dir_guard = dir_node.write().or_else(|_| {
            Err(MemFSErr::poisoned_lock())
        })?;

        match &*dir_guard {
            MemFSEntry::Directory(dir) => dir.remove_directory(path.split("/").last().expect("Path is unspecified")),
            MemFSEntry::File(_) => Err(MemFSErr::no_such_file_or_directory())
        }
    }

    fn path_str_to_iter(path: &str) -> Peekable<impl Iterator<Item = &OsStr>> {
        Path::new(path).iter().peekable()
    }

    fn get_directory_names_excluding_last_one(path: &str) -> impl Iterator<Item = &OsStr> {
        let path_iter = Path::new(path).iter();
        let iter_count = path.split("/").count();
        
        path_iter.take(iter_count.saturating_sub(1))
    }

    fn allocate_file_descriptor(&self) -> Result<usize> {
        let mut guard = self.file_descriptor_count.lock().or_else(|_| {
            Err(MemFSErr::poisoned_lock())
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

    // Creating new file should not fail even if there is already a file with same name.
    pub fn create_new_file(&self, file_name: &str) -> Result<()> {
        let mut guard = self.children.write().or_else(  |_| { Err(MemFSErr::poisoned_lock())})?;

        if let Entry::Vacant(v) = guard.entry(file_name.to_string()) {
            v.insert(Arc::new(RwLock::new(MemFSEntry::File(MemFSFileNode::new()))));
        }

        Ok(())
    }

    pub fn create_new_directory(&self, dir_name: &str) -> Result<()> {
        let mut guard = self.children.write().or_else(  |_| {
            Err(MemFSErr::poisoned_lock())
        })?;

        match guard.entry(dir_name.to_string()) {
            Entry::Occupied(_) => Err(
                MemFSErr::already_exists()
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

        if guard.contains_key(file_name) {
            let entry = guard.get(file_name).unwrap();
            let entry_guard = entry.write().or_else(|_| {
                Err(MemFSErr::poisoned_lock())
            })?;

            if let MemFSEntry::Directory(_) = *entry_guard {
                return Err(MemFSErr::is_directory())
            }
        }
        else {
            return Err(MemFSErr::no_such_file_or_directory())
        }

        guard.remove_entry(file_name);

        Ok(())
    }

    pub fn remove_directory(&self, dir_name: &str) -> Result<()> {
        let mut guard = self.children.write().or_else(|_| {
            Err(MemFSErr::poisoned_lock())
        })?;

        if guard.contains_key(dir_name) {
            let entry = guard.get(dir_name).unwrap();
            let entry_guard = entry.write().or_else(|_| {
                Err(MemFSErr::poisoned_lock())
            })?;

            if let MemFSEntry::Directory(dir_node) = &*entry_guard {
                let children_guard = dir_node.children.read().or_else(|_| {
                    Err(MemFSErr::poisoned_lock())
                })?;

                if !children_guard.is_empty() {
                    return Err(MemFSErr::is_not_empty());
                }
            }
            else {
                return Err(MemFSErr::is_not_directory());
            }
        }
        else {
            return Err(MemFSErr::no_such_file_or_directory());
        }

        guard.remove_entry(dir_name);

        Ok(())
    }

    pub fn search_entry_with_path<'a>(&self, mut iter: Peekable<impl Iterator<Item = &'a OsStr>>) -> Result<Arc<RwLock<MemFSEntry>>> {
        let current_elem = iter.next();

        let current_path = current_elem.unwrap().to_str().unwrap();

        let next_elem = iter.peek();

        let guard = self.children.read().or_else(|_| {
            Err(MemFSErr::poisoned_lock())
        })?;

        match next_elem {
            Some(_) => {
                match guard.get(current_path) {
                    Some(v) => {
                        let inner_guard = v.read().or_else(|_| {
                            Err(MemFSErr::poisoned_lock())
                        })?;

                        match &*inner_guard {
                            MemFSEntry::Directory(dir) => dir.search_entry_with_path(iter),
                            MemFSEntry::File(_) => Err(MemFSErr::no_such_file_or_directory())
                        }
                    },
                    None => Err(MemFSErr::no_such_file_or_directory())
                }
            },
            None => {
                // Now at the end of path string. current_elem should be the one you looking for.
                match guard.get(current_path) {
                    Some(v) => Ok(v.clone()),
                    None => Err(MemFSErr::no_such_file_or_directory())
                }
            }
        }
    }

    pub fn add_child(&self, name: &str, entry: MemFSEntry) -> Result<()> {
        let mut guard = self.children.write().or_else(|_| {
            Err(MemFSErr::poisoned_lock())
        })?;

        match guard.entry(name.to_string()) {
            Entry::Occupied(_) => Err(MemFSErr::already_exists()),
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
    _number: usize,
    flag: OpenFlag,
    file_offset: Arc<Mutex<usize>>,
    entry: Arc<RwLock<MemFSEntry>>,
}

impl MemFSFileDescriptor {
    pub fn new(number: usize, flag: OpenFlag, entry: Arc<RwLock<MemFSEntry>>) -> Self {
        Self {
            _number: number,
            flag,
            file_offset: Arc::new(Mutex::new(0)),
            entry,
        }
    }

    pub unsafe fn read_file(&self, buffer: &mut Vec<u8>, size: usize) -> Result<usize> {
        if self.flag.contains(OpenFlag::O_WRONLY) {
            return Err(MemFSErr::bad_file_descriptor());
        }

        let guard = self.entry.read().or_else(|_| { Err(MemFSErr::poisoned_lock())})?;

        if let MemFSEntry::File(file) = &*guard {
            let file_guard = file.value.get();

            let mut offset_guard = self.file_offset.lock().or_else(|_| { Err(MemFSErr::poisoned_lock())})?;

            let content = unsafe { &*file_guard };
            let reading_length = (*offset_guard + size).min(content.len()).saturating_sub(*offset_guard);

            let slice_from_file = content[*offset_guard..(*offset_guard + reading_length)].to_vec();

            if buffer.len() < reading_length {
                return Err(MemFSErr::bad_memory_access());
            }

            buffer[0..reading_length].copy_from_slice(&slice_from_file);

            *offset_guard += slice_from_file.len();

            Ok(slice_from_file.len())
        }
        else {
            Err(MemFSErr::no_such_file_or_directory())
        }
    }
    
    pub unsafe fn write_file(&self, buffer: &Vec<u8>, size: usize) -> Result<usize> {
        if self.flag.contains(OpenFlag::O_RDONLY) {
            return Err(MemFSErr::bad_file_descriptor());
        }

        let guard = self.entry.read().or_else(|_| { Err(MemFSErr::poisoned_lock())})?;

        if let MemFSEntry::File(file) = &*guard {
            let file_guard = file.value.get();

            let mut offset_guard = self.file_offset.lock().or_else(|_| { Err(MemFSErr::poisoned_lock())} )?;

            let file_content = unsafe { &mut *file_guard};
            
            let writing_content_size = size.min(buffer.len());

            if *offset_guard + writing_content_size > file_content.len() {
                file_content.resize(*offset_guard + writing_content_size, 0);
            }

            file_content[*offset_guard..(*offset_guard + writing_content_size)].copy_from_slice(&buffer[0..writing_content_size]);

            *offset_guard += writing_content_size;

            Ok(writing_content_size)
        }
        else {
            Err(MemFSErr::no_such_file_or_directory())
        }
    }

    pub unsafe fn seek_file(&self, seek_position: usize, flag: SeekFlag) -> Result<usize> {
        let mut offset_guard = self.file_offset.lock().or_else(|_| { Err(MemFSErr::poisoned_lock())})?;

        let fg = self.entry.read().or_else(|_| { Err(MemFSErr::poisoned_lock())})?;

        let maximum_offset = if let MemFSEntry::File(file) = &*fg {
            let inner_guard = file.value.get();

            unsafe {&*inner_guard }.len()
        }
        else {
            return Err(MemFSErr::no_such_file_or_directory());
        };

        let additional_offset = match flag {
            SeekFlag::SEEK_CUR => *offset_guard,
            SeekFlag::SEEK_END => maximum_offset,
            SeekFlag::SEEK_SET => 0,
        };

        *offset_guard = maximum_offset.min(additional_offset + seek_position);

        Ok(*offset_guard)
    }
}