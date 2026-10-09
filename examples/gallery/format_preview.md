# Input formatting walkthrough

Read [format_preview.ml](format_preview.ml) and its [interface](format_preview.mli). [pages.ml](pages.ml) mounts it in **Text editing → Options**. `Mode.t` is Reference/Decimal/Free; `Mode.format` constructs a pattern `99-99`, a grouped decimal with comma separator and at most two fraction digits, or no format. `Mode.sample` provides synthetic raw values.

`B.state` owns reactive mode and notice values. The configuration `let%arr` reads mode to derive a single-line native input configuration; `Editor.create` creates its placement with initial text `12-34`. The outer `let%arr` reads current controller snapshots and models to derive controls/readouts. Constructing a setter effect does not execute it. Native activation of Decimal executes `set_mode Decimal`, updates the mode model, derives the new configuration and raw-value readout, and updates the existing native editor. Changing format does not replace its draft; incompatible retained text is reported as such.

[`Input_format`](../../lib/core/input_format.mli) conversions are pure. `format_raw` inserts pattern literals or numeric grouping, rejecting mismatches/leftovers rather than dropping input; `raw_of_formatted` requires exact pattern prefixes or canonical numeric grouping. Numeric formatting keeps precision and trailing zeros without float conversion, permits incomplete decimal drafts and does not support exponent notation. Pattern slots count Unicode scalars, separate from grapheme-aware native editing. Input/output are bounded to 256 KiB.

`load_sample` first formats the selected raw sample and checks for an editor snapshot. If available, `B.Effect.Let_syntax` sequences `Editor.replace_if_unchanged` using the captured snapshot, End selection and Record undo policy. The native command rechecks exact lease/revision and rejects intervening edits or composition; its asynchronous result sets the notice. This effect binding waits for a command result, unlike reactive `let%arr`. Formatting alone neither edits text nor adds undo history.

Switch modes while typing, then explicitly load a sample and Undo. The raw readout uses the latest observed snapshot; during composition it shows “Composing…” rather than extracting partial text. All auxiliary buttons use Button.Focus.Preserve, preserving sibling focus on pointer activation without their own Tab/keyboard/semantic Focus target. Native GPUIO owns editing/undo/IME, Bonsai owns mode/notice, and the Eio adapter owns lease/request lifecycle. No background task or asset registration exists. Adapt with handled conversion errors and guarded replacement; do not treat display formatting as complete business validation or expect a native draft to survive page destruction without application storage.

## Run and review

From the repository root:

```sh
./scripts/gpuio build examples/gallery/main.exe
./_build/default/examples/gallery/main.exe
```

Use the repository wrapper for the isolated toolchain. These commands were not executed for this documentation change. There is no standalone executable or self-test for this component. Compilation alone does not establish native keyboard, animation, focus or platform acceptance. See [gallery instructions](README.md) and [development](../../docs/development.md).
