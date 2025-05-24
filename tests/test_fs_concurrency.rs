// Benchmarking platform FS.

use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::Path,
    thread,
    time::Instant,
};

use memfs::utils::{FILE_MAX_SIZE, generate_random_vector};

const TOTAL_WORKS: usize = 1usize << 16;

#[allow(unused)]
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

fn throughput_reporter<F>(f: F)
where
    F: Fn(usize) -> u128,
{
    let iter = 0..7;
    // let iter = std::iter::repeat(8).take(20);
    let threads: Vec<usize> = iter.map(|x| 1usize << x).collect();
    // let threads = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20];
    let loop_count = 16;
    let mut time_elapsed = Vec::new();

    let test_path = Path::new("ex");

    for i in threads.iter() {
        let mut avg: u128 = 0;

        for _ in 0..loop_count {
            let measured = f(*i);
            avg += measured;

            fs::remove_dir_all(test_path).unwrap();
            fs::create_dir(test_path).unwrap();
        }

        time_elapsed.push(avg / loop_count);
    }

    println!("\nResult\n|Threads|Time(us)|ops/s|\n|---|-----|-----|");

    for (i, thread_count) in threads.iter().enumerate() {
        let time_float = time_elapsed[i] as f64;
        let ops_per_second = 1000000.0 * (TOTAL_WORKS as f64) / time_float;

        println!(
            "|{}|{}|{:.2}|",
            thread_count, time_elapsed[i], ops_per_second
        );
    }
}

test_throughput_ig!(
    test_throughput_measure_on_creates_on_different_directory,
    helper_fs_creates_on_different_directory
);
test_throughput_ig!(
    test_throughput_measure_on_reads_on_single_file_with_multiple_descriptors,
    helper_fs_reads_from_single_file_through_multiple_file_descriptors
);
test_throughput_ig!(
    test_throughput_measure_on_writes_on_multiple_files,
    helper_fs_writes_on_multiple_files_without_o_append
);

fn helper_fs_creates_on_different_directory(thread_count: usize) -> u128 {
    /* Arrange */

    let file_name = "eternal.return";
    let work_per_thread = TOTAL_WORKS / thread_count;
    let mut handles = Vec::new();

    for i in 0..thread_count {
        let dir_name = std::fmt::format(format_args!("ex/dir{}", i));

        fs::create_dir(Path::new(dir_name.as_str())).unwrap();
    }

    let timer = Instant::now();

    /* Action */

    for i in 0..thread_count {
        handles.push(thread::spawn(move || {
            let mut open_success = 0;

            for j in 0..work_per_thread {
                let file_name = std::fmt::format(format_args!("ex/dir{}/{}{}", i, j, file_name));
                let path = Path::new(file_name.as_str());

                let result = OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .open(path);

                if result.is_ok() {
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

fn helper_fs_reads_from_single_file_through_multiple_file_descriptors(thread_count: usize) -> u128 {
    /* Arrange */

    let work_per_thread = TOTAL_WORKS / thread_count;
    let true_total_work = work_per_thread * thread_count;
    let buffer_size = FILE_MAX_SIZE;
    let file_name = "ex/readers.txt";
    let mut handles = Vec::new();
    let mut fds = Vec::new();

    let random_vector = generate_random_vector(buffer_size);
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(file_name)
        .unwrap();
    file.write(random_vector.as_slice()).unwrap();

    drop(file);

    for _ in 0..thread_count {
        let fd = OpenOptions::new().read(true).open(file_name).unwrap();
        fds.push(fd);
    }

    let timer = Instant::now();

    /* Action */

    // Just performance check of read.
    for mut fd in fds.into_iter() {
        handles.push(thread::spawn(move || {
            let mut read_success = 0;
            let mut read_buffer = vec![0; FILE_MAX_SIZE];

            for _ in 0..work_per_thread {
                if fd.read(read_buffer.as_mut_slice()).is_ok() {
                    read_success += 1;
                }
            }

            read_success
        }));
    }

    let mut count = 0;

    for handle in handles {
        count += handle.join().unwrap();
    }

    let measured = timer.elapsed().as_micros();

    /* Assert */

    assert_eq!(count, true_total_work);

    measured
}

fn helper_fs_writes_on_multiple_files_without_o_append(thread_count: usize) -> u128 {
    /* Arrange */

    let buffer_size = 4096;
    let file_prefix = "go_home";
    let mut handles = Vec::new();
    let mut fds = Vec::new();

    for i in 0..thread_count {
        let file_name = format!("ex/{}{}.txt", file_prefix, i);
        let fd = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(file_name)
            .unwrap();

        fds.push(fd);
    }

    let timer = Instant::now();

    /* Action */

    // Each thread opens file descriptor on the same file, and do writes multiple times.
    for (i, mut fd) in fds.into_iter().enumerate() {
        handles.push(thread::spawn(move || {
            let mut written = 0;
            let write_buffer = vec![40u8; buffer_size];

            let thread_works = if (TOTAL_WORKS % thread_count) > i {
                TOTAL_WORKS / thread_count + 1
            } else {
                TOTAL_WORKS / thread_count
            };

            for _ in 0..thread_works {
                if fd.write(write_buffer.as_slice()).is_ok() {
                    written += buffer_size;
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
