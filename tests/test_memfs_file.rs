use memfs::memfs::MemFS;
use memfs::utils::{generate_random_vector, MemFSErrType, OpenFlag, SeekFlag};
use rand::Rng;

#[test]
fn memfs_create_file() {
    // Arrange
    let fs = MemFS::new();

    // Action
    let result = fs.open("/my_file.txt", OpenFlag::O_CREAT | OpenFlag::O_RDONLY);

    // Assert
    assert!(result.is_ok());
}

#[test]
fn memfs_open_nonexistent_file_without_o_creat() {
    let fs = MemFS::new();

    let open_result = fs.open("/create_file.sh", OpenFlag::O_RDWR);

    assert!(open_result.is_err_and(|e| { matches!(e.err_type, MemFSErrType::ENOENT)}));
}

#[test]
fn memfs_create_existing_file_name_should_work() {
    let fs = MemFS::new();

    let first_create = fs.open("/existing.rs", OpenFlag::O_CREAT | OpenFlag::O_RDWR);
    
    assert!(first_create.is_ok());

    let second_create = fs.open("/existing.rs", OpenFlag::O_CREAT | OpenFlag::O_RDWR);

    assert!(second_create.is_ok());
}

#[test]
fn memfs_create_existing_directory_name_should_err() {
    let fs = MemFS::new();

    let dir_create = fs.mkdir("/mkdir");

    assert!(dir_create.is_ok());

    let file_create = fs.open("/mkdir", OpenFlag::O_CREAT | OpenFlag::O_RDONLY);

    assert!(file_create.is_err_and(|e| {
        matches!(e.err_type, MemFSErrType::EISDIR)
    }));
}

#[test]
fn memfs_remove_existing_file() {
    let fs = MemFS::new();

    let create_result = fs.open("/example.md", OpenFlag::O_CREAT | OpenFlag::O_RDONLY);

    assert!(create_result.is_ok());

    let remove_result = fs.remove("/example.md");

    assert!(remove_result.is_ok());

    let open_result = fs.open("/example.md", OpenFlag::O_RDONLY);

    assert!(open_result.is_err_and(|e| {
        matches!(e.err_type, MemFSErrType::ENOENT)
    }));
}

#[test]
fn memfs_remove_nonexistent_file() {
    let fs = MemFS::new();

    let create_result = fs.open("/file1.c", OpenFlag::O_CREAT | OpenFlag::O_RDONLY);

    assert!(create_result.is_ok());

    let close_result = fs.close(create_result.unwrap());

    assert!(close_result.is_ok());

    let remove_result = fs.remove("/file1.c");

    assert!(remove_result.is_ok());

    let remove_again_result = fs.remove("/file1.c");

    assert!(remove_again_result.is_err_and(|e| {
        matches!(e.err_type, MemFSErrType::ENOENT)
    }));

    let remove_out_of_nowhere = fs.remove("/file2.c");

    assert!(remove_out_of_nowhere.is_err_and(|e| {
        matches!(e.err_type, MemFSErrType::ENOENT)
    }));
}

#[test]
fn memfs_remove_directory_should_err() {
    let fs = MemFS::new();

    fs.mkdir("/filelike_dir").unwrap();

    let remove_result = fs.remove("/filelike_dir");

    assert!(remove_result.is_err_and(|e| {
        matches!(e.err_type, MemFSErrType::EISDIR)
    }));
}

#[test]
fn memfs_open_file_in_directory() {
    let fs = MemFS::new();

    let mkdir_result = fs.mkdir("/dir");

    assert!(mkdir_result.is_ok());

    let open_result = fs.open("/dir/fanta.jpg", OpenFlag::O_CREAT | OpenFlag::O_RDONLY);

    assert!(open_result.is_ok());
}

#[test]
fn memfs_open_directory_should_err() {
    let fs = MemFS::new();

    let mkdir_result = fs.mkdir("/memfs");

    assert!(mkdir_result.is_ok());

    let open_result = fs.open("/memfs", OpenFlag::O_CREAT | OpenFlag::O_RDONLY);
    
    assert!(open_result.is_err());
}

#[test]
fn memfs_read_on_closed_file_descriptor_should_err() {
    let fs = MemFS::new();
    let buffer_size = 64;
    let mut buffer = vec![0; buffer_size];


    let fd = fs.open("/closing.cls", OpenFlag::O_CREAT | OpenFlag::O_RDONLY).unwrap();
    let close_result = fs.close(fd);

    assert!(close_result.is_ok());

    let read_after_close = fs.read(fd, &mut buffer, buffer_size);

    assert!(read_after_close.is_err_and(|e| {
        matches!(e.err_type, MemFSErrType::EBADF)
    }))
}

