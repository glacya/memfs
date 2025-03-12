use memfs::memfs::MemFS;
use memfs::utils::{generate_random_vector, MemFSErrType, OpenFlag};

#[test]
fn memfs_mkdir() {
    let fs = MemFS::new();

    let result = fs.mkdir("/test_dir");
    
    assert!(result.is_ok());
}

#[test]
fn memfs_mkdir_nonexistent_directory() {
    let fs = MemFS::new();

    let result = fs.mkdir("/nonexistent/dir");

    assert!(result.is_err_and(|e| {
        matches!(e.err_type, MemFSErrType::ENOENT)
    }));
}

#[test]
fn memfs_rmdir_empty_directory() {
    let fs = MemFS::new();

    let mkdir_result = fs.mkdir("/dir1");
    let rmdir_result = fs.rmdir("/dir1");
    let mkdir_again_result = fs.mkdir("/dir1");

    assert!(mkdir_result.is_ok());
    assert!(rmdir_result.is_ok());
    assert!(mkdir_again_result.is_ok());
}

#[test]
fn memfs_rmdir_nonempty_directory_should_err() {
    let fs = MemFS::new();

    fs.mkdir("/dir1").unwrap();
    fs.mkdir("/dir1/dir2").unwrap();
    fs.mkdir("/dir1/dir3").unwrap();
    let fd = fs.open("/dir1/dir2/quack.duck", OpenFlag::O_CREAT | OpenFlag::O_RDONLY).unwrap();
    fs.close(fd).unwrap();

    let empty_rmdir = fs.rmdir("/dir1/dir3");
    let nonempty_rmdir1 = fs.rmdir("/dir1");
    let nonempty_rmdir2 = fs.rmdir("/dir1/dir2");

    assert!(empty_rmdir.is_ok());
    assert!(nonempty_rmdir1.is_err_and(|e| { matches!(e.err_type, MemFSErrType::ENOTEMPTY)}));
    assert!(nonempty_rmdir2.is_err_and(|e| { matches!(e.err_type, MemFSErrType::ENOTEMPTY)}));

    fs.remove("/dir1/dir2/quack.duck").unwrap();
    
    let now_rmdir1 = fs.rmdir("/dir1/dir2");
    let now_rmdir2 = fs.rmdir("/dir1");

    assert!(now_rmdir1.is_ok());
    assert!(now_rmdir2.is_ok());
}

#[test]
fn memfs_mkdir_with_existing_file_name_should_err() {
    let fs = MemFS::new();

    let fd = fs.open("/noodle", OpenFlag::O_CREAT | OpenFlag::O_RDONLY).unwrap();
    fs.close(fd).unwrap();

    let mkdir_with_existing_file_name = fs.mkdir("/noodle");
    assert!(mkdir_with_existing_file_name.is_err_and(|e| { matches!(e.err_type, MemFSErrType::EEXIST)}));

    fs.remove("/noodle").unwrap();

    let mkdir_after_remove = fs.mkdir("/noodle");
    assert!(mkdir_after_remove.is_ok());
}

#[test]
fn memfs_rmdir_on_file_should_err() {
    let fs = MemFS::new();

    let fd = fs.open("/imfile", OpenFlag::O_CREAT | OpenFlag::O_RDONLY).unwrap();
    fs.close(fd).unwrap();

    let rmdir_on_file = fs.rmdir("/imfile");
    assert!(rmdir_on_file.is_err_and(|e| {
        matches!(e.err_type, MemFSErrType::ENOTDIR)
    }));
}

#[test]
fn memfs_mkdir_and_rmdir_with_tremendous_levels() {
    let fs = MemFS::new();
    let loops = 256;
    let numbers = generate_random_vector(loops);

    // Create a spire of directories.
    for i in 0..loops {
        let dir_name: String = numbers[0..(i + 1)].iter().map(|v| { std::fmt::format(format_args!("/{}", v))}).collect();

        let mkdir_result = fs.mkdir(dir_name.as_str());
        assert!(mkdir_result.is_ok());
    }

    // Remove the spire.
    for i in (0..loops).rev() {
        let dir_name: String = numbers[0..(i + 1)].iter().map(|v| { std::fmt::format(format_args!("/{}", v))}).collect();

        let rmdir_result = fs.rmdir(dir_name.as_str());
        assert!(rmdir_result.is_ok());
    }
}