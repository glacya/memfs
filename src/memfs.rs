use crate::utils::{MemFSErr, OpenFlag, Result, SeekFlag};
use std::{
    cell::UnsafeCell,
    collections::{hash_map::Entry, HashMap},
    iter::Peekable,
    sync::{Arc, Mutex, RwLock, Weak}
};

/// Implementation of In-Memory file system that supports the following system calls:
/// [open], [close], [unlink], [read], [write], [lseek], [mkdir], [rmdir]
pub struct MemFS {
    root: Arc<RwLock<MemFSEntry>>,
    cwd_node: Arc<RwLock<MemFSEntry>>,
    // cwd_path: Arc<RwLock<String>>,
    file_descriptiors: Arc<RwLock<HashMap<usize, MemFSFileDescriptor>>>,
    file_descriptor_count: Arc<Mutex<usize>>,
}

unsafe impl Sync for MemFS {}

unsafe impl Send for MemFS {}

impl MemFS {
    pub fn new() -> Self {
        let root = Arc::new(RwLock::new(MemFSEntry::Directory(MemFSDirNode::root())));

        Self {
            root: root.clone(),
            cwd_node: root, 
            // cwd_path: Arc::new(RwLock::new("/".to_string())),
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
            self.create(path, OpenFlag::O_EXCL & (flag.clone()))?;
        }

        let item_node = self.get_node_of_given_path(path)?;

        let item_guard = item_node.read().map_err(|_| MemFSErr::poisoned_lock())?;

        match &*item_guard {
            MemFSEntry::Directory(_) => Err(MemFSErr::is_directory()),
            MemFSEntry::File(_) => {
                let fd = self.allocate_file_descriptor()?;

                let mut guard = self
                    .file_descriptiors
                    .write()
                    .map_err(|_| MemFSErr::poisoned_lock())?;

                guard.insert(
                    fd,
                    MemFSFileDescriptor::new(fd, flag & !(OpenFlag::O_CREAT), item_node.clone()),
                );

                Ok(fd)
            }
        }
    }

    pub fn close(&self, fd: usize) -> Result<()> {
        let mut guard = self
            .file_descriptiors
            .write()
            .map_err(|_| MemFSErr::poisoned_lock())?;

        if guard.contains_key(&fd) {
            guard.remove(&fd);
            Ok(())
        } else {
            Err(MemFSErr::bad_file_descriptor())
        }
    }

    pub fn unlink(&self, path: &str) -> Result<()> {
        // let (dir_path, last_elem) = self.path_str_to_iter_and_prepare_last_component(path)?;
        // let dir_node = self.root.search_entry_with_path(dir_path)?;
        let dir_node = self.get_parent_directory_node_of_given_path(path)?;
        let last_elem = Self::get_last_component_of_path(path)?;
        let dir_guard = dir_node.write().map_err(|_| MemFSErr::poisoned_lock())?;

        match &*dir_guard {
            MemFSEntry::Directory(dir) => {
                dir.remove_file(last_elem)
            }
            MemFSEntry::File(_) => Err(MemFSErr::no_such_file_or_directory()),
        }
    }

    pub fn read(&self, fd: usize, buffer: &mut Vec<u8>, size: usize) -> Result<usize> {
        let fd_map = self
            .file_descriptiors
            .read()
            .map_err(|_| MemFSErr::poisoned_lock())?;

        if let Some(v) = fd_map.get(&fd) {
            unsafe { v.read_file(buffer, size) }
        } else {
            Err(MemFSErr::bad_file_descriptor())
        }
    }

    pub fn write(&self, fd: usize, buffer: &Vec<u8>, size: usize) -> Result<usize> {
        let fd_map = self
            .file_descriptiors
            .read()
            .map_err(|_| MemFSErr::poisoned_lock())?;

        if let Some(v) = fd_map.get(&fd) {
            unsafe { v.write_file(buffer, size) }
        } else {
            Err(MemFSErr::bad_file_descriptor())
        }
    }

