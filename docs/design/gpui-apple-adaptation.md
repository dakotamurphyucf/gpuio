# Pinned Apple renderer adaptation

The `gpui_apple` crate is vendored from the same Zed revision as GPUI core and the
macOS platform. `third_party/sources.json` records the archive and patch hashes.
The source manifest is preserved as `Cargo.toml.upstream`, and the Apache license
is copied from that verified archive. No shared Cargo checkout is modified.

The only behavioral adaptation is the default-off `presentation-diagnostics`
feature. It implies the matching GPUI core feature and attaches a Metal
`addPresentedHandler` after successful frame encoding, before either existing
presentation path. `presentsWithTransaction`, command-buffer commit/wait, drawable
presentation and normal frame scheduling retain their original implementation.
With the feature disabled, none of the added measurement code is compiled.

The callback captures only a `gpui::presentation::NativeFrame` ticket. It does
not capture the drawable, command buffer, layer, view, renderer or window. Metal
supplies the callback drawable; its OS-reported `presentedTime` and the separately
observed callback host time are reported to the bounded core collector.
`CACurrentMediaTime` is a thread-safe host-clock reading and requires no owned
native object. The core's bracketed clock sample converts matching native-input
ages without assuming an `Instant` epoch. No callback calls OCaml, performs I/O
or schedules another frame.

GPUIO exposes this through opt-in
`gpuio_native::performance::presentation::Session`. The wrapper rejects
TestPlatform and non-macOS windows as `Unsupported`. Starting/stopping/snapshotting
never creates a timer. Session stop closes admission; the qualification driver
has a finite settlement deadline and must report remaining pending callbacks.
See [the full measurement contract](metal-presentation-qualification.md).

Root and generated application Cargo patches select this exact renderer. Their
Dune source dependencies include `vendor/gpui-apple`; the composer and standalone
table probe reproduce that selection. Reconstruct into a fresh directory:

```sh
python3 scripts/vendor_gpui.py --crate gpui_apple --output FRESH_DIRECTORY
```

`--archive` accepts the same hash-verified local Zed archive. The current patch
adds one Cargo feature and the small renderer hook; reconstruction must match all
committed files. Default macOS/Linux builds and independent consumers remain
required. Linux does not gain a Metal presentation capability from this change.
Native hook evidence is separate from optimized workload, overhead and GPU/resource
acceptance; see [local qualification](../evidence/metal-presentation-hook-och17.md).
