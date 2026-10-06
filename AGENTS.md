# GPUIO agent guidance

Read `docs/status.md`, `CONTRIBUTING.md` and
`docs/design/engineering-standards.md`, then the relevant live Linear ticket.
Follow the accepted stock OCaml 5.3/Bonsai v0.17/Core/Eio design. Use the repository
toolchain and isolated environment; never mutate unrelated switches or defaults.

Platform priority (owner, 2026-09-28): milestone 07 is a macOS-first v1 release.
Linux builds/unit/private-bus/consumer checks stay required; graphical smoke is
informational. Full Linux desktop qualification is OCH-47 in deferred milestone
07b and does not block milestone 07, milestone 08 or feature development. Do not
claim Linux GUI acceptance from compilation. No local VM or remote machine is
required now. See `docs/platform-release-policy.md` for the current scope split.

Local desktop courtesy (owner, 2026-09-13, revised): prefer background windows
where a test permits them, but local foreground GUI/focus/IME tests are explicitly
authorized for fast iteration. The owner accepts focus interruptions when needed;
do not wait for CI alone to debug native behavior or ask permission for each run.
Avoid unnecessary activation and repeated runs. Background rendering/layout
checks must not be reported as real foreground keyboard/IME validation.

VoiceOver authorization (owner, 2026-10-05, revised): the owner lifted the earlier
hold. VoiceOver testing, configuration and automation may resume as part of macOS
accessibility validation; immediate execution is not required. Restore temporary
test settings afterward and qualify acceptance only from actual test evidence.

OCH-11 delivery workflow (owner, 2026-09-13): complete the remaining ticket scope
locally, validating incrementally with local builds/tests. Do not wait on hosted
CI between component families. Submit the complete remaining change for CI and
resolve hosted failures together before merging. An open PR must not pause useful
local implementation. Required merge gates remain in force.

Draft coherent types/interfaces before implementation. Use typed comparison,
receiver-first APIs, validated invariants, Jane Street formatting/PPX and expect
tests. Keep Rust native ownership and asynchronous event delivery explicit.
Do not promote prototype shortcuts into production contracts.

Historical documents contain research paths and screenshots; these are evidence,
not build dependencies or current functionality. Keep implementation status honest
and record exact commands, revisions and actual platform coverage in Linear.
Complete only tickets whose acceptance criteria have passed.

Every new example component needs an adjacent Markdown walkthrough linked from
its owning README. Keep it current when behavior, structure or commands change.
Follow `examples/coverage-guide.md`, maintain its source inventory and run
`python3 scripts/audit_example_docs.py`. Review explanations against the actual
code; source/file coverage alone does not prove onboarding or platform acceptance.

Use `scratch/` for local experiments, logs, implementation notes and handoffs that
must survive context compaction. Create it when absent; it is ignored by Git and
excluded from Dune discovery.

Every implementing agent must keep its own notepad under
`scratch/agents/<unique-agent-or-session-id>/`. When working on a ticket, use a
separate `<ticket-id>.md` for that ticket. Keep `index.md` in your own directory
short: active tickets, links and immediate next steps. Do not append implementation
history to a shared global notepad or overwrite another agent's notes.

Update the relevant ticket notepad as work proceeds and before compaction/handoff:
record decisions, changed files, exact commands/results, running processes and the
next concrete steps. Summarize stale detail and link separate logs/artifacts so
the notepad stays useful. For work without a ticket, use a named task notepad in
your own directory. Durable accepted designs and completion evidence also belong
in versioned docs and Linear; scratch notes stay local.
