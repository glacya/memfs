use memfs::memfs::MemFS;

fn main() {
    let fs = MemFS::new();

    fs.create("/mit").unwrap();
    fs.open("/mit", None).unwrap();

    fs.create("/mit").unwrap();
}