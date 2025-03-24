use memfs::memfs::MemFS;
use memfs::utils::{MemFSErrType, OpenFlag, SeekFlag, generate_random_vector};
use rand::Rng;

#[test]
fn test_should_succeed_when_creating_file() {
    // Arrange
    let fs = MemFS::new();

    // Action
    let result = fs.open("/my_file.txt", OpenFlag::O_CREAT | OpenFlag::O_RDONLY);

    // Assert
    assert!(result.is_ok());
}

#[test]
fn test_should_fail_on_opening_empty_path() {
    let fs = MemFS::new();

    let result = fs.open("", OpenFlag::O_CREAT | OpenFlag::O_RDONLY);

    assert!(result.is_err_and(|e| { matches!(e.err_type, MemFSErrType::ENOENT )}))
}

#[test]
fn test_should_fail_when_opening_nonexistent_file_without_o_creat() {
    let fs = MemFS::new();

    let open_result = fs.open("/create_file.sh", OpenFlag::O_RDWR);

    assert!(open_result.is_err_and(|e| { matches!(e.err_type, MemFSErrType::ENOENT) }));
}

#[test]
fn test_should_succeed_when_creating_existing_file_name() {
    let fs = MemFS::new();
    fs.open("/existing.rs", OpenFlag::O_CREAT | OpenFlag::O_RDWR)
        .unwrap();

    let second_create = fs.open("/existing.rs", OpenFlag::O_CREAT | OpenFlag::O_RDWR);

    assert!(second_create.is_ok());
}

#[test]
fn test_should_fail_when_creating_existing_directory_name() {
    let fs = MemFS::new();
    fs.mkdir("/mkdir").unwrap();

    let file_create = fs.open("/mkdir", OpenFlag::O_CREAT | OpenFlag::O_RDONLY);

    assert!(file_create.is_err_and(|e| { matches!(e.err_type, MemFSErrType::EISDIR) }));
}

#[test]
fn test_should_succeed_when_opening_file_in_directory() {
    let fs = MemFS::new();

    let mkdir_result = fs.mkdir("/dir");
    let open_result = fs.open("/dir/fanta.jpg", OpenFlag::O_CREAT | OpenFlag::O_RDONLY);

    assert!(mkdir_result.is_ok());
    assert!(open_result.is_ok());
}

#[test]
fn test_should_fail_when_opening_directory_instead_of_file() {
    let fs = MemFS::new();
    fs.mkdir("/memfs").unwrap();

    let open_result = fs.open("/memfs", OpenFlag::O_CREAT | OpenFlag::O_RDONLY);

    assert!(open_result.is_err_and(|e| { matches!(e.err_type, MemFSErrType::EISDIR) }));
}

#[test]
fn test_should_fail_when_opening_path_of_which_middle_component_is_file_instead_of_directory() {
    let fs = MemFS::new();
    fs.mkdir("/dir1").unwrap();
    let fd = fs.open("/dir1/dir2", OpenFlag::O_CREAT | OpenFlag::O_RDONLY).unwrap();
    fs.close(fd).unwrap();

    let open_result = fs.open("/dir1/dir2/file", OpenFlag::O_RDWR);

    assert!(open_result.is_err_and(|e| { matches!(e.err_type, MemFSErrType::ENOTDIR)}));
}

