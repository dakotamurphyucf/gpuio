# Passive menu section labels — OCH-41

Local macOS arm64 validation, 2026-10-05, based on
`bd9094f68a3217b9d3b7a43f2226a2ca82d8ffd5` plus the archived source changes.
The [contract](../design/menu-labels.md) adds `Menu.Item.Label` to drawn menus.
The [source review](../catalog/commands-menus-review.md) retains the remaining
icons/custom rows, palette and native popup gaps. OCH-41 and OCH-17 remain open.

## API, wire and native semantics

OCaml expect tests validate labels, item/text budgets, command-ID separation,
platform-bar rejection and an independent byte fixture. Rust checks all drawn
presentations, nested platform rejection, truncation and bounded allocation before
decoding overlong or aggregate oversized text. Existing menu bytes are unchanged.
The complete protocol/native Rust suite passes **1,352 tests**, with two existing
skips. Full OCaml `@runtest`, formatting and gallery build pass. Strict Clippy
passes for default targets and the feature-enabled test code. All four menu
observation/rendering regressions pass after the final validation-order change.
The new production-renderer test on GPUI TestPlatform checks passive
accessibility text, no click/focus action, no disabled-control state, keyboard
navigation/typeahead, invocation and a label-only menu. TestPlatform evidence is
not OS input or screen-reader acceptance.

The original new test incorrectly assumed that opening left selection empty.
The renderer already selects the first enabled command. The corrected assertion
checks initial selection, Down/wrap, Home/End and typeahead without changing
production selection behavior. Both failing and corrected logs are preserved.
Review also moved platform label detection after bounded depth validation, so
invalid raw native values cannot trigger an unbounded preliminary traversal.

## Installed macOS gallery

A fresh installed public consumer passes the complete Feedback walkthrough
(session 80464, exit 0). The new checks find `AXStaticText` without `AXPress` in
the dropdown, nested drawn menu bar and editor context menu. OS keyboard Down/
Enter invokes the expected command; Shift-F10 opens the retained editor context
menu and Escape restores editor focus. Existing command enablement, shortcuts,
palette dispatch, notifications, expiry, page remount and shutdown also pass.
The owned application was confirmed absent afterward.

Binary SHA-256:
`edc32104b3dc3ef02c14870b721e1a2d8960699d9ea6012dea4d95f3b105662e`.
The physical binary predates the validation-order hardening for invalid inputs;
its source snapshot is retained separately. Final native validation covers that
ordering change. No screenshot, VoiceOver, Linux GUI, physical presentation
latency or complete-release acceptance is claimed. Hosted checks remain required.

[Commands, original failures, source snapshots and logs](menu-labels-och41/validation.tar.gz)
are retained with a [checksum manifest](menu-labels-och41/manifest.json).
