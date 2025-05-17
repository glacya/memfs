use memfs::{
    memfs::MemFS,
    utils::{generate_random_vector, OpenFlag, SeekFlag, FILE_MAX_SIZE},
};
// use rand::Rng;

#[cfg(feature = "check-loom")]
pub(crate) use loom::{sync::Arc, thread};
use rand::Rng;

use std::{collections::HashMap, time::Instant};
#[cfg(not(feature = "check-loom"))]
pub(crate) use std::{sync::Arc, thread};

const TOTAL_WORKS: usize = 1usize << 16;

macro_rules! test_throughput {
    ($name:ident, $func:expr) => {
        #[test]
        fn $name() {
            throughput_reporter($func);
        }
    };
}

#[allow(unused)]
macro_rules! test_throughput_ig {
    ($name:ident, $func:expr) => {
        #[test]
        #[ignore]
        fn $name() {
            throughput_reporter($func);
        }
    };
}

fn throughput_reporter<F>(f: F) where F: Fn(usize) -> u128 {
    let iter = 0..8;
    // let iter = std::iter::repeat(8).take(20);
    let threads: Vec<usize> = iter.map(|x| 1usize << x).collect();
    let loop_per_count = 16;
    let mut time_elapsed = Vec::new();

    for i in threads.iter() {
        let mut avg = 0;

        for _ in 0..loop_per_count {
            let measured = f(*i);
            avg += measured;
        }
        
        time_elapsed.push(avg / loop_per_count);
    }

    println!("\nResult\n|Threads|Time(us)|ops/s|\n|---|-----|-----|");
    
    for (i, thread_count) in threads.iter().enumerate() {
        let time_float = time_elapsed[i] as f64;
        let ops_per_second = 1000000.0 * (TOTAL_WORKS as f64) / time_float;

        println!("|{}|{}|{:.2}|", thread_count, time_elapsed[i], ops_per_second);
    }
}

test_throughput_ig!(test_throughput_measure_on_creates_on_same_directory, helper_all_should_succeed_when_creating_multiple_file_names_on_same_directory);
test_throughput_ig!(test_throughput_measure_on_creates_on_different_directory, helper_all_should_succeed_when_creating_multiple_files_on_different_directory);
test_throughput_ig!(test_throughput_measure_on_creates_with_o_excl_on_same_directory, helper_only_one_should_succeed_when_opening_file_with_o_creat_and_o_excl_concurrently_on_same_directory);
test_throughput_ig!(test_throughput_measure_on_removes_on_different_directory, helper_all_should_succeed_when_removing_multiple_files_on_different_directory);
test_throughput_ig!(test_throughput_measure_on_writes_on_single_file_descriptor_without_o_append, helper_all_should_succeed_when_writing_on_single_file_descriptor_without_o_append);
test_throughput_ig!(test_throughput_measure_on_writes_on_single_file_descriptor_with_o_append, helper_check_whether_writes_on_file_descriptor_with_o_append_are_atomic);
test_throughput_ig!(test_throughput_measure_on_writes_on_multiple_file_descriptors_on_single_file_without_o_append, helper_all_should_succeed_when_writing_on_multiple_file_descriptors_on_single_file_without_o_append);
test_throughput!(test_throughput_measure_on_writes_on_multiple_files_without_o_append, helper_all_should_succeed_when_writing_on_multiple_files_without_o_append);
test_throughput!(test_throughput_measure_on_reads_on_single_file, helper_all_should_succeed_when_reading_from_single_file_through_multiple_file_descriptors);
test_throughput!(test_throughput_measure_on_reads_and_writes_on_single_file, helper_all_should_succeed_when_read_and_write_from_single_file_through_multiple_file_descriptors);
test_throughput!(test_throughput_measure_on_lseek_on_single_file_descriptor, helper_all_should_succeed_when_lseek_on_single_file_descriptor);
test_throughput_ig!(test_throughput_measure_on_mkdir_on_same_directory, helper_all_should_succeed_when_mkdir_on_same_directory);
test_throughput_ig!(test_throughput_measure_on_mkdir_on_different_directory, helper_all_should_succeed_when_mkdir_on_different_directory);