#[test]
fn test_check_combinations_of_mutually_exclusive_open_flags() {
    let fs = MemFS::new();

    // O_RDONLY, O_WRONLY, O_RDWR are mutually exclusive flags, so when opening a file only one of them should be applied.
    let r1 = fs.open(
        "/myfile1.my",
        OpenFlag::O_CREAT | OpenFlag::O_RDONLY | OpenFlag::O_WRONLY,
    );
    let r2 = fs.open(
        "/myfile2.my",
        OpenFlag::O_CREAT | OpenFlag::O_RDONLY | OpenFlag::O_RDWR,
    );
    let r3 = fs.open(
        "/myfile3.my",
        OpenFlag::O_CREAT | OpenFlag::O_WRONLY | OpenFlag::O_RDWR,
    );
    let r4 = fs.open(
        "/myfile4.my",
        OpenFlag::O_CREAT | OpenFlag::O_RDONLY | OpenFlag::O_RDWR | OpenFlag::O_WRONLY,
    );
    let r5 = fs.open("/myfile5.my", OpenFlag::O_CREAT | OpenFlag::O_RDONLY);
    let r6 = fs.open("/myfile6.my", OpenFlag::O_CREAT | OpenFlag::O_RDWR);
    let r7 = fs.open("/myfile7.my", OpenFlag::O_CREAT | OpenFlag::O_WRONLY);
    let r8 = fs.open("/myfile8.my", OpenFlag::O_CREAT);

    assert!(r1.is_err_and(|e| { matches!(e.err_type, MemFSErrType::EINVAL) }));
    assert!(r2.is_err_and(|e| { matches!(e.err_type, MemFSErrType::EINVAL) }));
    assert!(r3.is_err_and(|e| { matches!(e.err_type, MemFSErrType::EINVAL) }));
    assert!(r4.is_err_and(|e| { matches!(e.err_type, MemFSErrType::EINVAL) }));
    assert!(r5.is_ok());
    assert!(r6.is_ok());
    assert!(r7.is_ok());
    assert!(r8.is_err_and(|e| { matches!(e.err_type, MemFSErrType::EINVAL) }));
}

/// If O_EXCL is provided along with O_CREAT, the file must be created.
/// If the file with the same name already exists, open() call must fail.
#[test]
fn test_should_fail_when_creating_file_with_existing_file_name_again_with_o_creat_and_o_excl_flag()
{
    let fs = MemFS::new();
    let file_name = "/excl.creat";
    let fd = fs
        .open(
            file_name,
            OpenFlag::O_CREAT | OpenFlag::O_EXCL | OpenFlag::O_RDWR,
        )
        .unwrap();
    fs.close(fd).unwrap();

    let create_with_excl = fs.open(
        file_name,
        OpenFlag::O_CREAT | OpenFlag::O_EXCL | OpenFlag::O_RDWR,
    );

    assert!(create_with_excl.is_err_and(|e| { matches!(e.err_type, MemFSErrType::EEXIST) }))
}

#[test]
fn test_should_fail_when_closing_invalid_file_descriptor() {
    let fs = MemFS::new();
    let file_name = "/nonex.istent";
    let closing_fd = fs
        .open(file_name, OpenFlag::O_CREAT | OpenFlag::O_RDWR)
        .unwrap();
    let nonexistent_fd = closing_fd + 1;
    fs.close(closing_fd).unwrap();

    let close_again = fs.close(closing_fd);
    let close_out_of_nowhere = fs.close(nonexistent_fd);

    assert!(close_again.is_err_and(|e| { matches!(e.err_type, MemFSErrType::EBADF) }));
    assert!(close_out_of_nowhere.is_err_and(|e| { matches!(e.err_type, MemFSErrType::EBADF) }));
}

#[test]
fn test_should_succeed_when_removing_existing_file() {
    let fs = MemFS::new();
    fs.open("/example.md", OpenFlag::O_CREAT | OpenFlag::O_RDONLY)
        .unwrap();
    fs.unlink("/example.md").unwrap();

    let open_result = fs.open("/example.md", OpenFlag::O_RDONLY);

    assert!(open_result.is_err_and(|e| { matches!(e.err_type, MemFSErrType::ENOENT) }));
}

#[test]
fn test_should_fail_when_removing_nonexistent_file() {
    let fs = MemFS::new();
    let fd = fs
        .open("/file1.c", OpenFlag::O_CREAT | OpenFlag::O_RDONLY)
        .unwrap();
    fs.close(fd).unwrap();
    fs.unlink("/file1.c").unwrap();

    let remove_again_result = fs.unlink("/file1.c");
    let remove_out_of_nowhere = fs.unlink("/file2.c");

    assert!(remove_again_result.is_err_and(|e| { matches!(e.err_type, MemFSErrType::ENOENT) }));
    assert!(remove_out_of_nowhere.is_err_and(|e| { matches!(e.err_type, MemFSErrType::ENOENT) }));
}

