# Query-fenced external palette results — OCH-41

Locally qualified, 2026-10-06 UTC, on macOS 14.5 arm64. Source is
the worktree based on `37303a5`; no hosted or Linux coverage for this change is
claimed. This slice does not complete the component catalog or release gate.

## Behavior

`Search.External` preserves native editing while an application produces result
IDs and optional grouped order. `Palette_controller.publish_results ~expected`
validates the actual native query and the staged registry/config references,
then installs order and clears loading together. Registry definitions and rich
View content are separate accepted transactions. The public gallery stages
dynamic IDs, then publishes from the exact candidate's accepted Bonsai lifecycle.
See the [contract](../design/palette-external-results.md).

Native query revision catches edits even before their asynchronous observation,
including changing away from and back to the same text. Composition, obsolete
subscriptions, missing fences and invalid references cannot publish stale rows.
Query changes, configured-ID changes, policy changes, observer retirement and
closing retire the bounded external payload. Command activation rechecks current
native state. Result order does not replace the declared rich-content indices.

## Completed local checks

- Full OCaml `@runtest`, `@fmt` and gallery build pass. An added runtime test
  stages two successive command registries while native acceptance is pending;
  each lifecycle runs only after its own candidate is accepted.
- Full native library suite with image/canvas TestPlatform support passes:
  **971 passed, two existing ignored, zero failed**. New tests cover atomic
  loading/order, invalid references, required fences, immediate query ABA,
  queued activation, same-count reorder observations, grouped rich indices,
  disabled selection, composition preservation and retirement paths.
- Strict native/protocol all-target Clippy passes. Independent paired byte
  fixtures and malformed metadata tests cover publication and its bounds. The
  complete protocol suite passes **414 tests** after correcting the old
  unknown-search-tag fixture described below.
- The complete local macOS Feedback walkthrough passes real typing during a
  delayed producer, superseding/canceling that producer, empty results, new
  dynamic command IDs, Enter and Down/Enter activation, Escape before completion,
  page revisit and a fresh reopened query. Existing Feedback checks also pass.
  The wrapper reaps the application/driver and restores the saved clipboard.
- Screenshot inspection shows the retained query, two rich rows, first-row
  highlight and ready footer. Screenshots do not prove interaction by themselves.

A fresh independent consumer builds against staged installed public libraries
and passes catalog negotiation and the complete Feedback walkthrough. A final
demo-only change checks request identity again after the asynchronous loading
acknowledgement, so closing or superseding during that wait does not start new
producer work. Refreshing that one consumer module against the same installed
libraries passes rebuild/catalog negotiation and the full walkthrough. Final
installed executable SHA-256:
`4221296fc87003fed8e961feb6a46ef40c6b5dc7a18e9797c616d91865629582`.
The all-example build passes. No OS settings or VoiceOver configuration changed.

## Preserved failures and limits

The first local walkthrough reached all external search/activation checks but
failed its final assertion that switching pages resets the selection label.
Bonsai preserves ordinary branch state; the corrected assertion expects the
prior selection and verifies a fresh query after reopening. The demo explicitly
retires transient palette/controller/candidate state on page deactivation.
The modal is dismissed before navigation, so this walkthrough does not establish
cancellation of a still-open palette's producer by page switching alone.

Earlier compile failures were interface/record qualification mistakes, corrected
before the passing full build. An early native composition assertion expected
text where the API returns a range; the corrected test checks the real contract.
The first full protocol suite failed because an existing malformed-tag fixture
still used search tag 3, now the valid External policy. The fixture uses unknown
tag 4 and separately pins External's independent encoded bytes and decoding.
No expectation output was automatically promoted.

A later installed run stopped before the external-search checks: the existing
`Preview commands` palette failed its initial query-focus assertion and AX
reported the window itself as focused. This resembles the earlier loading-slice
failure; its cause remains unestablished. The all-example build was still linking
during that run, which is recorded as concurrent activity, not a proven cause.
The failure log and screenshot are retained. A later pass must not be interpreted
as an explanation or resolution of this intermittent focus failure.
The same final binary subsequently passes the full Feedback walkthrough after
the all-example build ends; the failure remains part of the release audit.

The 2 MiB external-mode reservation is conservative admission accounting, not
measured RSS or a buffer allocated eagerly. Command definitions remain shared
registry resources. These tests do not qualify physical frame cadence, a real
IME candidate panel, VoiceOver, Linux GUI or the complete palette family.
Persistent embedding, native OS popup menus and other catalog work remain open.

[Source patch/new modules, exact commands, logs and screenshots](palette-external-results-och41/validation.tar.gz)
are retained with a [verified checksum manifest](palette-external-results-och41/manifest.json).
Earlier hosted run 37400903839 targets `2e9cd54`, not this worktree. Current-source
Linux/hosted checks and the broader macOS release gates remain required.