#[test]
fn memfs_play_with_multiple_file_descriptors() {
    let fs = MemFS::new();
    let file_name = "/power.ade";
    let buffer_size = 64;
    let batch_size = 8;
    let loops = 256;
    let mut init_buffer = generate_random_vector(buffer_size);

    let init_fd = fs.open(file_name, OpenFlag::O_CREAT | OpenFlag::O_RDWR).unwrap();
    fs.write(init_fd, &init_buffer, buffer_size).unwrap();
    fs.close(init_fd).unwrap();

    let mut fd_vector = Vec::new();

    for _ in 0..loops {
        let fd = fs.open(file_name, OpenFlag::O_WRONLY).unwrap();
        fd_vector.push(fd);

        let random_write_buffer = generate_random_vector(batch_size);
        let random_offset = rand::rng().random_range(0..(buffer_size - batch_size));

        fs.lseek(fd, random_offset, SeekFlag::SEEK_SET).unwrap();
        fs.write(fd, &random_write_buffer, batch_size).unwrap();

        init_buffer[random_offset..(random_offset + batch_size)].copy_from_slice(random_write_buffer.as_slice());
    }

    for fd in fd_vector {
        fs.close(fd).unwrap();
    }

    let final_fd = fs.open(file_name, OpenFlag::O_RDONLY).unwrap();
    let mut final_buffer = vec![0; buffer_size];

    fs.read(final_fd, &mut final_buffer, buffer_size).unwrap();
    fs.close(final_fd).unwrap();

    assert_eq!(init_buffer, final_buffer);
}

#[test]
fn memfs_open_flag_mutual_exclusiveness_test() {
    let fs = MemFS::new();

    let r1 = fs.open("/myfile1.my", OpenFlag::O_CREAT | OpenFlag::O_RDONLY | OpenFlag::O_WRONLY);
    let r2 = fs.open("/myfile2.my", OpenFlag::O_CREAT | OpenFlag::O_RDONLY | OpenFlag::O_RDWR);
    let r3 = fs.open("/myfile3.my", OpenFlag::O_CREAT | OpenFlag::O_WRONLY | OpenFlag::O_RDWR);
    let r4 = fs.open("/myfile4.my", OpenFlag::O_CREAT | OpenFlag::O_RDONLY | OpenFlag::O_RDWR | OpenFlag::O_WRONLY);
    let r5 = fs.open("/myfile5.my", OpenFlag::O_CREAT | OpenFlag::O_RDONLY);
    let r6 = fs.open("/myfile6.my", OpenFlag::O_CREAT | OpenFlag::O_RDWR);
    let r7 = fs.open("/myfile7.my", OpenFlag::O_CREAT | OpenFlag::O_WRONLY);
    let r8 = fs.open("/myfile8.my", OpenFlag::O_CREAT);

    assert!(r1.is_err_and(|e| { matches!(e.err_type, MemFSErrType::EINVAL)}));
    assert!(r2.is_err_and(|e| { matches!(e.err_type, MemFSErrType::EINVAL)}));
    assert!(r3.is_err_and(|e| { matches!(e.err_type, MemFSErrType::EINVAL)}));
    assert!(r4.is_err_and(|e| { matches!(e.err_type, MemFSErrType::EINVAL)}));
    assert!(r5.is_ok());
    assert!(r6.is_ok());
    assert!(r7.is_ok());
    assert!(r8.is_err_and(|e| { matches!(e.err_type, MemFSErrType::EINVAL)}));
}

// MemFS does not allow file offset over file size.
#[test]
fn memfs_seek_offset() {
    let fs = MemFS::new();
    let file_size = 64;
    let random_buffer = generate_random_vector(file_size);
    let random_offset = rand::rng().random_range(0..file_size);

    let open_result = fs.open("/kaist.cp", OpenFlag::O_CREAT | OpenFlag::O_RDWR);
    assert!(open_result.is_ok());

    
    let fd = open_result.unwrap();

    let write_result = fs.write(fd, &random_buffer, file_size);
    assert!(write_result.is_ok());


    let offset1 = fs.lseek(fd, random_offset, SeekFlag::SEEK_SET).unwrap();
    let offset2 = fs.lseek(fd, random_offset, SeekFlag::SEEK_CUR).unwrap();
    let offset3 = fs.lseek(fd, random_offset, SeekFlag::SEEK_END).unwrap();

    assert_eq!(offset1, random_offset);
    assert_eq!(offset2, (2 * random_offset).min(file_size));
    assert_eq!(offset3, file_size);
}

