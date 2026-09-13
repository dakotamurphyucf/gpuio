#[path = "../tests/common/control_fixture.rs"]
mod control_fixture;
use binprot::BinProtWrite;
fn main() {
    let mut bytes = Vec::new();
    control_fixture::request()
        .binprot_write(&mut bytes)
        .unwrap();
    for byte in bytes {
        print!("{byte:02x}");
    }
    println!();
}
