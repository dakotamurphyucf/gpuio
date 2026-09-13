#[path = "../tests/common/choice_fixture.rs"]
mod choice_fixture;
use binprot::BinProtWrite;
fn main() {
    let mut bytes = Vec::new();
    if std::env::args().any(|arg| arg == "--events") {
        choice_fixture::events().binprot_write(&mut bytes).unwrap();
    } else {
        choice_fixture::request().binprot_write(&mut bytes).unwrap();
    }
    for byte in bytes {
        print!("{byte:02x}");
    }
    println!();
}
