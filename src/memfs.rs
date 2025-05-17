use crossbeam::queue::ArrayQueue;
use dashmap::{DashMap, Entry};

use crate::utils::{MemFSErr, OpenFlag, Result, SeekFlag, FILE_MAX_SIZE, NUMBER_OF_MAXIMUM_FILES};
use std::{
    cell::UnsafeCell, iter::Peekable, sync::{atomic::{AtomicUsize, Ordering}, Arc, Mutex, Weak}
};

// static THREAD_ASSIGN: AtomicUsize = AtomicUsize::new(0);

// thread_local! {
//     static THREAD_ID: usize = THREAD_ASSIGN.fetch_add(1, Ordering::AcqRel);
// }

/// Implementation of In-Memory file system that supports the following system calls:
/// [open], [close], [unlink], [read], [write], [lseek], [mkdir], [rmdir]
pub struct MemFS {
    root: Arc<MemFSEntry>,
    cwd_node: Arc<MemFSEntry>,
    file_descriptors: Arc<DashMap<usize, MemFSFileDescriptor>>,
    file_descriptor_count: AtomicUsize,
    file_memory: Arc<ArrayQueue<Vec<u8>>>,
}

unsafe impl Sync for MemFS {}
unsafe impl Send for MemFS {}

impl MemFS {
    pub fn new() -> Self {
        let root = Arc::new(MemFSEntry::Directory(MemFSDirNode::new()));
        let seg_queue = ArrayQueue::new(NUMBER_OF_MAXIMUM_FILES);

        for _ in 0..NUMBER_OF_MAXIMUM_FILES {
            seg_queue.push(vec![0; FILE_MAX_SIZE]).unwrap();
        }

        Self {
            root: root.clone(),
            cwd_node: root,
            file_descriptors: Arc::new(DashMap::new()),
            file_descriptor_count: AtomicUsize::new(0),
            file_memory: Arc::new(seg_queue),
        }   
    }

    pub fn open(&self, path: &str, flag: OpenFlag) -> Result<usize> {
        // Check flag. O_RDONLY, O_WRONLY, O_RDWR are the mutually exclusive ones.
        if !flag.check_mode_exclusiveness() {
            return Err(MemFSErr::invalid_value());
        }

        let parent_node = self.get_parent_directory_node_of_given_path(path)?;
        let last_elem = Self::get_last_component_of_path(path)?;

        match self.resolve_dir_and_entry(last_elem, &*parent_node)? {
            Entry::Vacant(v) => {
                if flag.contains(OpenFlag::O_CREAT) {
                    
                    // If the entry is empty and O_CREAT is specified, add the file entry.
                    let memory_block = self.allocate_file_memory()?;
                    let file_node = Arc::new(MemFSEntry::File(MemFSFileNode::new(memory_block)));

                    let fd = self.allocate_file_descriptor()?;
                    
                    v.insert(file_node.clone());

                    self.file_descriptors.insert(fd, MemFSFileDescriptor::new(fd, flag & !(OpenFlag::O_CREAT), file_node));

                    Ok(fd)
                }
                else {
                    Err(MemFSErr::no_such_file_or_directory())
                }
            },
            Entry::Occupied(v) => {
                if flag.contains(OpenFlag::O_CREAT | OpenFlag::O_EXCL) {
                    Err(MemFSErr::already_exists())
                }
                else {
                    let file_node = v.get();

                    match &**file_node {
                        MemFSEntry::File(_) => {
                            let fd = self.allocate_file_descriptor()?;

                            self.file_descriptors.insert(fd, MemFSFileDescriptor::new(fd, flag & !(OpenFlag::O_CREAT), file_node.clone()));

                            Ok(fd)
                        }
                        _ => Err(MemFSErr::is_directory())
                    }
                }
            },
        }
    }

    pub fn close(&self, fd: usize) -> Result<()> {
        let entry = self.file_descriptors.entry(fd);

        match entry {
            Entry::Occupied(e) => {
                e.remove();
                Ok(())
            },
            Entry::Vacant(_) => Err(MemFSErr::bad_file_descriptor())
        }
    }

    pub fn unlink(&self, path: &str) -> Result<()> {
        let dir_node = self.get_parent_directory_node_of_given_path(path)?;
        let last_elem = Self::get_last_component_of_path(path)?;

        match &*dir_node {
            MemFSEntry::Directory(dir) => dir.remove_file(last_elem),
            MemFSEntry::File(_) => Err(MemFSErr::no_such_file_or_directory()),
            MemFSEntry::ResolvedAsRoot => {
                if let MemFSEntry::Directory(dir) = &*self.root {
                    dir.remove_file(last_elem)
                } else {
                    Err(MemFSErr::no_such_file_or_directory())
                }
            }
        }
    }

