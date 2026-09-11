# GPUIO agent guidance

Read `docs/status.md`, `CONTRIBUTING.md` and
`docs/design/engineering-standards.md`, then the relevant live Linear ticket.
Follow the accepted stock OCaml 5.3/Bonsai v0.17/Core/Eio design. Use the repository
toolchain and isolated environment; never mutate unrelated switches or defaults.

Draft coherent types/interfaces before implementation. Use typed comparison,
receiver-first APIs, validated invariants, Jane Street formatting/PPX and expect
tests. Keep Rust native ownership and asynchronous event delivery explicit.
Do not promote prototype shortcuts into production contracts.

Historical documents contain research paths and screenshots; these are evidence,
not build dependencies or current functionality. Keep implementation status honest
and record exact commands, revisions and actual platform coverage in Linear.
Complete only tickets whose acceptance criteria have passed.
