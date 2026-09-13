//! Print reviewed editor codec fixtures for deliberate schema maintenance.
#[path = "../tests/common/editor_fixture.rs"]
mod fixture;
fn main() {
    use binprot::BinProtWrite;
    let mut requests = Vec::new();
    fixture::requests().binprot_write(&mut requests).unwrap();
    let mut events = Vec::new();
    fixture::events().binprot_write(&mut events).unwrap();
    for bytes in [requests, events] {
        for byte in bytes {
            print!("{byte:02x}");
        }
        println!();
    }
}