    pub fn read(&self, fd: usize, buffer: &mut Vec<u8>, size: usize) -> Result<usize> {
        if let Some(v) = self.file_descriptors.get(&fd) {
            unsafe { v.read_file(buffer, size) }
        } else {
            Err(MemFSErr::bad_file_descriptor())
        }
    }

    pub fn write(&self, fd: usize, buffer: &Vec<u8>, size: usize) -> Result<usize> {
        if let Some(v) = self.file_descriptors.get(&fd) {
            unsafe { v.write_file(buffer, size) }
        } else {
            Err(MemFSErr::bad_file_descriptor())
        }
    }

    pub fn lseek(&self, fd: usize, offset: usize, flag: SeekFlag) -> Result<usize> {
        if let Some(v) = self.file_descriptors.get(&fd) {
            unsafe { v.seek_file(offset, flag) }
        } else {
            Err(MemFSErr::bad_file_descriptor())
        }
    }

    pub fn mkdir(&self, path: &str) -> Result<()> {
        // println!("mkdir({})", path);

        if path == "/" {
            return Err(MemFSErr::already_exists());
        }

        let dir_node = self.get_parent_directory_node_of_given_path(path)?;
        let last_elem = Self::get_last_component_of_path(path)?;

        if last_elem == "." || last_elem == ".." {
            return Err(MemFSErr::already_exists());
        }

        match &*dir_node {
            MemFSEntry::Directory(dir) => dir.create_new_directory(last_elem, dir_node.clone()),
            MemFSEntry::File(_) => Err(MemFSErr::no_such_file_or_directory()),
            MemFSEntry::ResolvedAsRoot => Err(MemFSErr::already_exists()),
        }
    }

    pub fn rmdir(&self, path: &str) -> Result<()> {
        if path == "/" {
            return Err(MemFSErr::busy());
        }

        let dir_node = self.get_parent_directory_node_of_given_path(path)?;
        let last_elem = Self::get_last_component_of_path(path)?;

        if last_elem == "." {
            return Err(MemFSErr::invalid_value());
        } else if last_elem == ".." {
            return Err(MemFSErr::is_not_empty());
        }

        match &*dir_node {
            MemFSEntry::Directory(dir) => dir.remove_directory(last_elem),
            MemFSEntry::File(_) => Err(MemFSErr::no_such_file_or_directory()),
            MemFSEntry::ResolvedAsRoot => Err(MemFSErr::busy()),
        }
    }

    pub fn chdir(&mut self, path: &str) -> Result<()> {
        if path.is_empty() {
            return Err(MemFSErr::no_such_file_or_directory());
        } else if path == "/" {
            self.cwd_node = self.root.clone();
            return Ok(());
        }

        let dir_node = self.get_node_of_given_path(path)?;

        match &*dir_node {
            MemFSEntry::Directory(_) => {
                self.cwd_node = dir_node.clone();

                Ok(())
            }
            MemFSEntry::ResolvedAsRoot => {
                self.cwd_node = self.root.clone();

                Ok(())
            }
            _ => Err(MemFSErr::is_not_directory()),
        }
    }

    fn path_str_to_iter(&self, path: &str) -> Result<Peekable<impl Iterator<Item = String>>> {
        if path.is_empty() {
            return Err(MemFSErr::no_such_file_or_directory());
        }

        let vec: Vec<String> = path
            .split("/")
            .filter(|x| *x != "" && *x != ".")
            .map(|x| x.to_string())
            .collect();

        Ok(vec.into_iter().peekable())
    }

    fn path_str_to_iter_and_without_last_component(
        &self,
        path: &str,
    ) -> Result<Peekable<impl Iterator<Item = String>>> {
        if path.is_empty() {
            return Err(MemFSErr::no_such_file_or_directory());
        }

        let mut vec: Vec<String> = path
            .split("/")
            .filter(|x| *x != "" && *x != ".")
            .map(|x| x.to_string())
            .collect();
        
        vec.pop();

        Ok(vec.into_iter().peekable())
    }

    fn is_absolute_path(path: &str) -> bool {
        path.chars().nth(0).unwrap() == '/'
    }

    fn get_last_component_of_path(path: &str) -> Result<&str> {
        path.trim_end_matches('/')
            .split("/")
            .last()
            .ok_or(MemFSErr::no_such_file_or_directory())
    }

