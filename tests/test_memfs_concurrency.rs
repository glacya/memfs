use memfs::{memfs::MemFS, utils::{generate_random_vector, OpenFlag, SeekFlag}};
use rand::Rng;

#[cfg(feature = "check-loom")]
pub(crate) use loom::{sync::Arc, thread};

use std::collections::HashMap;
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
    
    /* Arrange */

    let arc_fs = Arc::new(MemFS::new());
    let dup_loops = 1;
    let basic_loops = 256;
    let loops = dup_loops * basic_loops;
    let block_size = 8;
    let file_name = "conc.write";

    let fd = arc_fs.open(file_name, OpenFlag::O_CREAT | OpenFlag::O_RDWR | OpenFlag::O_APPEND).unwrap();

    /* Action */
    
    let mut handles = vec![];

    for i in 0..loops {
        let value = (i % 256) as u8;
        let fs = arc_fs.clone();

        handles.push(thread::spawn(move || {
            let numbered_buffer = vec![value; block_size];
            let mut rng = rand::rng();

            fs.lseek(fd, rng.random_range(0..((i + 1) * block_size)), SeekFlag::SEEK_SET).unwrap();
            fs.write(fd, &numbered_buffer, block_size).unwrap();
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    arc_fs.lseek(fd, 0, SeekFlag::SEEK_SET).unwrap();

    let mut read_buffer = vec![0; loops * block_size];
    let written_bytes = arc_fs.read(fd, &mut read_buffer, loops * block_size).unwrap();
    arc_fs.close(fd).unwrap();

    /* Assert */

    let mut frequency_map = HashMap::new();

    assert_eq!(written_bytes, loops * block_size);

    for i in 0..loops {
        let read_slice = &read_buffer[(i * block_size)..((i + 1) * block_size)];
        let first_letter = read_buffer[i * block_size];

        assert_eq!(read_slice.to_vec(), vec![first_letter; block_size]);

        frequency_map.entry(first_letter).and_modify(|v| { *v += 1 }).or_insert(1usize);
    }

    for i in 0..basic_loops {
        let ui = (i % 256) as u8;

        let freq = frequency_map.get(&ui).unwrap();

        assert_eq!(*freq, dup_loops);
    }

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