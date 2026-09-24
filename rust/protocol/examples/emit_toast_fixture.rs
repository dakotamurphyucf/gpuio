#[path = "../tests/common/toast_fixture.rs"]
mod fixture;
use binprot::BinProtWrite;
fn main() {
    let mut bytes = vec![];
    if std::env::args().any(|arg| arg == "--events") {
        fixture::events().binprot_write(&mut bytes).unwrap();
    } else {
        fixture::request().binprot_write(&mut bytes).unwrap();
    }
    for byte in bytes {
        print!("{byte:02x}");
    }
    println!();
}