#[test]
fn test_should_fail_when_removing_directory_instead_of_file() {
    let fs = MemFS::new();
    fs.mkdir("/filelike_dir").unwrap();

    let remove_result = fs.unlink("/filelike_dir");

    assert!(remove_result.is_err_and(|e| { matches!(e.err_type, MemFSErrType::EISDIR) }));
}

#[test]
fn test_should_fail_on_removing_empty_path() {
    let fs = MemFS::new();
    
    let remove_result = fs.unlink("");

    assert!(remove_result.is_err_and(|e| { matches!(e.err_type, MemFSErrType::ENOENT )}));
}

#[test]
fn test_should_fail_when_reading_from_closed_file_descriptor() {
    let fs = MemFS::new();
    let buffer_size = 64;
    let mut buffer = vec![0; buffer_size];
    let fd = fs
        .open("/closing.cls", OpenFlag::O_CREAT | OpenFlag::O_RDONLY)
        .unwrap();
    fs.close(fd).unwrap();

    let read_after_close = fs.read(fd, &mut buffer, buffer_size);

    assert!(read_after_close.is_err_and(|e| { matches!(e.err_type, MemFSErrType::EBADF) }))
}

// Note that MemFS does not allow file offset over file size.
#[test]
fn test_check_lseek_offset_values_with_different_seekflag() {
    /* Arrange */

    let fs = MemFS::new();
    let file_size = 64;
    let random_buffer = generate_random_vector(file_size);
    let random_offset = rand::rng().random_range(0..file_size);

    let fd = fs
        .open("/kaist.cp", OpenFlag::O_CREAT | OpenFlag::O_RDWR)
        .unwrap();
    fs.write(fd, &random_buffer, file_size).unwrap();

    /* Action */

    let offset1 = fs.lseek(fd, random_offset, SeekFlag::SEEK_SET).unwrap();
    let offset2 = fs.lseek(fd, random_offset, SeekFlag::SEEK_CUR).unwrap();
    let offset3 = fs.lseek(fd, random_offset, SeekFlag::SEEK_END).unwrap();

    /* Assert */

    assert_eq!(offset1, random_offset);
    assert_eq!(offset2, (2 * random_offset).min(file_size));
    assert_eq!(offset3, file_size);
}

#[test]
fn test_should_succeed_on_basic_reading_and_writing_on_file() {
    /* Arrange */

    let fs = MemFS::new();
    let file_name = "/basic.sic";
    let buffer_size = 64;
    let random_buffer = generate_random_vector(buffer_size);
    let mut reading_buffer = vec![0; buffer_size];
    let fd = fs
        .open(file_name, OpenFlag::O_CREAT | OpenFlag::O_RDWR)
        .unwrap();

    /* Action */

    let write_result = fs.write(fd, &random_buffer, buffer_size);
    let read_result_without_seek = fs.read(fd, &mut reading_buffer, buffer_size);
    fs.lseek(fd, 0, SeekFlag::SEEK_SET).unwrap();
    let read_result_after_seek = fs.read(fd, &mut reading_buffer, buffer_size);

    /* Assert */

    assert!(write_result.is_ok_and(|result| { result == buffer_size }));
    assert!(read_result_without_seek.is_ok_and(|result| { result == 0 }));
    assert!(read_result_after_seek.is_ok_and(|result| { result == buffer_size }));
    assert_eq!(random_buffer, reading_buffer);
}