    fn get_node_of_given_path(&self, path: &str) -> Result<Arc<MemFSEntry>> {
        if path.is_empty() {
            return Err(MemFSErr::no_such_file_or_directory());
        }

        let mut iter = self.path_str_to_iter(path)?;

        if iter.peek().is_none() {
            return if Self::is_absolute_path(path) {
                Ok(self.root.clone())
            } else {
                Ok(self.cwd_node.clone())
            };
        }

        let starting_node = if Self::is_absolute_path(path) {
            // Absolute path
            self.root.clone()
        } else {
            // Relative path
            self.cwd_node.clone()
        };

        match &*starting_node {
            MemFSEntry::Directory(dir) => dir.search_entry_with_path(iter),
            MemFSEntry::File(_) => Err(MemFSErr::no_such_file_or_directory()),
            MemFSEntry::ResolvedAsRoot => Ok(self.root.clone()),
        }
    }

    fn get_parent_directory_node_of_given_path(
        &self,
        path: &str,
    ) -> Result<Arc<MemFSEntry>> {
        if path.is_empty() {
            return Err(MemFSErr::no_such_file_or_directory());
        }

        let mut iter = self.path_str_to_iter_and_without_last_component(path)?;

        if iter.peek().is_none() {
            return if Self::is_absolute_path(path) {
                Ok(self.root.clone())
            } else {
                Ok(self.cwd_node.clone())
            };
        }

        let starting_node = if Self::is_absolute_path(path) {
            // Absolute path
            self.root.clone()
        } else {
            // Relative path
            self.cwd_node.clone()
        };

        match &*starting_node {
            MemFSEntry::Directory(dir) => dir.search_entry_with_path(iter),
            MemFSEntry::File(_) => Err(MemFSErr::no_such_file_or_directory()),
            MemFSEntry::ResolvedAsRoot => Ok(self.root.clone()),
        }
    }

    fn allocate_file_descriptor(&self) -> Result<usize> {
        let fd = self.file_descriptor_count.fetch_add(1, Ordering::AcqRel);
        Ok(fd)
    }

    fn resolve_dir_and_entry<'a>(&'a self, last_elem: &str, parent_node: &'a MemFSEntry) -> Result<Entry<'a, String, Arc<MemFSEntry>>> {
        match parent_node {
            MemFSEntry::Directory(dir) => {
                Ok(dir.children.entry(last_elem.to_string()))
            },
            MemFSEntry::ResolvedAsRoot => {
                match &*self.root {
                    MemFSEntry::Directory(rootdir) => Ok(rootdir.children.entry(last_elem.to_string())),
                    _ => return Err(MemFSErr::no_such_file_or_directory()),
                }
            },
            MemFSEntry::File(_) => Err(MemFSErr::is_not_directory()),
        }
    }

    /// Allocates file memory.
    /// The implementation is very bad, but it can handle tests.
    fn allocate_file_memory(&self) -> Result<Vec<u8>> {
        if let Some(block) = self.file_memory.pop() {
            Ok(block)
        }
        else {
            Err(MemFSErr::out_of_memory())
        }
    }
}

unsafe impl Sync for MemFSDirNode {}
unsafe impl Send for MemFSDirNode {}

#[derive(Clone)]
pub struct MemFSDirNode {
    parent: Option<Weak<MemFSEntry>>,
    children: Arc<DashMap<String, Arc<MemFSEntry>>>,
}

impl MemFSDirNode {
    pub fn new() -> Self {
        Self {
            parent: None,
            children: Arc::new(DashMap::new()),
        }
    }

    pub fn with_parent(parent: Weak<MemFSEntry>) -> Self {
        Self {
            parent: Some(parent),
            children: Arc::new(DashMap::new()),
        }
    }

    /// Creating new file with O_CREAT flag should not fail even if there is already a file with same name.
    /// However, if O_EXCL flag is given along with O_CREAT, creating new file with existing file name
    /// must fail.
    // fn create_new_file(&self, file_name: &str, flag: OpenFlag) -> Result<()> {
    //     match self.children.entry(file_name.to_string()) {
    //         Entry::Vacant(v) => {
    //             v.insert(Arc::new(
    //                 MemFSEntry::File(MemFSFileNode::new()),
    //             ));
    //         }
    //         Entry::Occupied(_) => {
    //             if flag.contains(OpenFlag::O_EXCL) {
    //                 return Err(MemFSErr::already_exists());
    //             }
    //         }
    //     }

    //     Ok(())
    // }