fn helper_all_should_succeed_when_creating_multiple_file_names_on_same_directory(thread_count: usize) -> u128 {

    /* Arrange */
    
    let arc_fs = Arc::new(MemFS::new());
    let file_prefix = "file";
    let work_per_thread = TOTAL_WORKS / thread_count;
    let mut handles = Vec::new();
    let timer = Instant::now();

    /* Action */

    for i in 0..thread_count {
        let fs = arc_fs.clone();

        handles.push(thread::spawn(move || {
            let mut count = 0;

            for j in 0..work_per_thread {
                let file_name = std::fmt::format(format_args!("{}{}.txt", file_prefix, j + i * work_per_thread));

                let fd = fs.open(file_name.as_str(), OpenFlag::O_CREAT | OpenFlag::O_RDONLY);

                if fd.is_ok() {
                    count += 1;

                    // TODO: Close?
                }
            } 

            count
        }));
    }
    
    let mut success_count = 0;
    
    for handle in handles {
        success_count += handle.join().unwrap_or_else(|_| 0);
    }
    
    let measured = timer.elapsed().as_micros();

    /* Assert */

    assert_eq!(success_count, TOTAL_WORKS);

    measured
}

fn helper_all_should_succeed_when_creating_multiple_files_on_different_directory(thread_count: usize) -> u128 {

    /* Arrange */
    
    let arc_fs = Arc::new(MemFS::new());
    let file_name = "eternal.return";
    let work_per_thread = TOTAL_WORKS / thread_count;
    let mut handles = Vec::new();

    for i in 0..thread_count {
        let dir_name = std::fmt::format(format_args!("dir{}", i));
        
        arc_fs.mkdir(dir_name.as_str()).unwrap();
    }
    
    let timer = Instant::now();

    /* Action */

    for i in 0..thread_count {
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

    let measured = timer.elapsed().as_micros();

    /* Assert */

    assert_eq!(success_count, TOTAL_WORKS);

    measured
}

fn helper_only_one_should_succeed_when_opening_file_with_o_creat_and_o_excl_concurrently_on_same_directory(thread_count: usize) -> u128
 {
    /* Arrange */

    let arc_fs = Arc::new(MemFS::new());
    let file_suffix = "ran.dom";
    let work_per_thread = TOTAL_WORKS / thread_count;
    let mut handles = Vec::new();

    for i in 0..thread_count {
        let dir_name = std::fmt::format(format_args!("dir{}", i));
        arc_fs.mkdir(dir_name.as_str()).unwrap();
    }

    let timer = Instant::now();

    /* Action */

    for i in 0..thread_count {
        let fs = arc_fs.clone();

        handles.push(thread::spawn(move || {
            let file_name = std::fmt::format(format_args!("dir{}/{}", i, file_suffix));
            let mut count = 0;

            for _ in 0..work_per_thread {
                
                if fs.open(file_name.as_str(), OpenFlag::O_CREAT | OpenFlag::O_EXCL | OpenFlag::O_RDWR).is_ok() {
                    count += 1;
                }
            }

            count
        }));
    }

    let mut success_count = 0;

    for handle in handles {
        success_count += handle.join().unwrap_or_else(|_| 0);
    }

    let measured = timer.elapsed().as_micros();

    /* Assert */

    assert_eq!(success_count, thread_count);

    measured
}

fn helper_all_should_succeed_when_removing_multiple_files_on_different_directory(thread_count: usize) -> u128 {

    /* Arrange */

    let arc_fs = Arc::new(MemFS::new());
    let work_per_thread = TOTAL_WORKS / thread_count;
    let file_suffix = ".rs";

    let mut open_workers = vec![];

    // Create files in directories.

    for i in 0..thread_count {
        let fs = arc_fs.clone();
        let dir_name = format!("dir{}", i);

        fs.mkdir(dir_name.as_str()).unwrap();

        open_workers.push(thread::spawn(move || {
            for j in 0..work_per_thread {
                let file_name = format!("{}/{}{}", dir_name, j, file_suffix);

                fs.open(file_name.as_str(), OpenFlag::O_CREAT | OpenFlag::O_RDWR).unwrap();
            }
        }));
    }

    for handle in open_workers {
        handle.join().unwrap();
    }

    let timer = Instant::now();

    /* Action */

    let mut handles = vec![];

    for i in 0..thread_count {
        let fs = arc_fs.clone();
        let dir_name = format!("dir{}", i);

        handles.push(thread::spawn(move || {
            let mut count = 0;

            for j in 0..work_per_thread {
                let file_name = format!("{}/{}{}", dir_name, j, file_suffix);

                count += if fs.unlink(file_name.as_str()).is_ok() { 1 } else { 0 };
            }

            count
        }));
    }

    let mut success_count = 0;

    for handle in handles {
        success_count += handle.join().unwrap();
    }

    let measured = timer.elapsed().as_micros();

    /* Assert */

    assert_eq!(success_count, TOTAL_WORKS);


    measured
}

// Correctness test
#[test]
fn test_correctness_only_one_should_succeed_when_removing_multiple_files_on_different_directory() {
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


fn helper_check_whether_writes_on_file_descriptor_with_o_append_are_atomic(thread_count: usize) -> u128 {

    /* Arrange */

    let arc_fs = Arc::new(MemFS::new());
    let total_work_this = TOTAL_WORKS;
    let buffer_size = 1;
    let work_per_thread = total_work_this / thread_count;
    let file_name = "conc.write";

    let fd = arc_fs
    .open(
        file_name,
        OpenFlag::O_CREAT | OpenFlag::O_RDWR | OpenFlag::O_APPEND,
    )
    .unwrap();

    let mut handles = Vec::new();
    let timer = Instant::now();

    /* Action */

    // Write numbered buffer to file.
    // The number is determined by loop index.
    for i in 0..thread_count {
        let value = (i % 256) as u8;
        let fs = arc_fs.clone();

        handles.push(thread::spawn(move || {
            for _ in 0..work_per_thread {
                let numbered_buffer = vec![value; buffer_size];
                // let mut rng = rand::rng();
    
                // This lseek should show no effect, since the file is opened with O_APPEND.
                // fs.lseek(
                //     fd,
                //     rng.random_range(0..((i + 1) * buffer_size)),
                //     SeekFlag::SEEK_SET,
                // )
                // .unwrap();
                fs.write(fd, &numbered_buffer, buffer_size).unwrap();
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }
    
    let measured = timer.elapsed().as_micros();

    /* Assert */

    // Now, read the contents.
    arc_fs.lseek(fd, 0, SeekFlag::SEEK_SET).unwrap();

    let mut read_buffer = vec![0; total_work_this * buffer_size];
    let written_bytes = arc_fs
        .read(fd, &mut read_buffer, total_work_this * buffer_size)
        .unwrap();
    arc_fs.close(fd).unwrap();

    // The content should be consecutive blocks having same data by buffer_size.
    // For example, 1 1 2 2 3 3 5 5 4 4 10 10 ... if buffer_size = 2.
    let mut frequency_map = HashMap::new();

    assert_eq!(written_bytes, total_work_this * buffer_size);

    // Check if the content has the correct structure.
    for i in 0..total_work_this {
        // Get the first byte, and see if the bytes in the block also have the same value.
        let read_slice = &read_buffer[(i * buffer_size)..((i + 1) * buffer_size)];
        let first_letter = read_buffer[i * buffer_size];

        assert_eq!(read_slice.to_vec(), vec![first_letter; buffer_size]);

        // Save the frequency of the value of the first byte.
        frequency_map
            .entry(first_letter)
            .and_modify(|v| *v += 1)
            .or_insert(1usize);
    }

    // Check the frequency.
    let expected_freq = total_work_this / thread_count.min(256);

    for i in 0..=(256.min(thread_count) - 1) {
        let ui = (i % 256) as u8;

        let freq = frequency_map.get(&ui).unwrap();
        assert_eq!(*freq, expected_freq);
    }

    measured
}

fn helper_all_should_succeed_when_writing_on_single_file_descriptor_without_o_append(thread_count: usize) -> u128 {

    /* Arrange */

    let total_work_this = TOTAL_WORKS;
    let arc_fs = Arc::new(MemFS::new());
    let work_per_thread = total_work_this / thread_count;
    let buffer_size = 1;
    let file_name = "/food.food";
    let mut handles = Vec::new();

    let fd = arc_fs.open(file_name, OpenFlag::O_CREAT | OpenFlag::O_RDWR).unwrap();
    let timer = Instant::now();

    /* Action */

    for _ in 0..thread_count {
        let fs = arc_fs.clone();

        handles.push(thread::spawn(move || {
            for _ in 0..work_per_thread {
                let random_buffer = generate_random_vector(buffer_size);

                fs.write(fd, &random_buffer, buffer_size).unwrap();
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let measured = timer.elapsed().as_micros();

    let offset = arc_fs.lseek(fd, 0, SeekFlag::SEEK_CUR).unwrap();

    /* Assert */

    assert!(offset <= buffer_size * total_work_this);

    measured
}

fn helper_all_should_succeed_when_writing_on_multiple_file_descriptors_on_single_file_without_o_append(thread_count: usize) -> u128 {

    /* Arrange */

    let arc_fs = Arc::new(MemFS::new());
    let work_per_thread = TOTAL_WORKS / thread_count;
    let buffer_size = 1;
    let file_name = "mul.file";
    let mut handles = Vec::new();

    let init_fd = arc_fs.open(file_name, OpenFlag::O_CREAT | OpenFlag::O_RDWR).unwrap();
    arc_fs.close(init_fd).unwrap();

    let mut fds = Vec::new();
    
    for _ in 0..thread_count {
        fds.push(arc_fs.open(file_name, OpenFlag::O_RDWR).unwrap());
    }
    
    /* Action */
    let timer = Instant::now();

    // Each thread opens file descriptor on the same file, and do writes multiple times.
    for i in 0..thread_count {
        let fs = arc_fs.clone();
        let fd = fds[i];

        handles.push(thread::spawn(move || {
            let mut written = 0;
            let write_buffer = vec![10u8; buffer_size];

            for _ in 0..work_per_thread {
                fs.lseek(fd, 0, SeekFlag::SEEK_SET).unwrap();
                if fs.write(fd, &write_buffer, buffer_size).is_ok() {
                    written += buffer_size
                }
            }

            written
        }));
    }

    let mut count = 0;

    for handle in handles {
        count += handle.join().unwrap();
    }

    let measured = timer.elapsed().as_micros();

    /* Assert */

    assert_eq!(count, TOTAL_WORKS * buffer_size);

    measured
}

fn helper_all_should_succeed_when_writing_on_multiple_files_without_o_append(thread_count: usize) -> u128 {
    
    /* Arrange */

    let arc_fs = Arc::new(MemFS::new());
    let work_per_thread = TOTAL_WORKS / thread_count;
    let buffer_size = FILE_MAX_SIZE;
    let file_prefix = "go_home";
    let mut handles = Vec::new();
    let mut fds = Vec::new();

    for i in 0..thread_count {
        let file_name = format!("{}{}.txt", file_prefix, i);

        let fd = arc_fs.open(file_name.as_str(), OpenFlag::O_CREAT | OpenFlag::O_RDWR).unwrap();
        fds.push(fd);
    }

    let timer = Instant::now();

    /* Action */

    // Each thread has its own file and descriptor, and writes multiple times.
    for i in 0..thread_count {
        let fs = arc_fs.clone();
        let fd = fds[i];
        
        handles.push(thread::spawn(move || {
            let mut written = 0;
            let write_buffer = vec![10u8; buffer_size];

            for _ in 0..work_per_thread {
                fs.lseek(fd, 0, SeekFlag::SEEK_SET).unwrap();
                if fs.write(fd, &write_buffer, buffer_size).is_ok() {
                    written += buffer_size
                }
            }

            fs.close(fd).unwrap();

            written
        }));
    }

    let mut count = 0;

    for handle in handles {
        count += handle.join().unwrap();
    }

    let measured = timer.elapsed().as_micros();

    /* Assert */

    assert_eq!(count, TOTAL_WORKS * buffer_size);

    measured
}

fn helper_all_should_succeed_when_reading_from_single_file_through_multiple_file_descriptors(thread_count: usize) -> u128 {
     
    /* Arrange */

    let arc_fs = Arc::new(MemFS::new());
    let work_per_thread = TOTAL_WORKS / thread_count;
    let buffer_size = FILE_MAX_SIZE;
    let file_name = "readers.txt";
    let mut handles = Vec::new();
    let mut fds = Vec::new();

    let random_vector = generate_random_vector(buffer_size);
    let init_fd = arc_fs.open(file_name, OpenFlag::O_CREAT | OpenFlag::O_WRONLY).unwrap();
    arc_fs.write(init_fd, &random_vector, FILE_MAX_SIZE).unwrap();
    arc_fs.close(init_fd).unwrap();

    for _ in 0..thread_count {
        let fd = arc_fs.open(file_name, OpenFlag::O_RDONLY).unwrap();
        fds.push(fd);
    }

    let timer = Instant::now();

    /* Action */

    // Just performance check of read.
    for i in 0..thread_count {
        let fs = arc_fs.clone();
        let fd = fds[i];
        
        handles.push(thread::spawn(move || {
            let mut read_success = 0;
            let mut read_buffer = vec![0; FILE_MAX_SIZE];

            for _ in 0..work_per_thread {
                fs.lseek(fd, 0, SeekFlag::SEEK_SET).unwrap();
                if fs.read(fd, &mut read_buffer, FILE_MAX_SIZE).is_ok() {
                    read_success += 1;
                }
            }

            fs.close(fd).unwrap();

            read_success
        }));
    }

    let mut count = 0;

    for handle in handles {
        count += handle.join().unwrap();
    }

    let measured = timer.elapsed().as_micros();

    /* Assert */

    assert_eq!(count, TOTAL_WORKS);

    measured
}

fn helper_all_should_succeed_when_read_and_write_from_single_file_through_multiple_file_descriptors(thread_count: usize) -> u128 {
         
    /* Arrange */

    let arc_fs = Arc::new(MemFS::new());
    let work_per_thread = TOTAL_WORKS / thread_count;
    let buffer_size = FILE_MAX_SIZE;
    let file_name = "readers.txt";
    let mut handles = Vec::new();
    let mut fds = Vec::new();

    let random_vector = generate_random_vector(buffer_size);
    let init_fd = arc_fs.open(file_name, OpenFlag::O_CREAT | OpenFlag::O_WRONLY).unwrap();
    arc_fs.write(init_fd, &random_vector, FILE_MAX_SIZE).unwrap();
    arc_fs.close(init_fd).unwrap();

    for _ in 0..thread_count {
        let fd = arc_fs.open(file_name, OpenFlag::O_RDWR).unwrap();
        fds.push(fd);
    }

    let timer = Instant::now();

    /* Action */

    // Just performance check of read.
    for i in 0..thread_count {
        let fs = arc_fs.clone();
        let fd = fds[i];
        
        handles.push(thread::spawn(move || {
            let mut read_success = 0;
            let mut read_buffer = vec![0; FILE_MAX_SIZE];

            for _ in 0..work_per_thread {
                fs.lseek(fd, 0, SeekFlag::SEEK_SET).unwrap();

                if fs.read(fd, &mut read_buffer, FILE_MAX_SIZE).is_ok() {
                    read_success += 1;
                }

                fs.lseek(fd, 0, SeekFlag::SEEK_SET).unwrap();

                if fs.write(fd, &read_buffer, FILE_MAX_SIZE).is_ok() {
                    read_success += 1;
                }
            }

            fs.close(fd).unwrap();

            read_success
        }));
    }

    let mut count = 0;

    for handle in handles {
        count += handle.join().unwrap();
    }

    let measured = timer.elapsed().as_micros();

    /* Assert */

    assert_eq!(count, TOTAL_WORKS * 2);

    measured
}

fn helper_all_should_succeed_when_lseek_on_single_file_descriptor(thread_count: usize) -> u128 {
             
    /* Arrange */

    let arc_fs = Arc::new(MemFS::new());
    let work_per_thread = TOTAL_WORKS / thread_count;
    let buffer_size = FILE_MAX_SIZE;
    let file_name = "mouse.heejun";
    let mut handles = Vec::new();

    let random_vector = generate_random_vector(buffer_size);
    let fd = arc_fs.open(file_name, OpenFlag::O_CREAT | OpenFlag::O_WRONLY).unwrap();
    arc_fs.write(fd, &random_vector, FILE_MAX_SIZE).unwrap();

    let timer = Instant::now();

    /* Action */

    for _ in 0..thread_count {
        let fs = arc_fs.clone();
        
        handles.push(thread::spawn(move || {
            let mut lseek_success = 0;

            for _ in 0..work_per_thread {
                let r = rand::rng().random_range(0..FILE_MAX_SIZE);

                if fs.lseek(fd, r, SeekFlag::SEEK_SET).is_ok() {
                    lseek_success += 1;
                }
            }

            lseek_success
        }));
    }

    let mut count = 0;

    for handle in handles {
        count += handle.join().unwrap();
    }

    let measured = timer.elapsed().as_micros();

    /* Assert */

    assert_eq!(count, TOTAL_WORKS);

    measured
}

fn helper_all_should_succeed_when_mkdir_on_same_directory(thread_count: usize) -> u128 {
    
    /* Arrange */
    
    let arc_fs = Arc::new(MemFS::new());
    let dir_prefix = "dir";
    let work_per_thread = TOTAL_WORKS / thread_count;
    let mut handles = Vec::new();
    let timer = Instant::now();

    /* Action */


    for i in 0..thread_count {
        let fs = arc_fs.clone();

        handles.push(thread::spawn(move || {
            let mut count = 0;

            for j in 0..work_per_thread {
                let dir_name = format!("{}{}", dir_prefix, j + i * work_per_thread);

                let mkdir_result = fs.mkdir(dir_name.as_str());

                if mkdir_result.is_ok() {
                    count += 1;
                }
            } 

            count
        }));
    }
    
    let mut success_count = 0;
    
    for handle in handles {
        success_count += handle.join().unwrap_or_else(|_| 0);
    }
    
    let measured = timer.elapsed().as_micros();

    /* Assert */

    assert_eq!(success_count, TOTAL_WORKS);

    measured
}

fn helper_all_should_succeed_when_mkdir_on_different_directory(thread_count: usize) -> u128 {

    /* Arrange */
    
    let arc_fs = Arc::new(MemFS::new());
    let dir_suffix = "cs999";
    let work_per_thread = TOTAL_WORKS / thread_count;
    let mut handles = Vec::new();

    for i in 0..thread_count {
        let dir_name = std::fmt::format(format_args!("dir{}", i));
        
        arc_fs.mkdir(dir_name.as_str()).unwrap();
    }
    
    let timer = Instant::now();

    /* Action */

    for i in 0..thread_count {
        let fs = arc_fs.clone();

        handles.push(thread::spawn(move || {
            let mut mkdir_success = 0;

            for j in 0..work_per_thread {
                let dir_name = format!("dir{}/{}{}", i, j, dir_suffix);
                
                if fs.mkdir(dir_name.as_str()).is_ok() {
                    mkdir_success += 1;
                }
            }

            mkdir_success
        }));
    }

    let mut success_count = 0;

    for handle in handles {
        success_count += handle.join().unwrap_or_else(|_| 0);
    }

    let measured = timer.elapsed().as_micros();

    /* Assert */

    assert_eq!(success_count, TOTAL_WORKS);

    measured
}