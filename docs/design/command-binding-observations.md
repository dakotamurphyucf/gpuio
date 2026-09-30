# Native command-binding observations

OCH-41 implementation design. The typed [keyboard-label display API](keyboard-labels.md)
is implemented; this observation API is **not yet public or implemented**. The
shared native resolution foundation below is implemented and locally tested.
This work completes the remaining action/context/focus lookup portion of the
pinned Kbd source. It does not add multi-stroke input, synchronous Rust-to-OCaml
callbacks or an application keymap parser.

## Resolution already shared with invocation

`Tree.commands_from` enumerates the current native ancestor registries, nearest
scope first and in declaration order. It deduplicates command IDs **before** any
shortcut/input/availability filtering. An inner definition therefore hides the
outer ID even when disabled, without shortcuts, assigned a different chord or
restricted to a different routing phase. Siblings do not participate. The
iterator borrows existing shared command data and retains no payload after use.

`command.rs` now uses that iterator for actual shortcut routing. Its shared
`InputContext` distinguishes native navigation, composition and text-input gates,
and uses GPUI's own key matcher after physical modifier resolution. Primary and
Super/Control aliases agree with the typed display API. Unmodified/Shift-only
editor typing is excluded by ModifiedOnly; modifier-free Always is intentional.
Native-first Tab/Shift-Tab and ordinary button Enter/Space stay reserved. The early
navigation path avoids scanning registries.

Selection and invocation are distinct. The first matching declaration reserves a
chord **within its phase**, even if invocation fails because it is disabled,
blocked by a modal scope or unavailable for the retained native edit target. It
must not fall through to a later declaration in that phase. A failed Override
invocation does not itself consume the event: native handling and NativeFirst
may still follow. A successful Override consumes it before native handling.
NativeFirst is only reached if native handlers let the event bubble.

The observation implementation must reuse these policies rather than duplicate
an approximation in OCaml. OS-reserved events, keyboard-layout translation and
arbitrary widget handlers prevent an unconditional promise that a displayed chord
will invoke the command.

## Planned public interface

Draft shape; names and representations must be finalized with the paired wire
schema before exporting them:

```ocaml
module Command_binding : sig
  module Context : sig
    type t = Focused | Here
  end
  module Config : sig
    type t
    val create
      :  ?context:Context.t
      -> Command.Id.t list
      -> t Core.Or_error.t
  end
  module Observation : sig
    type t
    (* Ordered entries and a monotonically increasing native observation version. *)
  end
end

val command_binding_scope
  :  ?key:Key.t
  -> ?style:Style.t
  -> config:Command_binding.Config.t
  -> on_update:(Command_binding.Observation.t -> 'action)
  -> 'action View.t list
  -> 'action View.t
```

The wrapper adds no focus stop. It provides a mounted, generation-checked native
query owner and an ordinary asynchronous callback. A Bonsai adapter exposes the
latest observation as a value, pending until the first native sample; display can
then use `Presentation.Kbd`. Removing that branch removes the subscription.

`Focused` samples the actual focus context for that window, using the same root
fallback as invocation. `Here` resolves definitions at the observer's native tree
position: a structured command scope replaces source-specific context strings.
These are deliberately different answers. A Here result describes declarations
in that scope; it must not apply input/IME state from an unrelated focused editor
and call them currently usable. Its candidate disposition is Declared.

For Focused results, preserve all candidates in declaration order (currently at
most four per command). Report the selected definition's enabled state and whether
each candidate is eligible for Override, conditional on native-first delivery, or
suppressed by an explicit reason. Missing definition and present-with-no-shortcuts
are distinct. Relevant suppression reasons include disabled definition, blocked
scope, unavailable native edit action, composition, text-input policy, reserved
native navigation and another command winning resolution.

Conflict inspection must run the actual phase-selection rules. For a NativeFirst
candidate, consider a matching Override only if that route would be invocable;
an unavailable Override does not consume the event. Within either phase, an
unavailable earlier matching declaration still blocks later declarations. Test
both cases explicitly. A native-first candidate remains conditional even when
all known GPUIO gates pass; querying must never execute a command as a probe.

## Native action bindings are a separate source

The pinned GPUI `Window.highest_precedence_binding_for_action` reads its own
rendered dispatch tree; the `_in` variant resolves a specified native focus
handle, and `_in_context` resolves a parsed GPUI key context. The highest-precedence
GPUI binding is the last applicable keymap declaration. That is distinct from
GPUIO's nearest-scope/first-declaration registry policy. Do not apply one ordering
to the other or infer widget bindings from `Command.shortcuts`.

The native input widget registers Copy, Cut, Paste, SelectAll, Undo and Redo in its
own context. A GPUIO `Command.native` with no declared shortcut can therefore
coexist with a working widget keybinding. The final query API must account for
both sources explicitly, for example typed targets `Command of Command.Id.t` and
`Native_action of Command.Native.t`, with distinct result provenance. Do not report
an absent registry declaration as evidence that the native widget has no shortcut.
Use the actual GPUI lookup for native actions; do not synthesize platform defaults.

