use memfs::{
    memfs::MemFS,
    utils::{OpenFlag, SeekFlag, generate_random_vector},
};
use rand::Rng;

#[cfg(feature = "check-loom")]
pub(crate) use loom::{sync::Arc, thread};

use std::{collections::HashMap, time::Instant};
#[cfg(not(feature = "check-loom"))]
pub(crate) use std::{sync::Arc, thread};

const TOTAL_WORKS: usize = 1usize << 16;

#[test]
#[ignore = "rewriting"]
fn test_throughput_measure_on_creates_on_same_directory() {
    let loops = 64;
    let steps = 64;
    let mut time_elapsed = Vec::new();

    for i in 0..loops {
        let timer = Instant::now();

        helper_all_should_succeed_when_creating_multiple_file_names_on_same_directory((i + 1) * steps);

        let measured = timer.elapsed().as_millis();
        time_elapsed.push(measured);
    }

    println!("\nResult table (create without O_EXCL)\nThread Count\tMeasured Time (ms)");
    
    for i in 0..loops {
        let time_float = time_elapsed[i] as f64;
        let ops_per_second = 1000.0 * (((i + 1) * steps) as f64) / time_float;

        println!("{}\t\t{}\t\t{}", (i + 1) * steps, time_elapsed[i], ops_per_second);
    }
}

#[test]
fn test_throughput_measure_on_creates_on_different_directory() {
    let threads: Vec<usize> = (0..13).map(|x| 1usize << x).collect();
    let mut time_elapsed = Vec::new();

    for i in threads.iter() {
        let timer = Instant::now();

        helper_all_should_succeed_when_creating_multiple_files_on_different_directory(*i);

        let measured = timer.elapsed().as_micros();
        time_elapsed.push(measured);
    }

    println!("\nResult table (create without O_EXCL, different directory)\nThread Count\tMeasured Time (ms)");
    
    for (i, thread_count) in threads.iter().enumerate() {
        let time_float = time_elapsed[i] as f64;
        let ops_per_second = 1000000.0 * (TOTAL_WORKS as f64) / time_float;

        println!("{}\t\t{}\t\t{}", thread_count, time_elapsed[i], ops_per_second);
    }
}

#[test]
#[ignore = "rewriting"]
fn test_throughput_measure_on_creates_with_o_excl_on_same_directory() {
    let iterator = (0..12).map(|x| 1usize << x);
    let threads: Vec<usize> = iterator.collect();
    let mut time_elapsed = Vec::new();

    for i in threads.iter() {
        let timer = Instant::now();

        helper_only_one_should_succeed_when_opening_file_with_o_creat_and_o_excl_concurrently_on_same_directory(*i);

        let measured = timer.elapsed().as_millis();
        time_elapsed.push(measured);
    }

    println!("\nResult table (create with O_EXCL)\nThread Count\tMeasured Time (ms)\tops/s");

    for (i, thread_count) in threads.iter().enumerate() {
        let time_float = time_elapsed[i] as f64;
        let ops_per_second = 1000.0 * (TOTAL_WORKS as f64) / time_float;

        println!("{}\t\t{}\t\t{}", thread_count, time_elapsed[i], ops_per_second);
    }
}

fn helper_all_should_succeed_when_creating_multiple_file_names_on_same_directory(loops: usize) {

    /* Arrange */
    
    let arc_fs = Arc::new(MemFS::new());
    let file_prefix = "file";
    let mut handles = Vec::new();

    /* Action */

    for i in 0..loops {
        let fs = arc_fs.clone();

        handles.push(thread::spawn(move || {
            let file_name = std::fmt::format(format_args!("{}{}.txt", file_prefix, i));
            if fs.open(file_name.as_str(), OpenFlag::O_CREAT | OpenFlag::O_RDONLY).is_ok() {
                1
            }
            else {
                0
            }
        }));
    }

    let mut success_count = 0;

    for handle in handles {
        success_count += handle.join().unwrap_or_else(|_| 0);
    }

    /* Assert */

    assert_eq!(success_count, loops);
}