#[test]
fn test_should_succeed_when_writing_and_reading_file_with_seek() {
    /* Arrange */

    let fs = MemFS::new();
    let buffer_size = 64;
    let write_size = 8;
    let loops = 256;

    let fd = fs
        .open("/subject.sj", OpenFlag::O_CREAT | OpenFlag::O_RDWR)
        .unwrap();
    let mut comparison_buffer = generate_random_vector(buffer_size);

    fs.write(fd, &comparison_buffer, buffer_size).unwrap();
    fs.lseek(fd, 0, SeekFlag::SEEK_SET).unwrap();

    /* Action */

    let mut equal_count = 0;

    for _ in 0..loops {
        let random_seek_offset = rand::rng().random_range(0..(buffer_size - write_size));
        let write_random_buffer = generate_random_vector(write_size);

        // Write random data, on random position.
        fs.lseek(fd, random_seek_offset, SeekFlag::SEEK_SET)
            .unwrap();
        fs.write(fd, &write_random_buffer, write_size).unwrap();

        // Modify original buffer too, for comparison
        comparison_buffer[random_seek_offset..(random_seek_offset + write_size)]
            .copy_from_slice(write_random_buffer.as_slice());

        let mut reading_buffer = vec![0; buffer_size];

        // Read whole file, and check content.
        fs.lseek(fd, 0, SeekFlag::SEEK_SET).unwrap();
        fs.read(fd, &mut reading_buffer, buffer_size).unwrap();

        if reading_buffer == comparison_buffer {
            equal_count += 1;
        }

        // Set file offset to zero for next iteration.
        fs.lseek(fd, 0, SeekFlag::SEEK_SET).unwrap();
    }

    fs.close(fd).unwrap();

    /* Assert */

    // All `loops` opeations must success.
    assert_eq!(equal_count, loops);
}

#[test]
fn test_should_succeed_when_writing_on_single_file_through_multiple_file_descriptors() {
    /* Arrange */
    let fs = MemFS::new();
    let file_name = "/power.ade";
    let buffer_size = 64;
    let batch_size = 8;
    let loops = 256;
    let mut comparison_buffer = generate_random_vector(buffer_size);
    // Vector that collects open file descriptors.
    let mut fd_vector = Vec::new();

    // Create file, and write random content on it.
    let init_fd = fs
        .open(file_name, OpenFlag::O_CREAT | OpenFlag::O_RDWR)
        .unwrap();
    fs.write(init_fd, &comparison_buffer, buffer_size).unwrap();
    fs.close(init_fd).unwrap();

    /* Action */

    // Open multiple file descriptors on single file, and write random content on random offset.
    // For comparison, the same write operations are done on comparison_buffer too.
    for _ in 0..loops {
        let fd = fs.open(file_name, OpenFlag::O_WRONLY).unwrap();
        fd_vector.push(fd);

        let random_write_buffer = generate_random_vector(batch_size);
        let random_offset = rand::rng().random_range(0..(buffer_size - batch_size));

        fs.lseek(fd, random_offset, SeekFlag::SEEK_SET).unwrap();
        fs.write(fd, &random_write_buffer, batch_size).unwrap();

        comparison_buffer[random_offset..(random_offset + batch_size)]
            .copy_from_slice(random_write_buffer.as_slice());
    }

    // Close every file descriptor.
    for fd in fd_vector {
        fs.close(fd).unwrap();
    }

    // Now, the read from file again.
    let final_fd = fs.open(file_name, OpenFlag::O_RDONLY).unwrap();
    let mut final_buffer = vec![0; buffer_size];

    fs.read(final_fd, &mut final_buffer, buffer_size).unwrap();
    fs.close(final_fd).unwrap();

    /* Assert */

    // Finally, check if the content on file is identical to comparison_buffer.
    assert_eq!(comparison_buffer, final_buffer);
}

#[test]
fn test_should_fail_when_reading_from_write_only_file() {
    /* Arrange */

    let fs = MemFS::new();
    let buffer_size = 16;
    let random_buffer = generate_random_vector(buffer_size);
    let mut placeholder_buffer = vec![0; buffer_size];

    let write_only_fd = fs
        .open("/write.f2", OpenFlag::O_CREAT | OpenFlag::O_WRONLY)
        .unwrap();
    fs.write(write_only_fd, &random_buffer, buffer_size)
        .unwrap();

    /* Action */

    let read_result = fs.read(write_only_fd, &mut placeholder_buffer, buffer_size);

    /* Assert */

    assert!(read_result.is_err_and(|e| { matches!(e.err_type, MemFSErrType::EBADF) }))
}

