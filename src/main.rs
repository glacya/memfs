use memfs::{memfs::MemFS, utils::OpenFlag};

fn main() {
    let fs = MemFS::new();

    let result = fs.open("/mys", OpenFlag::O_CREAT | OpenFlag::O_RDONLY);

    println!("{:?}", result);

    result.unwrap();
}