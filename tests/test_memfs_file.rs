use memfs::memfs::MemFS;

#[test]
fn memfs_create_file() {
    // Arrange
    let fs = MemFS::new();

    // Action
    let result = fs.create("/my_file.txt");

    // Assert
    assert!(result.is_ok());
}

#[test]
fn memfs_create_and_open_file() {
    let fs = MemFS::new();

    let create_result = fs.create("/create_file.sh");

    assert!(create_result.is_ok());

    let open_result = fs.open("/create_file.sh", None);

    assert!(open_result.is_ok());
}

#[test]
fn memfs_create_existing_file_name_should_err() {
    let fs = MemFS::new();

    let first_create = fs.create("/existing.rs");
    
    assert!(first_create.is_ok());

    let second_create = fs.create("/existing.rs");

    assert!(second_create.is_err());
}

#[test]
fn memfs_create_existing_directory_name_should_err() {
    todo!()
}

#[test]
fn memfs_remove_existing_file() {
    let fs = MemFS::new();

    let create_result = fs.create("/example.md");

    assert!(create_result.is_ok());

    let remove_result = fs.create("/example.md");

    assert!(remove_result.is_ok());

    let open_result = fs.open("/example.md", None);

    assert!(open_result.is_err());
}

#[test]
fn memfs_remove_nonexistent_file() {
    let fs = MemFS::new();

    let create_result = fs.create("/file1.c");

    assert!(create_result.is_ok());

    let remove_result = fs.remove("/file1.c");

    assert!(remove_result.is_ok());

    let remove_again_result = fs.remove("/file1.c");

    assert!(remove_again_result.is_err());

    let remove_out_of_nowhere = fs.remove("/file2.c");

    assert!(remove_out_of_nowhere.is_err());
}

#[test]
fn memfs_open_nonexistent_file() {
    let fs = MemFS::new();

    let open_result = fs.open("/non.exist", None);

    assert!(open_result.is_err());
}

#[test]
fn memfs_write_and_read_file() {
    todo!()
}

#[test]
fn memfs_concurrent_write_to_single_file() {
    todo!()
}

#[test]
fn memfs_open_file_in_directory() {
    let fs = MemFS::new();

    let mkdir_result = fs.mkdir("/dir");

    assert!(mkdir_result.is_ok());

    let create_result = fs.create("/dir/fanta.jpg");

    assert!(create_result.is_ok());

    let open_result = fs.open("/dir/fanta.jpg", None);

    assert!(open_result.is_ok());
}

#[test]
fn memfs_open_directory_should_err() {
    let fs = MemFS::new();

    let mkdir_result = fs.mkdir("/memfs");

    assert!(mkdir_result.is_ok());

    let open_result = fs.open("memfs", None);
    
    assert!(open_result.is_err());
}