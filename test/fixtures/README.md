# Codec fixture

`codec-v1.hex` is the independent OCaml/Rust research fixture, ported unchanged.
Both languages construct the expected typed value, decode these bytes with full
consumption, and encode the value back to the same bytes. It covers integer
boundaries, UTF-8, NUL, variants, lists, options, booleans and a float.

This is a compatibility fixture, not the production GPUIO protocol schema.

`style-v1.hex` exercises every OCH-8 Field tag, both Fill variants, shadows and
state styles. Independent constructors live in `test/view_api/style_fixture_test.ml`
and `rust/protocol/tests/common/style_fixture.rs`. Rust also tests all truncated
prefixes. Deliberate schema edits may regenerate it with `cargo run -p gpuio-protocol
--example emit_style_fixture`, followed by review of both independent tests.

`controls-v1.hex` covers checkbox/switch kinds, every initial control configuration
and all appended semantic style state tags. Independent constructors live in
`test/view_api/control_fixture_test.ml` and
`rust/protocol/tests/common/control_fixture.rs`. Regenerate deliberately with
`cargo run -p gpuio-protocol --example emit_control_fixture`; both language tests
must still agree. The earlier editor fixture keeps its original capability mask
15 to preserve those bytes as a compatibility check.