fn helper_only_one_should_succeed_when_opening_file_with_o_creat_and_o_excl_concurrently_on_same_directory(thread_count: usize)
 {
    /* Arrange */

    let arc_fs = Arc::new(MemFS::new());
    let file_name = "/ran.dom";
    let mut handles = Vec::new();

    /* Action */

    for _ in 0..thread_count {
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

fn helper_all_should_succeed_when_creating_multiple_files_on_different_directory(thread_counts: usize) {

    /* Arrange */
    
    let arc_fs = Arc::new(MemFS::new());
    let file_name = "eternal.return";
    let work_per_thread = TOTAL_WORKS / thread_counts;
    let mut handles = Vec::new();

    for i in 0..thread_counts {
        let dir_name = std::fmt::format(format_args!("dir{}", i));

        arc_fs.mkdir(dir_name.as_str()).unwrap();
    }

    /* Action */

    for i in 0..thread_counts {
        let fs = arc_fs.clone();

        handles.push(thread::spawn(move || {
            let mut open_success = 0;

            for j in 0..work_per_thread {
                let file_name = std::fmt::format(format_args!("dir{}/{}{}", i, j, file_name));

                if fs.open(file_name.as_str(), OpenFlag::O_CREAT | OpenFlag::O_RDONLY).is_ok() {
                    open_success += 1;
                }
            }

            open_success
        }));
    }

    let mut success_count = 0;

    for handle in handles {
        success_count += handle.join().unwrap_or_else(|_| 0);
    }

    /* Assert */

    assert_eq!(success_count, TOTAL_WORKS);
}

#[test]
#[ignore = "rewriting"]
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
#[ignore = "rewriting"]
fn test_check_whether_concurrent_writes_are_atomic_and_sequential_on_file_descriptor_opened_with_o_append()
 {
    /* Arrange */

    let arc_fs = Arc::new(MemFS::new());
    let dup_loops = 1;
    let basic_loops = 256;
    let loops = dup_loops * basic_loops;
    let block_size = 8;
    let file_name = "conc.write";

    let fd = arc_fs
        .open(
            file_name,
            OpenFlag::O_CREAT | OpenFlag::O_RDWR | OpenFlag::O_APPEND,
        )
        .unwrap();

    /* Action */

    let mut handles = vec![];

    for i in 0..loops {
        let value = (i % 256) as u8;
        let fs = arc_fs.clone();

        handles.push(thread::spawn(move || {
            let numbered_buffer = vec![value; block_size];
            let mut rng = rand::rng();

            fs.lseek(
                fd,
                rng.random_range(0..((i + 1) * block_size)),
                SeekFlag::SEEK_SET,
            )
            .unwrap();
            fs.write(fd, &numbered_buffer, block_size).unwrap();
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    arc_fs.lseek(fd, 0, SeekFlag::SEEK_SET).unwrap();

    let mut read_buffer = vec![0; loops * block_size];
    let written_bytes = arc_fs
        .read(fd, &mut read_buffer, loops * block_size)
        .unwrap();
    arc_fs.close(fd).unwrap();

    /* Assert */

    let mut frequency_map = HashMap::new();

    assert_eq!(written_bytes, loops * block_size);

    for i in 0..loops {
        let read_slice = &read_buffer[(i * block_size)..((i + 1) * block_size)];
        let first_letter = read_buffer[i * block_size];

        assert_eq!(read_slice.to_vec(), vec![first_letter; block_size]);

        frequency_map
            .entry(first_letter)
            .and_modify(|v| *v += 1)
            .or_insert(1usize);
    }

    for i in 0..basic_loops {
        let ui = (i % 256) as u8;

        let freq = frequency_map.get(&ui).unwrap();

        assert_eq!(*freq, dup_loops);
    }
}

#[test]
#[ignore = "need bugfix"]
fn test_check_whether_concurrent_writes_on_file_descriptor_opened_without_o_append_are_interleaving()
 {
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
