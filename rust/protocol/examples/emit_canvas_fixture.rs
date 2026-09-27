#[path = "../tests/common/canvas_fixture.rs"]
mod fixture;
use binprot::BinProtWrite;
fn main() {
    let mut bytes = Vec::new();
    fixture::scene().binprot_write(&mut bytes).unwrap();
    for byte in bytes {
        print!("{byte:02x}");
    }
    println!();
}
