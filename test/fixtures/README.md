# Codec fixture

`codec-v1.hex` is the independent OCaml/Rust research fixture, ported unchanged.
Both languages construct the expected typed value, decode these bytes with full
consumption, and encode the value back to the same bytes. It covers integer
boundaries, UTF-8, NUL, variants, lists, options, booleans and a float.

This is a compatibility fixture, not the production GPUIO protocol schema.
