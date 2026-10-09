# Native content hint walkthrough

Read [content_hint_preview.ml](content_hint_preview.ml) and its [interface](content_hint_preview.mli). [pages.ml](pages.ml) mounts `component window palette graph` under **Text editing → Options**.

`Hint` aliases `Text_input.Content_hint`. `B.state` starts with `Some Email_address`; `B.toggle` starts read-only false; another state stores “Not checked yet”. The first `let%arr` derives a validated single-line `Text_input.Config` with accessible label “Contact detail”, placeholder and optional hint. `Editor.create` creates a native editor with synthetic initial text `hello@example.test`. Configuration updates retain that editor and its draft rather than applying the initial text again.

Bonsai state is reactive, so configuration is derived again when a hint or read-only value changes. Native activation of Website executes `set_hint (Some Url)`, updates the hint model, and makes the first `let%arr` derive a new configuration; the outer `let%arr` derives the Configured readout, and GPUIO reconciles both with the existing editor. Creating a setter or query effect during view construction does not execute it. Clicking Check native hint executes the query effect; its eventual reply runs `set_last_check`, updates that model, and derives a new native Last check readout.

The outer `let%arr` constructs controls and two asynchronous effects. Inside `B.Effect.Let_syntax`, `let%bind` waits for `Editor.content_hint_status editor`, then stores `describe result`. This is effect sequencing, distinct from the reactive `let%arr` that builds views. `Editor.focus` similarly waits for native focus completion and stores only errors. Effects target the exact editor lease; stale/closed targets use ordinary command errors, not a replacement editor.

[`Content_hint.Status`](../../lib/core/input_content_hint.mli) distinguishes Inactive None, Inactive with a configured hint, Exposed and Unavailable with Backend/Mapping/Native_view reasons. `hint_name` formats the typed S-expression name; `describe` includes the returned hint so a delayed reply cannot be mistaken for a newer configuration. [`content_hint_status`](../../lib/eio/text_input.mli) reads metadata at native query execution: it does not read or edit text, focus the field, update its stored snapshot or guarantee autofill.

Email, Website, IMEI and No hint assign option values. Their buttons and Check native hint use `Button.Focus.Preserve`, preserving editor focus on pointer activation and supplying no separate Tab/keyboard/semantic Focus target. The regular Focus contact field button explicitly requests focus. Read-only is an independent native configuration value; hints are metadata, not text validation or privacy policy.

Focus the contact field, then check to sample Email exposure. Choose Website and check again: the previous Last check remains until a new reply arrives, while Configured immediately shows the new choice. Choose IMEI to exercise an unavailable mapping on macOS, or No hint for no configured metadata. Make the field read-only and query again; eligibility can change without changing its text. Backend exposure is not a promise that the OS will offer autofill.

Bonsai owns hint/read-only choices and the latest formatted reply. GPUIO owns editing, focus and metadata exposure; the Eio adapter manages the editor lease and outstanding request lifecycle on departure. There is no file/network worker or asset scope. Adapt by handling status and command errors explicitly, validating text separately, and adding password privacy when choosing Password/New_password hints as required by the public contract. Use synthetic values for demonstrations, as this example does.

## Run and review

From the repository root, use the isolated repository wrapper:

```sh
./scripts/gpuio build examples/gallery/main.exe
./_build/default/examples/gallery/main.exe
```

These are instructions, not validation performed for this documentation change. This component has no standalone executable or self-test. Interactive behavior requires the native gallery; compilation alone does not establish keyboard, focus, accessibility or platform acceptance. See the [gallery README](README.md) and [development environment](../../docs/development.md).
