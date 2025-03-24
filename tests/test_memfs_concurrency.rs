use memfs::{memfs::MemFS, utils::{generate_random_vector, OpenFlag, SeekFlag}};

#[cfg(feature = "check-loom")]
pub(crate) use loom::{sync::Arc, thread};

#[cfg(not(feature = "check-loom"))]
pub(crate) use std::{sync::Arc, thread};

#[test]
fn test_only_one_should_succeed_when_opening_file_multiple_times_with_o_creat_and_o_excl_concurrently()
 {
    /* Arrange */

    let arc_fs = Arc::new(MemFS::new());
    let loops = 256;
    let file_name = "/ran.dom";
    let mut handles = Vec::new();

    /* Action */

    for _ in 0..loops {
        let fs = arc_fs.clone();

        handles.push(thread::spawn(move || {
            if fs
                .open(
                    file_name,
                    OpenFlag::O_CREAT | OpenFlag::O_EXCL | OpenFlag::O_RDWR,
                )
                .is_ok()
            {
                1
            } else {
                0
            }
        }));
    }

    let mut success_count = 0;

    for handle in handles {
        success_count += handle.join().unwrap_or_else(|_| 0);
    }

    /* Assert */

    assert_eq!(success_count, 1);
}

#[test]
fn test_only_one_should_succeed_when_removing_file_multiple_times_concurrently() {
    /* Arrange */

    let arc_fs = Arc::new(MemFS::new());
    let loops = 256;
    let file_name = "/my.fr";
    let mut handles = Vec::new();

    let fd = arc_fs
        .open(file_name, OpenFlag::O_CREAT | OpenFlag::O_RDWR)
        .unwrap();
    arc_fs.close(fd).unwrap();

    /* Action */

    for _ in 0..loops {
        let fs = arc_fs.clone();

        handles.push(thread::spawn(move || {
            if fs.unlink(file_name).is_ok() { 1 } else { 0 }
        }));
    }

    let mut success_count = 0;

    for handle in handles {
        success_count += handle.join().unwrap_or_else(|_| 0);
    }

    /* Assert */

    assert_eq!(success_count, 1);
}

#[test]
fn test_check_whether_concurrent_writes_are_atomic_and_sequential_on_file_descriptor_opened_with_o_append() {
    todo!()
}

#[test]
fn test_check_whether_concurrent_writes_on_file_descriptor_opened_without_o_append_are_interleaving() {
    let arc_fs = Arc::new(MemFS::new());
    let loops = 256;
    let buffer_size = 64;
    let file_name = "/food.food";
    let mut handles = Vec::new();

    let fd = arc_fs
        .open(file_name, OpenFlag::O_CREAT | OpenFlag::O_RDWR)
        .unwrap();

    /* Action */

    for _ in 0..loops {
        let fs = arc_fs.clone();

        handles.push(thread::spawn(move || {
            let random_buffer = generate_random_vector(buffer_size);

            fs.write(fd, &random_buffer, buffer_size).unwrap();
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }
    
    let offset = arc_fs.lseek(fd, 0, SeekFlag::SEEK_CUR).unwrap();

    /* Assert */

    assert!(offset <= buffer_size * loops);

}