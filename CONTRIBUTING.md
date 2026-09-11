# Contributing

Read [engineering standards](docs/design/engineering-standards.md) before writing
OCaml. Core is the standard library foundation; first-party OCaml I/O uses Eio.
Use Jane Street PPX and expect tests. The formatter profile is `janestreet`,
pinned to ocamlformat 0.28.1. Rust uses the checked-in toolchain.

Keep switches and build artifacts local to this checkout. Do not change global
opam/rustup selections or another project's dependencies. Follow the
[development guide](docs/development.md) and [validation status](docs/status.md).

Create `scratch/` for untracked local development files, experiments and notes.
Git ignores it and Dune excludes it from recursive build discovery. Each agent
must keep its own `scratch/agents/<unique-agent-or-session-id>/` notepads, one
`<ticket-id>.md` per ticket and a short personal `index.md`. Keep notes current
for compaction recovery without growing a shared global history. Do not rely on
scratch files as committed build inputs.

## Review checklist

- Coherent domain modules, principal `t`, receiver-first operations and `.mli`
  contracts; deliberate abstraction, units, IDs and validated decoding.
- Typed comparison; Core's integer `=` is valid. No polymorphic comparison.
- Boolean predicates use `if`; structural cases use exhaustive matching.
  Intentionally ignored fields/payloads are explicit; preserve warning 9.
- Descriptive public names, semantic labels and focused functions. Review meaning
  and readability, not arbitrary line/parameter counts.
- Eio capabilities, scopes, cancellation and typed recovery errors; explicit FFI
  ownership and exception containment.
- Deterministic expect tests for behavior; property/fixture/native tests where
  appropriate. Review expectation diffs before `dune promote`; never auto-promote
  routine checks or CI.
- Build/test/format evidence states the actual OS, architecture and display.
  Compilation is distinct from a real window/IME/accessibility check.

Use a branch such as `och-18-repository-scaffold`, with the Linear ticket in the
PR description, validation and remaining limitations. `main` is the default
branch. Keep accepted contracts current in versioned docs and link reviewed
commits back to Linear. One repository and release train covers both languages.
