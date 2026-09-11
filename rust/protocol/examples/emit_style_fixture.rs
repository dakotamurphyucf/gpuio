#[path = "../tests/common/style_fixture.rs"]
mod fixture;
use binprot::BinProtWrite;
fn main() {
    let mut bytes = Vec::new();
    fixture::request().binprot_write(&mut bytes).unwrap();
    for byte in bytes {
        print!("{byte:02x}");
    }
    println!();
}
