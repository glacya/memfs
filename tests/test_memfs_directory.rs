use memfs::memfs::MemFS;
use memfs::utils::{MemFSErrType, OpenFlag, generate_random_vector};

#[test]
fn test_should_success_on_simple_mkdir() {
    let fs = MemFS::new();

    let result = fs.mkdir("/test_dir");

    assert!(result.is_ok());
}

#[test]
fn test_should_fail_when_mkdir_on_nonexistent_path() {
    let fs = MemFS::new();

    let result = fs.mkdir("/nonexistent/dir");

    assert!(result.is_err_and(|e| { matches!(e.err_type, MemFSErrType::ENOENT) }));
}

#[test]
fn test_should_success_when_rmdir_on_existing_path() {
    let fs = MemFS::new();

    let mkdir_result = fs.mkdir("/dir1");
    let rmdir_result = fs.rmdir("/dir1");
    let mkdir_again_result = fs.mkdir("/dir1");

    assert!(mkdir_result.is_ok());
    assert!(rmdir_result.is_ok());
    assert!(mkdir_again_result.is_ok());
}

#[test]
fn test_should_fail_when_rmdir_on_nonempty_path() {
    let fs = MemFS::new();

    // Create some directories, and a file.
    fs.mkdir("/dir1").unwrap();
    fs.mkdir("/dir1/dir2").unwrap();
    fs.mkdir("/dir1/dir3").unwrap();
    let fd = fs
        .open(
            "/dir1/dir2/quack.duck",
            OpenFlag::O_CREAT | OpenFlag::O_RDONLY,
        )
        .unwrap();
    fs.close(fd).unwrap();

    // Try rmdir on directories.
    let empty_rmdir = fs.rmdir("/dir1/dir3");
    let nonempty_rmdir1 = fs.rmdir("/dir1");
    let nonempty_rmdir2 = fs.rmdir("/dir1/dir2");

    assert!(empty_rmdir.is_ok());
    assert!(nonempty_rmdir1.is_err_and(|e| { matches!(e.err_type, MemFSErrType::ENOTEMPTY) }));
    assert!(nonempty_rmdir2.is_err_and(|e| { matches!(e.err_type, MemFSErrType::ENOTEMPTY) }));

    // Try again after removing a file.
    fs.unlink("/dir1/dir2/quack.duck").unwrap();

    let now_rmdir1 = fs.rmdir("/dir1/dir2");
    let now_rmdir2 = fs.rmdir("/dir1");

    assert!(now_rmdir1.is_ok());
    assert!(now_rmdir2.is_ok());
}

#[test]
fn test_should_fail_on_mkdir_with_existing_file_name() {
    let fs = MemFS::new();
    let file_name = "/noodle";

    let fd = fs
        .open(file_name, OpenFlag::O_CREAT | OpenFlag::O_RDONLY)
        .unwrap();
    fs.close(fd).unwrap();

    let mkdir_with_existing_file_name = fs.mkdir("/noodle");
    assert!(
        mkdir_with_existing_file_name
            .is_err_and(|e| { matches!(e.err_type, MemFSErrType::EEXIST) })
    );

    fs.unlink(file_name).unwrap();

    let mkdir_after_remove = fs.mkdir("/noodle");
    assert!(mkdir_after_remove.is_ok());
}

#[test]
fn test_should_fail_when_rmdir_on_file() {
    let fs = MemFS::new();
    let file_name = "/imfile";

    let fd = fs
        .open(file_name, OpenFlag::O_CREAT | OpenFlag::O_RDONLY)
        .unwrap();
    fs.close(fd).unwrap();

    let rmdir_on_file = fs.rmdir(file_name);

    assert!(rmdir_on_file.is_err_and(|e| { matches!(e.err_type, MemFSErrType::ENOTDIR) }));
}

#[test]
fn test_should_succeed_on_repeated_mkdir_and_rmdir_with_tremendous_levels() {
    let fs = MemFS::new();
    let loops = 256;
    let numbers = generate_random_vector(loops);

    // Create a spire of directories.
    for i in 0..loops {
        let dir_name: String = numbers[0..(i + 1)]
            .iter()
            .map(|v| std::fmt::format(format_args!("/{}", v)))
            .collect();

        let mkdir_result = fs.mkdir(dir_name.as_str());
        assert!(mkdir_result.is_ok());
    }

    // Remove the spire from the deepest to the shallowest.
    for i in (0..loops).rev() {
        let dir_name: String = numbers[0..(i + 1)]
            .iter()
            .map(|v| std::fmt::format(format_args!("/{}", v)))
            .collect();

        let rmdir_result = fs.rmdir(dir_name.as_str());
        assert!(rmdir_result.is_ok());
    }
}