    pub fn lseek(&self, fd: usize, offset: usize, flag: SeekFlag) -> Result<usize> {
        let fd_map = self
            .file_descriptiors
            .read()
            .map_err(|_| MemFSErr::poisoned_lock())?;

        if let Some(v) = fd_map.get(&fd) {
            unsafe { v.seek_file(offset, flag) }
        } else {
            Err(MemFSErr::bad_file_descriptor())
        }
    }

    pub fn mkdir(&self, path: &str) -> Result<()> {
        // let (dir_path, last_elem) = self.path_str_to_iter_and_prepare_last_component(path)?;
        // let dir_node = self.root.search_entry_with_path(dir_path)?;
        let dir_node = self.get_parent_directory_node_of_given_path(path)?;
        let last_elem = Self::get_last_component_of_path(path)?;
        let dir_guard = dir_node.write().map_err(|_| MemFSErr::poisoned_lock())?;

        match &*dir_guard {
            MemFSEntry::Directory(dir) => {
                dir.create_new_directory(last_elem)
            }
            MemFSEntry::File(_) => Err(MemFSErr::no_such_file_or_directory()),
        }
    }

    pub fn rmdir(&self, path: &str) -> Result<()> {
        // let (dir_path, last_elem) = self.path_str_to_iter_and_prepare_last_component(path)?;
        // let root = *self.root.read().unwrap();
        // let dir_node = root.search_entry_with_path(dir_path)?;
        let dir_node = self.get_parent_directory_node_of_given_path(path)?;
        let last_elem = Self::get_last_component_of_path(path)?;
        let dir_guard = dir_node.write().map_err(|_| MemFSErr::poisoned_lock())?;

        match &*dir_guard {
            MemFSEntry::Directory(dir) => {
                dir.remove_directory(last_elem)
            }
            MemFSEntry::File(_) => Err(MemFSErr::no_such_file_or_directory()),
        }
    }

    pub fn chdir(&mut self, path: &str) -> Result<()> {
        println!("CHDIR({})", path);

        if path.is_empty() {
            return Err(MemFSErr::no_such_file_or_directory());
        }

        let dir_node = self.get_node_of_given_path(path)?;
        let dir_guard = dir_node.read().map_err(|_| MemFSErr::poisoned_lock())?;

        match &*dir_guard {
            MemFSEntry::Directory(_) => {
                self.cwd_node = dir_node.clone();
                Ok(())
            }
            _ => Err(MemFSErr::is_not_directory())
        }
    }

    fn create(&self, path: &str, flag: OpenFlag) -> Result<()> {
        let dir_node = self.get_parent_directory_node_of_given_path(path)?;
        let last_elem = Self::get_last_component_of_path(path)?;
        let dir_guard = dir_node.write().map_err(|_| MemFSErr::poisoned_lock())?;

        match &*dir_guard {
            MemFSEntry::Directory(dir) => {
                dir.create_new_file(last_elem, flag)
            }
            MemFSEntry::File(_) => Err(MemFSErr::no_such_file_or_directory()),
        }
    }

    fn path_str_to_iter(&self, path: &str) -> Result<Peekable<impl Iterator<Item = String>>> {
        // let resolved_path = self.resolve_path_to_full_absolute_path(path)?;
        // let vec: Vec<String> = resolved_path.split("/").map(|x| x.to_string()).collect();

        if path.is_empty() {
            return Err(MemFSErr::no_such_file_or_directory());
        }

        let add_front = path.chars().nth(0).unwrap() == '/';

        let mut vec: Vec<String> = path.split("/").filter(|x| *x != "").map(|x| x.to_string()).collect();
        
        if add_front {
            let mut front_vec = vec!["".to_string()];

            front_vec.extend(vec);

            vec = front_vec;
        }

        Ok(vec.into_iter().peekable())
    }

    fn path_str_to_iter_and_without_last_component(&self, path: &str) -> Result<Peekable<impl Iterator<Item = String>>> {
        // let resolved_path = self.resolve_path_to_full_absolute_path(path)?;
        // let vec: Vec<String> = resolved_path.split("/").map(|x| x.to_string()).collect();
        if path.is_empty() {
            return Err(MemFSErr::no_such_file_or_directory());
        }

        let add_front = path.chars().nth(0).unwrap() == '/';

        let mut vec: Vec<String> = path.split("/").filter(|x| *x != "").map(|x| x.to_string()).collect();

        if add_front {
            let mut front_vec = vec!["".to_string()];

            front_vec.extend(vec);

            vec = front_vec;
        }

        let iter_count = vec.len();
        let path_iter = vec.into_iter();

        Ok(path_iter.take(iter_count.saturating_sub(1)).peekable())
    }

