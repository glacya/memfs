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
fn test_should_fail_on_mkdir_with_empty_path() {
    let fs = MemFS::new();

    let result = fs.mkdir("");

    assert!(result.is_err_and(|e| { matches!(e.err_type, MemFSErrType::ENOENT) }));
}

#[test]
fn test_should_fail_on_mkdir_with_existing_file_name() {
    /* Arrange */

    let fs = MemFS::new();
    let file_name = "/noodle";

    let fd = fs
        .open(file_name, OpenFlag::O_CREAT | OpenFlag::O_RDONLY)
        .unwrap();
    fs.close(fd).unwrap();

    /* Action */

    let mkdir_with_existing_file_name = fs.mkdir("/noodle");
    fs.unlink(file_name).unwrap();
    let mkdir_after_remove = fs.mkdir("/noodle");

    /* Assert */

    assert!(
        mkdir_with_existing_file_name
            .is_err_and(|e| { matches!(e.err_type, MemFSErrType::EEXIST) })
    );
    assert!(mkdir_after_remove.is_ok());
}

#[test]
fn test_should_fail_on_mkdir_with_root_path() {
    let fs = MemFS::new();

    let root_mkdir = fs.mkdir("/");

    assert!(root_mkdir.is_err_and(|e| { matches!(e.err_type, MemFSErrType::EEXIST) }));
}

#[test]
fn test_should_fail_when_mkdir_with_name_of_dots() {
    let fs = MemFS::new();

    let mkdir_self = fs.mkdir(".");
    let mkdir_parent = fs.mkdir("..");

    assert!(mkdir_self.is_err_and(|e| { matches!(e.err_type, MemFSErrType::EEXIST) }));
    assert!(mkdir_parent.is_err_and(|e| { matches!(e.err_type, MemFSErrType::EEXIST) }));
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
    /* Arrange */

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

    /* Action */

    // Try rmdir on directories.
    let empty_rmdir = fs.rmdir("/dir1/dir3");
    let nonempty_rmdir1 = fs.rmdir("/dir1");
    let nonempty_rmdir2 = fs.rmdir("/dir1/dir2");

    // Try again after removing a file.
    fs.unlink("/dir1/dir2/quack.duck").unwrap();

    let now_rmdir1 = fs.rmdir("/dir1/dir2");
    let now_rmdir2 = fs.rmdir("/dir1");

    /* Assert */

    assert!(empty_rmdir.is_ok());
    assert!(nonempty_rmdir1.is_err_and(|e| { matches!(e.err_type, MemFSErrType::ENOTEMPTY) }));
    assert!(nonempty_rmdir2.is_err_and(|e| { matches!(e.err_type, MemFSErrType::ENOTEMPTY) }));
    assert!(now_rmdir1.is_ok());
    assert!(now_rmdir2.is_ok());
}

#[test]
fn test_should_fail_when_rmdir_on_file() {
    /* Arrange */

    let fs = MemFS::new();
    let file_name = "/imfile";

    let fd = fs
        .open(file_name, OpenFlag::O_CREAT | OpenFlag::O_RDONLY)
        .unwrap();
    fs.close(fd).unwrap();

    /* Action */

    let rmdir_on_file = fs.rmdir(file_name);

    /* Assert */

    assert!(rmdir_on_file.is_err_and(|e| { matches!(e.err_type, MemFSErrType::ENOTDIR) }));
}

#[test]
fn test_should_fail_on_rmdir_with_empty_path() {
    let fs = MemFS::new();

    let rmdir_empty_path = fs.rmdir("");

    assert!(rmdir_empty_path.is_err_and(|e| { matches!(e.err_type, MemFSErrType::ENOENT) }));
}

#[test]
fn test_should_fail_on_rmdir_with_root_path() {
    let fs = MemFS::new();

    let rmdir_empty_path = fs.rmdir("/");

    assert!(rmdir_empty_path.is_err_and(|e| { matches!(e.err_type, MemFSErrType::EBUSY) }));
}

#[test]
fn test_should_fail_on_rmdir_when_last_element_of_path_is_dots() {
    let fs = MemFS::new();

    let rmdir_last_self = fs.rmdir("/.");
    let rmdir_last_parent = fs.rmdir("/..");

    assert!(rmdir_last_self.is_err_and(|e| { matches!(e.err_type, MemFSErrType::EINVAL) }));
    assert!(rmdir_last_parent.is_err_and(|e| { matches!(e.err_type, MemFSErrType::ENOTEMPTY) }));
}

