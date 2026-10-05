# Palette search policies — OCH-41

Local macOS arm64 evidence, 2026-10-05, based on `3635a04` plus the archived
source changes. [Contract](../design/palette-policies.md). OCH-41 and OCH-17 remain
open; this qualifies the bounded search/visibility/Escape addition only.

The public API adds command keywords, all-term/substring/unfiltered matching,
optional query-field visibility and clear-query-first Escape. Default requests
retain the existing palette encoding; operation 125 carries only changed options.
Policy-only reconciliation retains node identity and supports resetting defaults.
Native command eligibility, generation checks and captured document targeting
remain authoritative at invocation.

## Validation

OCaml expect tests cover invalid/duplicate/unknown keyword references, text/count/
aggregate bounds, defaults, policy-only reconciliation, unchanged identity and
reset. Independent Rust and OCaml assertions pin the same 30-byte request, including
UTF-8. Rust adds truncated payloads, malformed tags, invalid strings/counts, shared
metadata budgets and matching semantics. Non-ASCII whitespace keeps the existing
Core.String.strip command-metadata semantics in both languages.

Four GPUI TestPlatform tests exercise the production host: retained query identity,
keyword results, hidden-field navigation, query/scope focus transfers, current
registry eligibility, reset, IME-composition Escape precedence, clear-first versus
hidden-query dismissal, atomic rejection, memory accounting and configuration
updates after a hidden palette retires its focus scope. That last case stays
closed and retains its query rather than panicking or reopening. Hidden query
accessibility actions are additionally gated by current visibility.

The full native/protocol Rust run passes **1,357 tests**, with two existing skips.
Subsequent lifecycle refinements pass the four focused native tests. Full OCaml
checks and strict default/feature-enabled Clippy commands are retained in the
archive. TestPlatform composition is not a physical IME acceptance claim.

## Installed macOS gallery

Two fresh independent consumer builds and complete Feedback walkthroughs pass.
The final one uses the production focus-scope guard and verifies keyword-only
matching, reversed-term versus whole-substring behavior, unfiltered results,
query-field absence, clear-first Escape and one-step hidden-query dismissal.
Query values are set through macOS accessibility; Escape/selection/shortcut paths
use native key events. Existing command enablement, registry dispatch, rich menu
pixels, nested/context menu focus, notification expiry/dismissal, page remount and
shutdown also pass. The harness closes/reaps its app; no owned gallery process
remains. The final keyword screenshot was visually inspected.

Final installed executable SHA-256:
`f6476346c40b04872c72f7bec798e967c5d1abb76f708512a34de7efda256cbb`.
No VoiceOver, physical palette IME, Linux GUI or release/performance acceptance
is claimed. Broader palette presentation/query controllers and native OS popup
menus remain open catalog requirements.

## Preserved investigation

The first handwritten codec assertion mistakenly used message tag 1; the existing
Apply tag is 3. Both independent assertions were corrected from the wire declaration.
An initial test import incorrectly targeted a private decoder module.

Two attempted tests assumed an absent focus scope during ordinary mounting.
Inspection showed `sync_tooltips` synchronizes focus before layout; those
assumptions were wrong. The meaningful missing-scope case is an already hidden,
retired palette. Its retained configuration update is now directly tested.

A proposed Unicode-whitespace rejection was withdrawn after a test and source
inspection confirmed that native command validation deliberately matches Core's
byte-oriented whitespace policy. Paired positive regressions preserve that
existing contract. No test expectations or acceptance thresholds were weakened.

[Commands, source snapshots, original failures, final logs and images](palette-policies-och41/validation.tar.gz)
are retained with a [checksum manifest](palette-policies-och41/manifest.json).
