# How the Rust counter factory owns native state and callbacks

[Package README](../../README.md) · [Source](lib.rs)
· [OCaml consumer API](../../ocaml/gpuio_example_counter.md)

This is component-author Rust code using only the public pinned extension SDK
and its GPUI re-export. The composer statically links `factory()` into the chosen
backend; it is not a binary plugin ABI or an OCaml Bonsai component.

`Factory::descriptor` pairs the counter name/version/fingerprint with SDK/GPUI
revisions and payload maxima 2/1/1. Properties accept `[value, step]` with value
0–100 and step 1–10; commands accept one value byte. Validation precedes indexing.
`mount` creates `Counter` with an `Rc<CounterValue>` wrapping native `Cell<u8>`
and optional trace identity. `update` sets properties on the existing state;
`command` applies the already sequenced host command. Host generation/epoch rules
and catalog checks are separate from this package's byte validators.

`render` constructs one accessible Button-like native div using the host focus
handle and a current event sink. Disabled accessibility metadata is applied to
the actual button, because the host group cannot propagate it automatically.
Rendered labels show current native value. Closures retain an Rc value copy,
step, event sink, and focus handle; they retain no OCaml model or Window/App
reference beyond callback invocation.

On activation, the closure computes a saturating step capped at 100. It calls
`events.emit` **before** mutating local value, so rejected admission does not hide
a local change. Success updates state, focuses the control, and refreshes the
window. Rust queues one-byte observation; later host/OCaml decoding returns Data
and the consumer's Bonsai setter updates application text/properties. There is
no synchronous OCaml callback during native render/layout.

Real mouse clicks use `guard_pointer`; other click events and accessibility use
`guard`. These current-lease checks reject hidden/obsolete input and contain
ordinary unwinding panics. PointerEvents policy can suppress pointer interaction
without removing keyboard/AX use. GPUI's click listener already handles focused
Enter/Space; adding a second key-down handler would double-activate. See
[SDK guard contract](../../../../rust/extension-sdk/src/lib.rs).

`unmount` traces retirement. Counter Drop and CounterValue Drop are distinct:
callbacks can retain the Rc payload after the component object drops. With
`GPUIO_COUNTER_TRACE=1`, logs distinguish mount/unmount/component_drop/value_drop
and accepted command values. Normal runs are silent. Trace ID is diagnostic,
not part of schema, native ABI, or persistent application identity.

From the repository root using [development setup](../../../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo check --offline --locked -j2 -p gpuio-example-counter
GPUIO_JOBS=2 ./scripts/gpuio build examples/extension_consumer/main.exe
GPUIO_COUNTER_TRACE=1 _build/default/examples/extension_consumer/main.exe
```

The crate has no binary or local test module; Cargo check proves compilation,
not lifetime/input acceptance. The consumer requires a native graphical session.
Its --smoke path observes a sequenced command/frame then closes; it does not test
physical clicks/IME/VoiceOver. Signal Studio's separate workload checks lifetime
traces; see the package README for that evidence boundary.

The manifest's components entry selects this factory. Run
`python3 scripts/compose_backend.py manifest.json generated-directory`, review
output and Cargo.lock, and select that generated Dune backend. Paths are relative
to the manifest. Source packages pin compatibility; schema or factory construction
alone does not register behavior after catalogs freeze. When extending, retain
revocable guards and admission-before-mutation, cancel owned work on unmount,
and validate all payloads before allocating/indexing.