    fn create_new_directory(
        &self,
        dir_name: &str,
        parent_ptr: Arc<MemFSEntry>,
    ) -> Result<()> {
        match self.children.entry(dir_name.to_string()) {
            Entry::Occupied(_) => Err(MemFSErr::already_exists()),
            Entry::Vacant(v) => {
                v.insert(Arc::new(MemFSEntry::Directory(
                    MemFSDirNode::with_parent(Arc::downgrade(&parent_ptr)),
                )));
                Ok(())
            }
        }
    }

    fn remove_file(&self, file_name: &str) -> Result<()> {
        match self.children.entry(file_name.to_string()) {
            Entry::Occupied(v) => {
                let inner = v.get();

                if let MemFSEntry::File(_) = &**inner {
                    v.remove();
                    Ok(())
                }
                else {
                    Err(MemFSErr::is_directory())
                }
            },
            Entry::Vacant(_) => Err(MemFSErr::no_such_file_or_directory())
        }
    }

    fn remove_directory(&self, dir_name: &str) -> Result<()> {
        match self.children.entry(dir_name.to_string()) {
            Entry::Occupied(v) => {
                let inner = v.get();

                if let MemFSEntry::Directory(dir_node) = &**inner {
                    if dir_node.children.is_empty() {
                        v.remove();
                        Ok(())
                    }
                    else {
                        Err(MemFSErr::is_not_empty())
                    }
                }
                else {
                    Err(MemFSErr::is_not_directory())
                }
            },
            Entry::Vacant(_) => Err(MemFSErr::no_such_file_or_directory())
        }
    }

    fn search_entry_with_path(
        &self,
        mut iter: Peekable<impl Iterator<Item = String>>,
    ) -> Result<Arc<MemFSEntry>> {
        let current_elem = iter.next();

        let cv = current_elem.unwrap();
        let current_path = cv.as_str();

        let next_elem = iter.peek();

        // println!("{}, {:?}", current_path, next_elem);
        // println!("Entries:");
        
        // for (key, entry) in self.children.iter() {
        //     println!("{} -> {:?}", key, Arc::as_ptr(&entry));
        // }
        // println!("{} entries.", hmap.len());

        match next_elem {
            Some(_) => match self.children.get(current_path) {
                Some(v) => {
                    match &**v {
                        MemFSEntry::Directory(dir) => dir.search_entry_with_path(iter),
                        MemFSEntry::File(_) => Err(MemFSErr::is_not_directory()),
                        _ => unreachable!(),
                    }
                }
                None => {
                    match current_path {
                        // "." => self.search_entry_with_path(iter),
                        ".." => match &self.parent {
                            Some(parent) => {
                                if let Some(inner) = parent.upgrade() {
                                    if let MemFSEntry::Directory(dir) = &*inner {
                                        dir.search_entry_with_path(iter)
                                    } else {
                                        Err(MemFSErr::is_not_directory())
                                    }
                                } else {
                                    Err(MemFSErr::no_such_file_or_directory())
                                }
                            }
                            None => self.search_entry_with_path(iter),
                        },
                        _ => Err(MemFSErr::no_such_file_or_directory()),
                    }
                }
            },
            None => {
                // Now at the end of path string. current_elem should be the one you looking for.
                match self.children.get(current_path) {
                    Some(v) => Ok(v.clone()),
                    None => match current_path {
                        ".." => match &self.parent {
                            Some(parent) => {
                                if let Some(inner) = parent.upgrade() {
                                    if let MemFSEntry::Directory(_) = &*inner {
                                        Ok(inner.clone())
                                    } else {
                                        Err(MemFSErr::is_not_directory())
                                    }
                                } else {
                                    Err(MemFSErr::no_such_file_or_directory())
                                }
                            }
                            None => Ok(Arc::new(MemFSEntry::ResolvedAsRoot)),
                        },
                        _ => Err(MemFSErr::no_such_file_or_directory()),
                    },
                }
            }
        }
    }
}

unsafe impl Sync for MemFSFileNode {}
unsafe impl Send for MemFSFileNode {}

pub struct MemFSFileNode {
    size: AtomicUsize,
    data: UnsafeCell<Vec<u8>>,
    // page_cache: Arc<HashMap<usize, Vec<u8>>>,
}

impl MemFSFileNode {
    pub fn new(space: Vec<u8>) -> Self {
        // let iter = (0..THREAD_MAX_ID).map(|x| (x, ));
        // let mut prepared_map = HashMap::from_iter(iter);

        Self {
            size: AtomicUsize::new(0),
            data: UnsafeCell::new(space),
            // page_cache: Arc::new(prepared_map)
        }
    }
}

