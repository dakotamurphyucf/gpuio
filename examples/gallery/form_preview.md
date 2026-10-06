# Form layout walkthrough

Read [form_preview.ml](form_preview.ml) and its [interface](form_preview.mli). [pages.ml](pages.ml) mounts `component window palette graph` under **Text editing → Forms**. [`Form`](../../lib/core/form.mli) is ordinary validated composition, not a validation engine or editor controller.

`B.state_machine0` returns reactive models and action-injection functions for columns (initially 2, cycling 1–3), size (initially Medium) and action count. Injected effects reduce against the latest model when executed; constructing `act ()` does not save anything. Toggles own horizontal labels, sample error, reversal, absent-label indentation, spans, refinements, hiding and two preferences. Indentation/notifications start true, others false. `let%arr` derives the form from current models.

`Editor.create` owns a single native single-line placement seeded “Northstar”. `Form.Field.create` supplies its semantic label/help/required status and optional sample error. `Form.Item.of_field` associates that metadata directly with the supported editor root; rich label/description overrides change display without discarding associations. The error toggle is synthetic: it does not inspect the draft. Mixed spans use `Style.Grid_location.Axis.span` bounded by columns; hiding adds `Display Hidden` to the keyed name item rather than removing it.

Other keyed items use `Form.Item.create`: preferences contains a controlled switch/checkbox with a 96-pixel label-width override, unlabeled contains the mock inspection action, and summary spans all columns with explicit Vertical/Small overrides. `Form.create` uses key `workspace-form`, columns, collection layout/size, 112-pixel labels and a mock save footer. Item overrides inherit unspecified settings. `named` groups rich content for accessibility; arbitrary items need caller-supplied semantics, unlike `of_field` association.

Native activation of **Form columns** executes its injected unit action, advances the latest model, and causes `let%arr` to derive a new valid grid. GPUIO reconciles the existing keyed editor within it. Type a draft, change orientation/columns, reverse entries and add/remove the sample error to exercise continuity. Clicking Form notifications runs its toggle effect, updates that boolean and derives the switch’s checked description. Save/inspect only increment actions; no validation, permission request or persistence occurs.

Form columns accept 1–1024, reject duplicate item keys and out-of-range placement. Structural grid/placement fields are applied after custom styles; use typed layout arguments instead of state styles to alter structure. Size affects spacing/label typography, not the caller-sized editor. Bonsai owns options/preferences; GPUIO owns editing/layout and native leases. No background worker or asset scope exists. Adapt with domain-stable keys, real validation results and explicit save effects; mirror draft text if it must survive page destruction, rather than assuming Form retains it permanently.

## Run and review

From the repository root:

```sh
./scripts/gpuio build examples/gallery/main.exe
./_build/default/examples/gallery/main.exe
```

The wrapper uses the repository toolchain. These commands are instructions, not checks executed for this documentation change. This component has no standalone executable or self-test. Native interaction requires the running gallery; compilation does not establish focus, keyboard, accessibility or platform acceptance. See [gallery instructions](README.md) and the [development environment](../../docs/development.md).
