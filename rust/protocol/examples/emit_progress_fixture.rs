#[path = "../tests/common/progress_fixture.rs"]
mod fixture;
use binprot::BinProtWrite;
fn main() {
    let mut bytes = vec![];
    fixture::request().binprot_write(&mut bytes).unwrap();
    for byte in bytes {
        print!("{byte:02x}");
    }
    println!();
}
