#[path = "../tests/common/palette_fixture.rs"]
mod palette_fixture;
use binprot::BinProtWrite;
fn main() {
    let mut bytes = Vec::new();
    if std::env::args().any(|arg| arg == "--events") {
        palette_fixture::events().binprot_write(&mut bytes).unwrap();
    } else {
        palette_fixture::request()
            .binprot_write(&mut bytes)
            .unwrap();
    }
    for byte in bytes {
        print!("{byte:02x}");
    }
    println!();
}