#[test]
fn test_should_succeed_when_reading_and_fail_when_writing_on_read_only_file() {
    /* Arrange */

    let fs = MemFS::new();
    let buffer_size = 64;
    let random_buffer = generate_random_vector(buffer_size);
    let mut buffer = vec![0; buffer_size];

    // Prepare file, and write random content on it.
    let initial_fd = fs
        .open("/victim.vic", OpenFlag::O_CREAT | OpenFlag::O_RDWR)
        .unwrap();
    fs.write(initial_fd, &random_buffer, buffer_size).unwrap();
    fs.close(initial_fd).unwrap();

    // Open file again, now on read-only mode.
    let read_only_fd = fs.open("/victim.vic", OpenFlag::O_RDONLY).unwrap();

    /* Action */

    // Try reading, and writing on file.
    let read_result = fs.read(read_only_fd, &mut buffer, buffer_size);
    let write_on_read_only = fs.write(read_only_fd, &random_buffer, buffer_size);

    /* Assert */

    // Reading should succeed.
    assert!(read_result.is_ok());
    assert_eq!(random_buffer, buffer);

    // Writing should fail.
    assert!(write_on_read_only.is_err_and(|e| { matches!(e.err_type, MemFSErrType::EBADF) }));
}

#[test]
fn test_should_fail_when_reading_from_file_by_more_than_buffer_size() {
    /* Arrange */

    let fs = MemFS::new();
    let file_size = 256;
    let buffer_size = 64;
    let file_name = "/dontreadtoomuch";
    let random_buffer = generate_random_vector(file_size);

    // Create a file and write random content on it.
    let initial_fd = fs
        .open(file_name, OpenFlag::O_CREAT | OpenFlag::O_RDWR)
        .unwrap();
    fs.write(initial_fd, &random_buffer, file_size).unwrap();
    fs.close(initial_fd).unwrap();

    let mut read_buffer = vec![0; buffer_size];

    let fd = fs.open(file_name, OpenFlag::O_RDONLY).unwrap();

    /* Action */

    // Try to read more than buffer size; it should fail.
    let read_result = fs.read(fd, &mut read_buffer, file_size);

    /* Assert */

    assert!(read_result.is_err_and(|e| { matches!(e.err_type, MemFSErrType::EFAULT) }));
}

#[test]
fn test_should_succeed_when_reading_from_file_by_more_than_buffer_size_but_actual_content_size_is_not_larger_than_buffer_size()
 {
    /* Arrange */

    let fs = MemFS::new();
    let file_size = 64;
    let pseudo_read_size = 256;
    let buffer_size = file_size;
    let file_name = "/donald.trump";
    let random_buffer = generate_random_vector(file_size);

    // Create a file and write random content on it.
    let initial_fd = fs
        .open(file_name, OpenFlag::O_CREAT | OpenFlag::O_RDWR)
        .unwrap();
    fs.write(initial_fd, &random_buffer, file_size).unwrap();
    fs.close(initial_fd).unwrap();

    let mut read_buffer = vec![0; buffer_size];
    let fd = fs.open(file_name, OpenFlag::O_RDONLY).unwrap();

    /* Action */

    // Try to read more than buffer size, but the actual content size is not larger than buffer size.
    let read_result = fs.read(fd, &mut read_buffer, pseudo_read_size);

    /* Assert */

    assert!(read_result.is_ok_and(|result| { result == file_size }));
}

#[test]
fn test_should_succeed_when_writing_over_the_file_size() {
    /* Arrange */

    let fs = MemFS::new();
    let small_buffer_size = 64;
    let large_buffer_size = 256;
    let file_name = "/enlarge.lag";
    let random_buffer = generate_random_vector(small_buffer_size);
    let large_random_buffer = generate_random_vector(large_buffer_size);

    // Create file and write random content.
    let initial_fd = fs
        .open(file_name, OpenFlag::O_CREAT | OpenFlag::O_RDWR)
        .unwrap();
    fs.write(initial_fd, &random_buffer, small_buffer_size)
        .unwrap();
    fs.close(initial_fd).unwrap();

    let write_fd = fs.open(file_name, OpenFlag::O_WRONLY).unwrap();

    /* Action */

    let large_write_result = fs.write(write_fd, &large_random_buffer, large_buffer_size);

    /* Assert */

    assert!(large_write_result.is_ok_and(|v| { v == large_buffer_size }));

    fs.close(write_fd).unwrap();
}

#[test]
fn test_check_whether_writes_on_descriptor_with_o_append_are_done_regardless_of_offset() {
    todo!()
}