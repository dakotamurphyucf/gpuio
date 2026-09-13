#[path = "../tests/common/file_dialog_fixture.rs"]
mod fixture;
use binprot::BinProtWrite;
fn print(value: impl BinProtWrite) {
    let mut bytes = vec![];
    value.binprot_write(&mut bytes).unwrap();
    for byte in bytes {
        print!("{byte:02x}");
    }
    println!();
}
fn main() {
    if std::env::args().any(|arg| arg == "--events") {
        print(fixture::events());
    } else {
        for request in fixture::requests() {
            print(request);
        }
    }
}
