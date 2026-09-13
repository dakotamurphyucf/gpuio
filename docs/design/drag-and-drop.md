# Drag and drop

OCH-11 implementation in progress. The typed data/configuration layer and its
OCaml/Rust codecs are implemented. Native gestures, view factories, event routing,
capability reporting and actual desktop tests are still pending. The transport
does not advertise a drag/drop capability yet.

## Data and application API

`Gpuio.Drag_and_drop` provides `Custom_kind`, `Format`, `File`, `Payload`, `Source`
and `Target`. Constructors preserve immutable values and perform no I/O.

| Payload | Meaning and bound |
| --- | --- |
| Text | UTF-8 without NUL, 0..262,144 bytes |
| Files | 1..128 entries, at most 262,144 total path bytes |
| Custom | Exact case-sensitive kind and 0..262,144 opaque data bytes |

Files reuse `File_path`: absolute Unix path bytes, no NUL, at most 16,384 bytes
per path. Preserve non-UTF-8 bytes, separators, dot components, ordering and
duplicates. No normalization, existence check, symlink resolution or automatic
file read occurs. Applications use an explicit Eio capability for subsequent I/O.

Each file also has optional caller-supplied directory metadata. Incoming native
paths may have unknown metadata. A source must supply metadata for every entry
before enabling `allow_desktop_files`; otherwise construction fails for the whole
source. Never perform a synchronous filesystem query at drag start or silently
export only a subset. The flag defaults to false and only permits file payloads.

Custom kinds contain 1..128 ASCII letters, digits or `._-/+`, for example
`com.example.task/v1`. They are application identifiers, with exact matching;
there is no implicit MIME negotiation, case folding, object deserialization or
cross-application custom-data transfer. Opaque data can contain NUL and invalid
UTF-8. Do not encode OCaml heap values or native pointers as transferable handles.

Sources have a label, payload, disabled flag and desktop-file opt-in. Targets
have a label, disabled flag and nonempty allowlist of at most 16 distinct formats.
Labels are nonblank UTF-8 without NUL, at most 4096 bytes. A source label also
provides its default native preview text. A target accepts an exact matching
format when enabled. Rust applies this declared policy synchronously; an OCaml
callback observes the result and cannot retroactively veto it. Provide ordinary
keyboard-operable commands as alternatives to pointer dragging.

Configuration construction currently looks like this; view integration follows
in the next checkpoint:

```ocaml
let open Or_error.Let_syntax in
let%bind kind = Drag_and_drop.Custom_kind.of_string "com.example.task/v1" in
let%bind payload = Drag_and_drop.Payload.custom ~kind ~data:"task-123" in
let%bind source =
  Drag_and_drop.Source.create ~label:"Move task" ~payload ()
in
let%map target =
  Drag_and_drop.Target.create ~label:"Tasks" ~accept:[ Custom kind ] ()
in
source, target
```

## Serialization and bounds

`Wire.Drag_and_drop` aliases a separate internal wire module. Source field order
is label, payload, disabled, allow_desktop_files; target field order is label,
accepted formats, disabled. Payload/format variant tags are Text=0, Files=1,
Custom=2. Files encode their path as a bin_prot string followed by an optional
directory Boolean. Custom data also uses bin_prot **string** encoding; Rust's
generic integer-per-element `Vec<u8>` encoding would corrupt byte compatibility.

Readers bound string lengths, file count, total path bytes and format count
before allocating the declared values. Invalid metadata, tags, text, identifiers,
duplicate formats or incompatible desktop offering fail the whole decode.
Rust payload/source/target representations are private and validated; the OCaml
application API uses abstract/private types. Expert conversion independently
validates manually constructed incoming wire payloads.

Independent checked-in hex fixtures specify the bytes in both languages. Tests
cover every fixture truncation, malformed input, oversized declared lengths,
inclusive limits, non-UTF-8 paths/custom bytes, and exact format ordering. The
standalone Rust source/target decode entry points reject trailing data. These
data tests do not prove native event delivery or OS drag support.

## Native integration constraints

Inspected pinned GPUI revision
`a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b`:

- `crates/gpui/src/interactive.rs`: `ExternalPaths`, `ExternalDragPayload`,
  `FileDragPaths`, `FileDropEvent`.
- `crates/gpui/src/elements/div.rs`: typed `on_drag`, `on_drag_move`, `on_drop`,
  `can_drop` and `external_drag_payload`.
- `crates/gpui/src/window.rs` and `app.rs`: active-drag dispatch, promotion to
  platform ownership, restoration and release.
- `crates/gpui_macos/src/window.rs` and the Linux Wayland/X11 window adapters:
  outbound platform support and startup requirements.

The pinned external payload enum supports files. macOS and Wayland implement
outbound file dragging; X11 uses the unsupported default. A Wayland capability
check alone does not prove a particular start will succeed: platform devices,
serials and other state still matter. GPUI does not expose an authoritative
completed external copy/move result. Offering a payload must never be reported
as a successful external operation, nor used to authorize file deletion/moving.

GPUI's current `on_drop` takes its active drag before evaluating `can_drop`.
Therefore a rejecting child can consume a matching typed drag before an accepting
parent handles it. The planned GPUIO target adapter uses its own bubble listener:
check current hitbox, eligibility and format first; consume and stop propagation
only on acceptance. Test a rejecting nested target with an accepting ancestor.
This does not require a GPUI patch.

The native source must snapshot immutable data at gesture start and retain it
through the actual gesture/OS handoff. Payload updates for the same owner must
not mutate an active offer. Track gesture identity separately from node identity;
validate live owners and revisions. Avoid retaining full payload copies in each
motion event or preview render. Hover/motion can carry bounded offer metadata;
deliver the full payload on drop. Lifecycle edges must not coalesce.

Native implementation must distinguish an internal accepted drop, local
cancellation, an external offer, and unconfirmed completion. A removed source
must stop local delivery and release framework state, but GPUI's public
`stop_active_drag` cannot promise cancellation of an already suspended OS drag.
Do not infer internal source identity by matching incoming file paths. Typed
cross-window behavior needs native evidence before being documented as supported.

## Remaining acceptance

1. Add view/reconciler and tree/protocol operations, event contracts, bounded
   mailbox accounting and capability negotiation together with native support.
2. Implement immutable gesture ownership, target hover/drop dispatch, native file
   conversion, cancellation and cleanup across disable/hide/remove/close/shutdown.
3. Validate nested rejection fallback, payload updates, blocked focus scopes,
   terminal delivery once, native file offers and unsupported backend behavior.
4. Run an OCaml/Bonsai example through actual local macOS native dragging; keep
   Linux build/unit gates and defer full Linux GUI acceptance to OCH-17.
5. Publish evidence and complete the remaining OCH-11 families and consolidated
   CI/merge. This data checkpoint does not complete OCH-11 or milestone 2.