    fn get_last_component_of_path(path: &str) -> Result<&str> {
        path.split("/").last().ok_or(MemFSErr::no_such_file_or_directory())
    }

    fn get_node_of_given_path(&self, path: &str) -> Result<Arc<RwLock<MemFSEntry>>> {
        if path.is_empty() {
            return Err(MemFSErr::no_such_file_or_directory());
        }
        
        let iter = self.path_str_to_iter(path)?;

        let guard = if path.chars().nth(0).unwrap() == '/' {
            // Absolute path
            self.root.read().map_err(|_| MemFSErr::poisoned_lock())
        }
        else {
            // Relative path
            self.cwd_node.read().map_err(|_| MemFSErr::poisoned_lock())
        }?;

        match &*guard {
            MemFSEntry::Directory(dir) => dir.search_entry_with_path(iter),
            MemFSEntry::File(_) => Err(MemFSErr::no_such_file_or_directory())
        }
    }

    fn get_parent_directory_node_of_given_path(&self, path: &str) -> Result<Arc<RwLock<MemFSEntry>>> {
        if path.is_empty() {
            return Err(MemFSErr::no_such_file_or_directory());
        }

        let mut iter = self.path_str_to_iter_and_without_last_component(path)?;

        if iter.peek().is_none() {
            return Ok(self.root.clone());
        }

        let guard = if path.chars().nth(0).unwrap() == '/' {
            // Absolute path
            self.root.read().map_err(|_| MemFSErr::poisoned_lock())
        }
        else {
            // Relative path
            self.cwd_node.read().map_err(|_| MemFSErr::poisoned_lock())
        }?;

        match &*guard {
            MemFSEntry::Directory(dir) => dir.search_entry_with_path(iter),
            MemFSEntry::File(_) => Err(MemFSErr::no_such_file_or_directory())
        }
    }

    fn allocate_file_descriptor(&self) -> Result<usize> {
        let mut guard = self
            .file_descriptor_count
            .lock()
            .map_err(|_| MemFSErr::poisoned_lock())?;

        let fd = *guard;
        *guard += 1;

        Ok(fd)
    }
}

#[derive(Clone)]
pub struct MemFSDirNode {
    parent: Option<Weak<RwLock<MemFSEntry>>>,
    children: Arc<RwLock<HashMap<String, Arc<RwLock<MemFSEntry>>>>>,
}