#[test]
fn test_should_succeed_on_repeated_mkdir_and_rmdir_with_tremendous_levels() {
    /* Arrange */

    let fs = MemFS::new();
    let loops = 256;
    let numbers = generate_random_vector(loops);

    let first_path = std::fmt::format(format_args!("/{}", numbers[0]));
    fs.mkdir(first_path.as_str()).unwrap();

    /* Action */

    // Create a spire of directories.
    for i in 1..loops {
        let dir_name: String = numbers[0..(i + 1)]
            .iter()
            .map(|v| std::fmt::format(format_args!("/{}", v)))
            .collect();

        fs.mkdir(dir_name.as_str()).unwrap();
    }

    // Remove the spire from the deepest to the shallowest.
    for i in (1..loops).rev() {
        let dir_name: String = numbers[0..(i + 1)]
            .iter()
            .map(|v| std::fmt::format(format_args!("/{}", v)))
            .collect();

        fs.rmdir(dir_name.as_str()).unwrap();
    }

    // Now check if the directory is empty.
    let remove_first_path = fs.rmdir(first_path.as_str());

    /* Assert */

    assert!(remove_first_path.is_ok());
}

#[test]
fn test_should_succeed_on_basic_chdir() {
    /* Arrange */

    let mut fs = MemFS::new();
    let dir1 = "/river";
    let dir2 = "/river/ocean";
    let file_name = "/river/ocean/sky.sk";

    fs.mkdir(dir1).unwrap();
    fs.mkdir(dir2).unwrap();
    let fd = fs
        .open(file_name, OpenFlag::O_CREAT | OpenFlag::O_RDONLY)
        .unwrap();
    fs.close(fd).unwrap();

    /* Action */

    let chdir1 = fs.chdir(dir1);
    let check_dir2_exist = fs.mkdir(dir2);
    let chdir2 = fs.chdir(dir2);
    let check_file_exist = fs.unlink(file_name);

    /* Assert */

    assert!(chdir1.is_ok());
    assert!(check_dir2_exist.is_err_and(|e| { matches!(e.err_type, MemFSErrType::EEXIST) }));
    assert!(chdir2.is_ok());
    assert!(check_file_exist.is_ok());
}

#[test]
fn test_should_fail_when_chdir_to_nonexistent_directory() {
    /* Arrange */

    let mut fs = MemFS::new();
    let dir1 = "/kaist";
    let dir2 = "postech";
    let ghost_dir = "snu";

    fs.mkdir(dir1).unwrap();
    fs.chdir(dir1).unwrap();
    fs.mkdir(dir2).unwrap();
    fs.chdir(dir2).unwrap();

    /* Action */

    let chdir1 = fs.chdir(ghost_dir);
    let chdir2 = fs.chdir(dir1);
    let rmdir1 = fs.rmdir(dir2);
    let chdir3 = fs.chdir(dir2);

    /* Assert */

    assert!(chdir1.is_err_and(|e| { matches!(e.err_type, MemFSErrType::ENOENT) }));
    assert!(chdir2.is_ok());
    assert!(rmdir1.is_ok());
    assert!(chdir3.is_err_and(|e| { matches!(e.err_type, MemFSErrType::ENOENT) }));
}

#[test]
fn test_should_succeed_when_chdir_to_self_and_parent() {
    /* Arrange */

    let mut fs = MemFS::new();
    let parent_name = "parent_folder";
    let file_name = "place.holder";
    fs.mkdir(parent_name).unwrap();
    fs.chdir(parent_name).unwrap();
    let fd = fs
        .open(file_name, OpenFlag::O_CREAT | OpenFlag::O_RDONLY)
        .unwrap();
    fs.close(fd).unwrap();

    /* Action */

    let chdir_self = fs.chdir(".");
    let self_test = fs.open(
        file_name,
        OpenFlag::O_CREAT | OpenFlag::O_EXCL | OpenFlag::O_RDONLY,
    );
    let chdir_parent = fs.chdir("..");
    let parent_test = fs.mkdir(parent_name);
    let chdir_parent_on_root = fs.chdir("..");
    let root_test = fs.mkdir(parent_name);

    /* Assert */

    assert!(chdir_self.is_ok());
    assert!(self_test.is_err_and(|e| { matches!(e.err_type, MemFSErrType::EEXIST) }));
    assert!(chdir_parent.is_ok());
    assert!(parent_test.is_err_and(|e| { matches!(e.err_type, MemFSErrType::EEXIST) }));
    assert!(chdir_parent_on_root.is_ok());
    assert!(root_test.is_err_and(|e| { matches!(e.err_type, MemFSErrType::EEXIST) }));
}

