# Input observations — OCH-41 evidence

Status: domain/codec foundation only. No mounted native region, protocol capability
or event-delivery acceptance is claimed. [Contract and remaining implementation](../design/input-observations.md).

The public `Input_region` module validates opt-in subscriptions, static phase/
policy choices, bounded labels and explicit focus policy. It exposes typed,
validated observations, preserving logical positions, mouse buttons/counts,
key/character/repeat and wheel units/phases. Derived events cannot claim to prevent
an earlier default action. Canonical subscription order rejects duplicates and
makes reordering the same declarations semantically inert.

Existing `Wire.Pointer` now aliases a factored `Pointer_wire` module, avoiding a
cycle when the general observation codec shares button/modifier wire definitions.
Existing pointer tags/field order are unchanged. Native event enums and bounded
Rust decoding live in `rust/protocol/src/input.rs` and `decode/input.rs`; this does
not change the advertised host capability mask or add a native dispatch path.

Independent Rust/OCaml fixtures cover all thirteen event kinds, five buttons,
optional pressed-button/character data, UTF-8, maximum u32 click count, both wheel
units and all touch phases. A second fixture covers capture/bubble, all four native
policies, disabled and three focus modes. Tests exercise 104 policy combinations,
canonical ordering, invalid focus subscriptions/labels, nonfinite coordinates,
wrong click buttons/counts, malformed text/tags, every truncated valid message,
trailing bytes and oversized inputs. Rust rejects count/length bounds before
allocation. OCaml validates decoded records before exposing public event types.

Validation commands, macOS 14.5 arm64:

```sh
./scripts/gpuio exec cargo test -p gpuio-protocol --locked -j 2
./scripts/gpuio exec cargo test -p gpuio-protocol --test input --locked -j 2
./scripts/gpuio exec cargo clippy -p gpuio-protocol --all-targets --locked -j 2 -- -D warnings
./scripts/gpuio exec dune build @fmt
./scripts/gpuio exec dune runtest
```

The full protocol suite passes, including pre-existing captured-pointer fixture
checks. The final focused Rust input suite passes all four tests; strict protocol
Clippy passes. Full `dune runtest`, Dune formatting, Rust formatting and the
structural catalog audit also pass.
No GUI windows are required by this foundation check. Native dispatch, current
modal/visibility gating, configuration-bound handler retirement, queue coalescing,
focus/IME interaction and the public gallery remain the next implementation gates.
Linux compilation and all broader release gates remain separate.