impl MemFSDirNode {
    pub fn new() -> Self {
        Self {
            parent: None,
            children: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn root() -> Self {
        let inner = MemFSEntry::Directory(Self::new());
        let mut map = HashMap::new();

        map.insert("".to_string(), Arc::new(RwLock::new(inner)));

        Self {
            parent: None,
            children: Arc::new(RwLock::new(map)),
        }
    }

    pub fn with_parent(parent: Weak<RwLock<MemFSEntry>>) -> Self {
        Self {
            parent: Some(parent),
            children: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Creating new file with O_CREAT flag should not fail even if there is already a file with same name.
    /// However, if O_EXCL flag is given along with O_CREAT, creating new file with existing file name
    /// must fail.
    fn create_new_file(&self, file_name: &str, flag: OpenFlag) -> Result<()> {
        let mut guard = self
            .children
            .write()
            .map_err(|_| MemFSErr::poisoned_lock())?;

        match guard.entry(file_name.to_string()) {
            Entry::Vacant(v) => {
                v.insert(Arc::new(RwLock::new(
                    MemFSEntry::File(MemFSFileNode::new()),
                )));
            }
            Entry::Occupied(_) => {
                if flag.contains(OpenFlag::O_EXCL) {
                    return Err(MemFSErr::already_exists());
                }
            }
        }

        Ok(())
    }

    fn create_new_directory(&self, dir_name: &str) -> Result<()> {
        let mut guard = self
            .children
            .write()
            .map_err(|_| MemFSErr::poisoned_lock())?;

        match guard.entry(dir_name.to_string()) {
            Entry::Occupied(_) => Err(MemFSErr::already_exists()),
            Entry::Vacant(v) => {
                v.insert(Arc::new(RwLock::new(MemFSEntry::Directory(
                    MemFSDirNode::new(),
                ))));
                Ok(())
            }
        }
    }

    fn remove_file(&self, file_name: &str) -> Result<()> {
        let mut guard = self
            .children
            .write()
            .map_err(|_| MemFSErr::poisoned_lock())?;

        if guard.contains_key(file_name) {
            let entry = guard.get(file_name).unwrap();
            let entry_guard = entry.write().map_err(|_| MemFSErr::poisoned_lock())?;

            if let MemFSEntry::Directory(_) = *entry_guard {
                return Err(MemFSErr::is_directory());
            }
        } else {
            return Err(MemFSErr::no_such_file_or_directory());
        }

        guard.remove_entry(file_name);

        Ok(())
    }

    fn remove_directory(&self, dir_name: &str) -> Result<()> {
        let mut guard = self
            .children
            .write()
            .map_err(|_| MemFSErr::poisoned_lock())?;

        if guard.contains_key(dir_name) {
            let entry = guard.get(dir_name).unwrap();
            let entry_guard = entry.write().map_err(|_| MemFSErr::poisoned_lock())?;

            if let MemFSEntry::Directory(dir_node) = &*entry_guard {
                let children_guard = dir_node
                    .children
                    .read()
                    .map_err(|_| MemFSErr::poisoned_lock())?;

                if !children_guard.is_empty() {
                    return Err(MemFSErr::is_not_empty());
                }
            } else {
                return Err(MemFSErr::is_not_directory());
            }
        } else {
            return Err(MemFSErr::no_such_file_or_directory());
        }

        guard.remove_entry(dir_name);

        Ok(())
    }

    fn search_entry_with_path(
        &self,
        mut iter: Peekable<impl Iterator<Item = String>>,
    ) -> Result<Arc<RwLock<MemFSEntry>>> {
        let current_elem = iter.next();

        let cv = current_elem.unwrap();
        let current_path = cv.as_str();

        let next_elem = iter.peek();

        println!("{}, {:?}", current_path, next_elem);

        let guard = self
            .children
            .read()
            .map_err(|_| MemFSErr::poisoned_lock())?;

        print!("Entry names: ");
        let hmap = &*guard;
        for key in hmap.keys() {
            print!("{}, ", key);
        }
        println!("");

        match next_elem {
            Some(_) => match guard.get(current_path) {
                Some(v) => {
                    let inner_guard = v.read().map_err(|_| MemFSErr::poisoned_lock())?;

                    match &*inner_guard {
                        MemFSEntry::Directory(dir) => dir.search_entry_with_path(iter),
                        MemFSEntry::File(_) => Err(MemFSErr::is_not_directory()),
                    }
                }
                None => Err(MemFSErr::no_such_file_or_directory()),
            },
            None => {
                // Now at the end of path string. current_elem should be the one you looking for.
                match guard.get(current_path) {
                    Some(v) => Ok(v.clone()),
                    None => Err(MemFSErr::no_such_file_or_directory()),
                }
            }
        }
    }
}

pub struct MemFSFileNode {
    value: UnsafeCell<Vec<u8>>,
}

impl MemFSFileNode {
    pub fn new() -> Self {
        Self {
            value: UnsafeCell::new(vec![]),
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
    file_offset: Arc<RwLock<usize>>,
    entry: Arc<RwLock<MemFSEntry>>,
    append_mutex: Arc<Mutex<()>>,
}

impl MemFSFileDescriptor {
    pub fn new(number: usize, flag: OpenFlag, entry: Arc<RwLock<MemFSEntry>>) -> Self {
        Self {
                _number: number,
                flag,
                file_offset: Arc::new(RwLock::new(0)),
                entry,
                append_mutex: Arc::new(Mutex::new(())),
        }
    }

    unsafe fn read_file(&self, buffer: &mut Vec<u8>, size: usize) -> Result<usize> {
        if self.flag.contains(OpenFlag::O_WRONLY) {
            return Err(MemFSErr::bad_file_descriptor());
        }

        let guard = self.entry.read().map_err(|_| MemFSErr::poisoned_lock())?;

        if let MemFSEntry::File(file) = &*guard {
            let file_guard = file.value.get();

            let offset_read_guard = self
                .file_offset
                .read()
                .map_err(|_| MemFSErr::poisoned_lock())?;

            let current_offset = *offset_read_guard;
            drop(offset_read_guard);

            let content = unsafe { &*file_guard };
            let reading_length = ((current_offset).saturating_add(size))
                .min(content.len())
                .saturating_sub(current_offset);

            let slice_from_file =
                content[current_offset..(current_offset).saturating_add(reading_length)].to_vec();

            if buffer.len() < reading_length {
                return Err(MemFSErr::bad_memory_access());
            }

            buffer[0..reading_length].copy_from_slice(&slice_from_file);

            let mut offset_write_guard = self.file_offset.write().map_err(|_| MemFSErr::poisoned_lock())?;

            *offset_write_guard = (*offset_write_guard).saturating_add(reading_length);

            Ok(slice_from_file.len())
        } else {
            Err(MemFSErr::no_such_file_or_directory())
        }
    }

    unsafe fn write_file(&self, buffer: &Vec<u8>, size: usize) -> Result<usize> {
        if self.flag.contains(OpenFlag::O_RDONLY) {
            return Err(MemFSErr::bad_file_descriptor());
        }

        let guard = self.entry.read().map_err(|_| MemFSErr::poisoned_lock())?;

        if let MemFSEntry::File(file) = &*guard {
            let file_guard = file.value.get();
            let file_content = unsafe { &mut *file_guard };

            let lock =  self.append_mutex.lock().map_err(|_| MemFSErr::poisoned_lock());

            let current_offset = if self.flag.contains(OpenFlag::O_APPEND) {
                let mut offset_write_guard = self
                    .file_offset
                    .write()
                    .map_err(|_| MemFSErr::poisoned_lock())?;

                let current_file_size = file_content.len();

                *offset_write_guard = current_file_size;
                drop(offset_write_guard);

                current_file_size
            }
            else {
                drop(lock);
                
                let offset_read_guard = self
                    .file_offset
                    .read()
                    .map_err(|_| MemFSErr::poisoned_lock())?;

                let value = *offset_read_guard;
                drop(offset_read_guard);

                value
            };

            let writing_content_size = size.min(buffer.len());
            let expected_offset = current_offset.saturating_add(writing_content_size);

            if expected_offset > file_content.len() {
                file_content.resize(expected_offset, 0);
            }

            file_content[current_offset..expected_offset]
                .copy_from_slice(&buffer[0..writing_content_size]);

            let mut offset_write_guard = self
                .file_offset
                .write()
                .map_err(|_| MemFSErr::poisoned_lock())?;

            *offset_write_guard = expected_offset;

            Ok(writing_content_size)
        } else {
            Err(MemFSErr::no_such_file_or_directory())
        }
    }

    unsafe fn seek_file(&self, seek_position: usize, flag: SeekFlag) -> Result<usize> {
        let offset_guard = self
            .file_offset
            .read()
            .map_err(|_| MemFSErr::poisoned_lock())?;

        let fg = self.entry.read().map_err(|_| MemFSErr::poisoned_lock())?;
        let current_offset = *offset_guard;

        drop(offset_guard);

        let maximum_offset = if let MemFSEntry::File(file) = &*fg {
            let inner_guard = file.value.get();

            unsafe { &*inner_guard }.len()
        } else {
            return Err(MemFSErr::no_such_file_or_directory());
        };

        let additional_offset = match flag {
            SeekFlag::SEEK_CUR => current_offset,
            SeekFlag::SEEK_END => maximum_offset,
            SeekFlag::SEEK_SET => 0,
        };

        let mut write_guard = self.file_offset.write().map_err(|_| MemFSErr::poisoned_lock())?;

        *write_guard = maximum_offset.min(additional_offset.saturating_add(seek_position));

        Ok(*write_guard)
    }
}
