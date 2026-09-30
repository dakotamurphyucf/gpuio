#[path = "../tests/common/input_fixture.rs"]
mod fixture;
use binprot::BinProtWrite;
fn print(value: &impl BinProtWrite) {
    let mut bytes = Vec::new();
    value.binprot_write(&mut bytes).unwrap();
    println!(
        "{}",
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
    );
}
fn main() {
    if std::env::args().nth(1).as_deref() == Some("--request") {
        print(&fixture::request());
        return;
    }
    if std::env::args().nth(1).as_deref() == Some("--events") {
        print(&fixture::envelopes());
        return;
    }
    if std::env::args().nth(1).as_deref() == Some("--policies") {
        for config in fixture::policy_configs() {
            print(&config);
        }
        return;
    }
    print(&fixture::config());
    for event in fixture::events() {
        print(&event);
    }
}
