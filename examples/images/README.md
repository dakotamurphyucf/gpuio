# Scoped images

Run `./scripts/gpuio exec dune exec examples/images/main.exe` to display a generated
PNM asset through the public Eio/Bonsai image API. It uses no filesystem/network
acquisition. For file or remote images, acquire bytes with explicit Eio capabilities
before constructing `Asset.Source` and registering it under an application scope.

`./scripts/gpuio exec dune exec examples/images/main.exe -- --self-test` opens a
short-lived native window and checks upload → native Ready, restyling after asset
retirement, rejection of a newly keyed mount, and shutdown. It tests public FFI/event
integration; exact GPU pixels and macOS accessibility are checked by the separate
`native_image_views` target.

Add `--svg` to display a full-color embedded SVG or `--icon` for a monochrome SVG
using the view foreground. Either flag combines with `--self-test`, exercising the
same public registration, native metadata, retirement and remount contracts.
The SVG canvas adapts to native measured size/device scale; icons also adapt to
foreground changes without requiring an OCaml event for each native state change.


The icon mode also shows leading/trailing button decorations, a labelled icon-only
button, and a command button whose registry label updates when clicked. The self-test
removes a trailing decoration after registration retirement while retaining the other
mounted icons. Native button-input/pixel/accessibility checks are in `native_image_views`;
the public self-test covers accepted FFI trees and lifecycle, not simulated user clicks.
