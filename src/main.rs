use memfs::{memfs::MemFS, utils::OpenFlag};

fn main() {
    let fs = MemFS::new();

    fs.open("/mit", OpenFlag::O_RDONLY).unwrap();
}