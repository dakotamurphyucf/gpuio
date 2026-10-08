# Disclosure walkthrough

[disclosure_preview.ml](disclosure_preview.ml) has no separate interface. [navigation_page.ml](navigation_page.ml) mounts `component window palette graph` on **Navigation**, as “Reveal the right amount”.

`definitions` supplies stable identity/behavior/lifetime IDs. `items ~locked` creates a validated `Choice.Collection`, disabling Behavior when requested. The local `action` type separates native `Request of Disclosure.Request.t`, mode changes, global disabled changes and Behavior availability changes. The pure `apply` function delegates to [`Disclosure`](../../lib/core/disclosure.mli) operations; it does not itself mount views or run tasks.

`B.state_machine0` owns the reactive `Disclosure.t`, initially optional single mode with Identity expanded. It returns the current model plus `inject`, an action-to-effect function. Executing that effect reduces against the latest model; constructing `inject (Mode Multiple)` alone changes nothing. `B.toggle` owns Keep drafts and Animate (both initially true). `let%arr` derives rich heading labels, expansion indicators and accordion configuration from current values.

`V.accordion_with_labels` receives the controlled model and `on_request` that injects `Request request`. A native Behavior trigger activation emits a typed toggle request; the effect reaches `apply`, which updates expansion against the latest model; `let%arr` derives the new open panels, and GPUIO reconciles them and animates reveal. Native heading navigation/focus is separate from application expansion. Required single mode cannot collapse its sole expanded member. Missing/disabled requests and requests while globally disabled do nothing. Changing items/mode normalizes expansion in collection order.

`Editor.create` supplies a multiline Identity draft. [`Content_policy.Retain`](../../lib/core/content_policy.mli) keeps closed native children mounted but inert to input/AX/IME; Unmount destroys them, creating new leases when reopened. Keep drafts selects this native policy, not Bonsai computation lifetime or an Eio cancellation scope. The controller remains allocated outside the panels, but it does not preserve a destroyed native buffer. Its initial seed is used on a new mount. Write text, close/reopen Identity with Keep drafts on, then turn it off and repeat to compare lifetimes.

`Disclosure.Motion.standard` uses native measured-height spring reveal with no per-frame OCaml callbacks. Initial mounting is settled; reduced motion/inactive windows settle immediately. Unmount removes closed descendants immediately and skips closing motion. Immediate motion is selected when Animate is off. Motion assumes ordinary vertical flow; incompatible placement/parent-dependent heights use immediate layout.

No assets or background work are created. To adapt, keep IDs stable, apply semantic requests to the latest model, mirror drafts explicitly if they must survive Unmount, and use Bonsai branches/Eio scopes separately when panel departure must stop work. Do not infer task cancellation from a native hidden policy.

## Run and review

From the repository root:

```sh
./scripts/gpuio build examples/gallery/main.exe
./_build/default/examples/gallery/main.exe
```

The wrapper uses the repository toolchain. This component has no standalone executable or self-test. Native interaction requires the running gallery; compilation does not establish focus, keyboard, accessibility or platform acceptance. See [gallery instructions](README.md) and the [development environment](../../docs/development.md).

The focused macOS driver is `python3 scripts/test_gallery.py --section disclosure`.
It repeats the editing/policy interaction through Light and Dark appearances and
Comfortable, Large and Compact application sizes. Each case leaves and remounts
Navigation to reset its Bonsai model. It writes a Unicode draft, replaces it with
a keyboard edit, closes/reopens the retained native buffer and checks undo/redo.
It then changes single/multiple/nonempty modes, disables Behavior and turns off
Keep drafts while Identity is closed to check a new native buffer on reopening.
Hidden accessibility-node removal is checked separately from native buffer
retention; an AX object need not survive a hide/show cycle. The driver requires
macOS Accessibility access and a US/ABC input source and does not change the
input source. It does not establish VoiceOver speech or animation frame timing.
Home/End/Down move native heading focus without changing application expansion;
Down skips a disabled Behavior heading. The driver also checks whole-group
disabled state and the still-expanded Identity panel's content after recovery.
The [desktop evidence](../../docs/evidence/disclosure-macos-och41.md) records
the tested repository and installed-consumer binaries and remaining limits.
