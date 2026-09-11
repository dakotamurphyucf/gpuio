# Implementation status

Updated 2026-09-11. Milestone 01: reproducible foundation, in progress.

- OCH-18: repository scaffold and versioned design import complete locally;
  remote publication and fresh-clone verification are next.
- OCH-19: dependency locks/native Bonsai packaging pending.
- OCH-20: isolated contributor environment pending.
- OCH-21: Dune/Cargo native smoke integration pending.
- OCH-22: macOS/Linux CI and clean-checkout validation pending.

Repository: `dakotamurphyucf/gpuio`, public, Apache-2.0, default branch `main`.
These settings were selected by the owner on 2026-09-11.

No current scaffold test proves native platform support. The historical v0.17
experiment ran on macOS arm64 only; Linux graphical acceptance remains open.
The production protocol/API/runtime are later milestones.

Scaffold validation on macOS arm64: Dune 3.24.2 `build @all @runtest @fmt`,
`opam lint gpuio.opam`, Rust 1.97.1 `cargo test --workspace`, `cargo fmt --all
--check` and `cargo clippy --workspace --all-targets -- -D warnings` passed.
The Rust scaffold contains no behavior tests yet. OCaml checks used the compatible
existing switch read-only; OCH-19 validates a clean isolated dependency closure.