#[test]
fn test_should_fail_when_chdir_to_file() {
    /* Arrange */

    let mut fs = MemFS::new();
    let dir = "dir";
    let dir_file = "dir/flie";
    let file = "flie";
    let path_with_file = "flie/nonex";

    fs.mkdir(dir).unwrap();
    let fd = fs
        .open(dir_file, OpenFlag::O_CREAT | OpenFlag::O_RDONLY)
        .unwrap();
    fs.close(fd).unwrap();
    fs.chdir(dir).unwrap();

    /* Action */

    let chdir_to_file = fs.chdir(file);
    let chdir_to_path_with_file_component = fs.chdir(path_with_file);

    /* Assert */

    println!("{:?}", chdir_to_file);
    println!("{:?}", chdir_to_path_with_file_component);

    assert!(chdir_to_file.is_err_and(|e| { matches!(e.err_type, MemFSErrType::ENOTDIR) }));
    assert!(
        chdir_to_path_with_file_component
            .is_err_and(|e| { matches!(e.err_type, MemFSErrType::ENOTDIR) })
    );
}

#[test]
fn test_should_succeed_on_parsing_paths_with_strange_slashes() {
    let mut fs = MemFS::new();

    let r1 = fs.mkdir("////one");
    let r2 = fs.mkdir("///one//two");
    let r3 = fs.mkdir("/one///two//////////three");
    let r4 = fs.mkdir("/////////one/two/three//four/////");
    let r5 = fs.chdir("one");
    let r6 = fs.chdir("two/////three////");
    let r7 = fs.chdir("four/");
    let r8 = fs.chdir("..////..//.///.//..");
    let r9 = fs.chdir("///one////");
    let r10 = fs.rmdir("two//////three////////////");
    let r11 = fs.mkdir("//one///zero");
    let r12 = fs.open(
        "..//one//zero/fin.txt",
        OpenFlag::O_CREAT | OpenFlag::O_RDONLY,
    );
    let r13 = fs.chdir("two//three/four");
    let r14 = fs.unlink("..//.././..///./..//..///..//one////zero//fin.txt");

    assert!(r1.is_ok());
    assert!(r2.is_ok());
    assert!(r3.is_ok());
    assert!(r4.is_ok());
    assert!(r5.is_ok());
    assert!(r6.is_ok());
    assert!(r7.is_ok());
    assert!(r8.is_ok());
    assert!(r9.is_ok());
    assert!(r10.is_err_and(|e| { matches!(e.err_type, MemFSErrType::ENOTEMPTY) }));
    assert!(r11.is_ok());
    assert!(r12.is_ok());
    assert!(r13.is_ok());
    assert!(r14.is_ok());
}

#[test]
fn test_should_succeed_on_mkdir_and_chdir_with_tremendous_levels() {
    /* Arrange */

    let mut fs = MemFS::new();
    let loops = 256;
    let numbers = generate_random_vector(loops);

    let first_path = numbers[0].to_string();
    fs.mkdir(first_path.as_str()).unwrap();
    fs.chdir(first_path.as_str()).unwrap();

    /* Action */

    // Create a spire of directories.
    for i in 1..loops {
        let dir_name = numbers[i].to_string();

        fs.mkdir(dir_name.as_str()).unwrap();
        fs.chdir(dir_name.as_str()).unwrap();
    }

    let entire_path = numbers
        .iter()
        .map(|x| std::fmt::format(format_args!("/{}", x)))
        .fold("".to_string(), |a, b| a + b.as_str());

    let chdir_root = fs.chdir("/");
    let chdir_deepest = fs.chdir(entire_path.as_str());

    // Remove the spire from the deepest to the shallowest.
    for i in (1..loops).rev() {
        let dir_name: String = numbers[i].to_string();

        fs.chdir("..").unwrap();
        fs.rmdir(dir_name.as_str()).unwrap();
    }

    // Now check if the directory is empty.
    fs.chdir("..").unwrap();
    let remove_first_path = fs.rmdir(first_path.as_str());

    /* Assert */

    assert!(chdir_root.is_ok());
    assert!(chdir_deepest.is_ok());
    assert!(remove_first_path.is_ok());
}
