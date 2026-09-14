# Scoped images

Run `./scripts/gpuio exec dune exec examples/images/main.exe` to display a generated
PNM asset through the public Eio/Bonsai image API. It uses no filesystem/network
acquisition. For file or remote images, acquire bytes with explicit Eio capabilities
before constructing `Asset.Source` and registering it under an application scope.

`./scripts/gpuio exec dune exec examples/images/main.exe -- --self-test` opens a
short-lived native window and checks upload → native Ready, restyling after asset
retirement, rejection of a newly keyed mount, and shutdown. It tests public FFI/event
integration; exact GPU pixels and macOS accessibility are checked by the separate
`native_image_views` target. SVG/icon support remains in progress.