Native action/context support needs finalized typed context semantics before the
wire schema is frozen. The raw GPUI focus handle stays native; a query owner or
validated existing editor lease can identify its target. A scoped registry Here
query does not by itself reproduce a hypothetical widget focus context. Missing
or retired widget context must be explicit. GPUI bindings may contain multiple
strokes or a Function modifier absent from GPUIO's single-chord input type. The
source Kbd displays only the first stroke and ignores Function; do not silently
claim a fully invocable chord after dropping unsupported data. Define a display-
only native-stroke representation or an explicit unsupported disposition, and
record that mapping in the source ledger. This remains required design/delivery
work before full Kbd acceptance.

Pinned local evidence: `vendor/gpui/src/window.rs` action-binding APIs and
`vendor/gpui-base/src/input/base/state.rs` native edit keymap registration.

## Bounds, scheduling and staleness

Planned admission bounds: 1..64 unique command IDs per observer and at most 64
observers per window; the existing 256-byte ID and four-shortcuts-per-command
limits still apply. Bound aggregate configuration, queued results and retained
candidate metadata in the paired codecs and native session. Decode and validate
before atomically publishing a tree mutation. Add capability negotiation before
the public observer can be used against an older backend.

Sampling should occur after native focus/registry state is available, including
focus restoration and modal transitions. Reuse the native post-paint lifecycle
or explicit invalidation hooks, with tests proving that focus, composition, native
editor availability and registry changes trigger updates. Do not assume a cached
root renders whenever a child editor changes: inspect the pinned GPUI lifecycle
and verify that case. No permanent timer or request-next-frame loop is allowed.
Unchanged snapshots emit nothing; an observation changing a label must settle
without a render/observation feedback loop. Tree revision changes alone are not
a reason to emit another identical result.

Carry originating window generation, observer node/handler generation, query
configuration generation and a monotonically increasing native observation
version. Track relevant native focus/registry provenance; replace obsolete
undrained samples with the latest result for the same owner. The native producer
must verify the owner/configuration still exists before enqueueing. OCaml rejects
retired/reconfigured owners and out-of-order observations, including delayed events
after page remount or window close. A stale cached native task must not publish
into a replacement owner. Counter exhaustion needs an explicit terminal policy.

An observation is a snapshot, not a lock on future keyboard focus. Even a correctly
fenced result can become historical while crossing the asynchronous bridge.
Expose/document pending and observation-version semantics, and never use this
API to bypass current command invocation validation. Scope/configuration changes
must clear the adapter's old value while awaiting a fresh result; they must not
silently relabel an old observation as current.

Queued observations use a bounded latest-value lane, without competing away
lossless command invocations or growing an unbounded per-focus history. Unmount
and window close remove pending samples and release retained definitions. Hidden
or retained pages need an explicit pause/recovery rule consistent with the
repository's visibility semantics. Independent windows must not share results.

## Foundation evidence and remaining delivery

Local macOS 14.5 arm64, 2026-09-30:

- Three policy tests cover native-navigation reservations, input/composition
  gates, physical aliases and routing phase.
- Three registry tests cover nearest/ordered declarations, disabled and unbound
  shadowing, siblings, agreement with named lookup, context reparenting, definition
  replacement and weak-reference payload release. Repeated queries leave tree
  revision and retained-byte accounting unchanged.
- The existing command-session test still passes atomic reference and generation
  guards. The complete native library unit suite passes (416 passed, two ignored).
- The foreground native-controls fixture passes new unbound shadowing,
  cross-phase Override, disabled conflicting commands and declaration reorder
  assertions. A disabled Override lets a different NativeFirst command run; an
  enabled Override consumes before it, regardless of declaration order. These pass together with its existing command, menu, palette, composition,
  focus/restoration, independent-window and disposal cases. It uses GPUI-injected
  keystrokes and native widget/platform adapters; it is not physical-keyboard,
  real OS IME or VoiceOver qualification.

Commands:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -j2 -p gpuio-native --lib --test commands --test command_resolution --locked
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -j2 -p gpuio-native --features native-tests --test native_controls --no-run --locked
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -j2 -p gpuio-native --all-targets --features native-tests --locked -- -D warnings
# Run the resulting native_controls executable under a 240-second process-group watchdog.
```

The final native executable and strict all-target Clippy exited zero; owned test
children were reaped. The wire schema,
observer lifecycle, Core/Bonsai callback/value integration, actual desktop gallery,
installed consumer and consolidated release evidence are still required. Do not
mark Kbd source parity, OCH-41 or the milestone complete from this foundation.
Required Linux build/unit/private-bus/consumer checks remain; full Linux desktop
qualification stays deferred to OCH-47.