unsafe impl Sync for MemFSEntry {}
unsafe impl Send for MemFSEntry {}

pub enum MemFSEntry {
    Directory(MemFSDirNode),
    File(MemFSFileNode),
    ResolvedAsRoot,
}

struct MemFSFileDescriptor {
    _number: usize,
    flag: OpenFlag,
    file_offset: AtomicUsize,
    entry: Arc<MemFSEntry>,
    append_mutex: Arc<Mutex<()>>,
}

impl MemFSFileDescriptor {
    pub fn new(number: usize, flag: OpenFlag, entry: Arc<MemFSEntry>) -> Self {
        Self {
            _number: number,
            flag,
            file_offset: AtomicUsize::new(0),
            entry,
            append_mutex: Arc::new(Mutex::new(())),
        }
    }

    unsafe fn read_file(&self, buffer: &mut Vec<u8>, size: usize) -> Result<usize> {
        if self.flag.contains(OpenFlag::O_WRONLY) {
            return Err(MemFSErr::bad_file_descriptor());
        }

        if let MemFSEntry::File(file) = &*self.entry {
            let file_guard = file.data.get();
            let current_offset = self.file_offset.load(Ordering::Acquire);
            let file_size = file.size.load(Ordering::Relaxed);

            let content = unsafe { &*file_guard };
            let reading_length = ((current_offset).saturating_add(size))
                .min(file_size)
                .saturating_sub(current_offset);

            let slice_from_file =
                content[current_offset..(current_offset).saturating_add(reading_length)].to_vec();

            if buffer.len() < reading_length {
                return Err(MemFSErr::bad_memory_access());
            }

            buffer[0..reading_length].copy_from_slice(&slice_from_file);

            self.file_offset.fetch_add(reading_length, Ordering::AcqRel);

            Ok(slice_from_file.len())
        } else {
            Err(MemFSErr::no_such_file_or_directory())
        }
    }

    unsafe fn write_file(&self, buffer: &Vec<u8>, size: usize) -> Result<usize> {
        if self.flag.contains(OpenFlag::O_RDONLY) {
            return Err(MemFSErr::bad_file_descriptor());
        }

        if let MemFSEntry::File(file) = &*self.entry {
            let file_guard = file.data.get();
            let file_content = unsafe { &mut *file_guard };

            
            if self.flag.contains(OpenFlag::O_APPEND) {
                let _lock = self
                    .append_mutex
                    .lock()
                    .map_err(|_| MemFSErr::poisoned_lock());

                let current_offset = file.size.load(Ordering::Acquire);
                
                let writing_content_size = size.min(buffer.len());
                let expected_offset = current_offset.saturating_add(writing_content_size);
                
                if expected_offset > FILE_MAX_SIZE {
                    return Err(MemFSErr::file_too_large());
                }

                // self.file_offset.store(current_offset, Ordering::Release);

                file.size.store(expected_offset, Ordering::Release);

                file_content[current_offset..expected_offset]
                    .copy_from_slice(&buffer[0..writing_content_size]);

                self.file_offset.store(expected_offset, Ordering::Release);

                Ok(writing_content_size)
            } else {
                let current_offset = self.file_offset.load(Ordering::Acquire);
                let writing_content_size = size.min(buffer.len());
                let expected_offset = current_offset.saturating_add(writing_content_size);

                if expected_offset > FILE_MAX_SIZE {
                    return Err(MemFSErr::file_too_large());
                }

                file.size.fetch_max(expected_offset, Ordering::Relaxed);

                file_content[current_offset..expected_offset]
                    .copy_from_slice(&buffer[0..writing_content_size]);

                self.file_offset.store(expected_offset, Ordering::Release);

                Ok(writing_content_size)
            }
        } else {
            Err(MemFSErr::no_such_file_or_directory())
        }
    }

    unsafe fn seek_file(&self, seek_position: usize, flag: SeekFlag) -> Result<usize> {
        let current_offset = self.file_offset.load(Ordering::Acquire);

        let maximum_offset = if let MemFSEntry::File(file) = &*self.entry {
            file.size.load(Ordering::Acquire)
        }
        else {
            return Err(MemFSErr::is_directory())
        };

        let additional_offset = match flag {
            SeekFlag::SEEK_CUR => current_offset,
            SeekFlag::SEEK_END => maximum_offset,
            SeekFlag::SEEK_SET => 0,
        };

        let final_offset =  maximum_offset.min(additional_offset.saturating_add(seek_position));
        self.file_offset.store(final_offset, Ordering::Release);

        Ok(final_offset)
    }
}