#[test]
fn memfs_write_and_read_file_with_seek() {
    let fs = MemFS::new();
    let buffer_size = 64;
    let write_size = 8;

    let open_result = fs.open("/subject.sj", OpenFlag::O_CREAT | OpenFlag::O_RDWR);
    assert!(open_result.is_ok());

    let fd = open_result.unwrap();
    let mut random_buffer = generate_random_vector(buffer_size);

    fs.write(fd, &random_buffer, buffer_size).unwrap();
    fs.lseek(fd, 0, SeekFlag::SEEK_SET).unwrap();

    for _ in 0..100 {
        let random_seek_offset = rand::rng().random_range(0..(buffer_size - write_size));
        let write_random_buffer = generate_random_vector(write_size);

        // Write random data, on random position.
        fs.lseek(fd, random_seek_offset, SeekFlag::SEEK_SET).unwrap();
        fs.write(fd, &write_random_buffer, write_size).unwrap();

        // Modify original buffer too, for comparison
        random_buffer[random_seek_offset..(random_seek_offset + write_size)].copy_from_slice(write_random_buffer.as_slice());

        let mut reading_buffer = vec![0; buffer_size];

        // Read whole file, and check content.
        fs.lseek(fd, 0, SeekFlag::SEEK_SET).unwrap();
        fs.read(fd, &mut reading_buffer, buffer_size).unwrap();

        assert_eq!(reading_buffer, random_buffer);

        // Set file offset to zero for next iteration.
        fs.lseek(fd, 0, SeekFlag::SEEK_SET).unwrap();
    }

    fs.close(fd).unwrap();
}

#[test]
fn memfs_test_write_only_file() {
    let fs = MemFS::new();
    let buffer_size = 16;

    let write_only_open = fs.open("/write.f2", OpenFlag::O_CREAT | OpenFlag::O_WRONLY);
    assert!(write_only_open.is_ok());
    let write_only_fd = write_only_open.unwrap();

    let random_buffer = generate_random_vector(buffer_size);
    let write_result = fs.write(write_only_fd, &random_buffer, buffer_size);
    
    assert!(write_result.is_ok());

    let mut placeholder_buffer = vec![0; buffer_size];
    let read_result = fs.read(write_only_fd, &mut placeholder_buffer, buffer_size);

    assert!(read_result.is_err_and(|e| {
        matches!(e.err_type, MemFSErrType::EBADF)
    }))
    
}

#[test]
fn memfs_test_read_only_file() {
    let fs = MemFS::new();
    let buffer_size = 64;

    let initial_open = fs.open("/victim.vic", OpenFlag::O_CREAT | OpenFlag::O_RDWR);

    assert!(initial_open.is_ok());

    let initial_fd = initial_open.unwrap();

    let random_buffer = generate_random_vector(buffer_size);
    let write_result = fs.write(initial_fd, &random_buffer, buffer_size);
    assert!(write_result.is_ok());

    let close_result = fs.close(initial_fd);
    assert!(close_result.is_ok());

    let mut buffer = vec![0; buffer_size];

    let read_only_open = fs.open("/victim.vic", OpenFlag::O_RDONLY);
    assert!(read_only_open.is_ok());
    let read_only_fd = read_only_open.unwrap();

    let read_result = fs.read(read_only_fd, &mut buffer, buffer_size);
    
    assert!(read_result.is_ok());
    assert_eq!(random_buffer, buffer);

    let write_on_read_only = fs.write(read_only_fd, &random_buffer, buffer_size);
    assert!(write_on_read_only.is_err_and(|e| {
        matches!(e.err_type, MemFSErrType::EBADF)
    }));
}


#[test]
fn memfs_read_more_than_buffer_size_should_err() {
    let fs = MemFS::new();
    let file_size = 256;
    let buffer_size = 64;

    let initial_open = fs.open("/dontreadtoomuch", OpenFlag::O_CREAT | OpenFlag::O_RDWR);

    assert!(initial_open.is_ok());

    let initial_fd = initial_open.unwrap();

    let random_buffer = generate_random_vector(file_size);
    let write_result = fs.write(initial_fd, &random_buffer, file_size);
    assert!(write_result.is_ok());

    let close_result = fs.close(initial_fd);
    assert!(close_result.is_ok());


    let mut read_buffer = vec![0; buffer_size];

    let read_open = fs.open("/dontreadtoomuch", OpenFlag::O_RDONLY);
    assert!(read_open.is_ok());

    let read_result = fs.read(read_open.unwrap(), &mut read_buffer, file_size);
    assert!(read_result.is_err_and(|e| {
        matches!(e.err_type, MemFSErrType::EFAULT)
    }));
}

#[test]
fn memfs_write_over_file_size() {
    let fs = MemFS::new();
    let small_buffer_size = 64;
    let large_buffer_size = 256;

    let initial_open = fs.open("/enlarge.lag", OpenFlag::O_CREAT | OpenFlag::O_RDWR);

    assert!(initial_open.is_ok());

    let initial_fd = initial_open.unwrap();

    let random_buffer = generate_random_vector(small_buffer_size);
    let write_result = fs.write(initial_fd, &random_buffer, small_buffer_size);
    assert!(write_result.is_ok());

    let close_result = fs.close(initial_fd);
    assert!(close_result.is_ok());

    
    let write_open = fs.open("/enlarge.lag", OpenFlag::O_WRONLY);
    assert!(write_open.is_ok());

    let write_fd = write_open.unwrap();

    let large_random_buffer = generate_random_vector(large_buffer_size);
    let large_write_result = fs.write(write_fd, &large_random_buffer, large_buffer_size);
    assert!(large_write_result.is_ok_and(|v| {
        v == large_buffer_size
    }));

    let write_close = fs.close(write_fd);
    assert!(write_close.is_ok());

}