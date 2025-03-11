use memfs::memfs::MemFS;

#[test]
fn memfs_mkdir() {
    let fs = MemFS::new();

    let result = fs.mkdir("/test_dir");
    
    assert!(result.is_ok());
}

#[test]
fn memfs_mkdir_nonexistent_directory() {

}

#[test]
fn memfs_rmdir_empty_directory() {

}

#[test]
fn memfs_rmdir_nonempty_directory_should_err() {

}

#[test]
fn memfs_mkdir_with_existing_file_name_should_err() {

}

#[test]
fn memfs_rmdir_on_file_should_err() {

}

#[test]
fn memfs_mkdir_multiple_layer() {

}

#[test]
fn memfs_concurrent_mkdir() {

}