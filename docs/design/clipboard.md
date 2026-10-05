# Plain-text clipboard writes and copied feedback

OCH-41 implementation contract, following the
[pinned Clipboard review](../catalog/assets-clipboard-review.md).
[Local tests and macOS gallery evidence](../evidence/clipboard-och41.md) now cover
the scoped operation; consolidated release qualification remains open.

`Gpuio.Clipboard.Text` admits at most 256 KiB of valid UTF-8 without NUL, including
empty text, preserving newlines/tabs/Unicode without normalization. Native decode
checks the same limit before allocation and dispatch validates again before any
clipboard mutation. This is one bounded plain-text payload, not a file/resource
handle, arbitrary format bundle or implicit clipboard read.

`Gpuio_eio.Clipboard.write_text app text` is an explicit application-scoped effect.
It queues through the existing shared desktop lane (16 pending requests), with
owned bytes and a correlated asynchronous result. No desktop identity or window
is required. Rust invokes GPUI's clipboard write on its UI thread; no Rust input,
layout or native delegate callback synchronously calls OCaml. Success means that
API was invoked: GPUI supplies no persistence/recipient-delivery receipt. Another
application can replace the clipboard immediately. Queue saturation reports Busy;
application shutdown closes pending replies. Canceling/unmounting the caller does
not retroactively undo an admitted write.

The request appends `Write_clipboard_text` at desktop request tag 8 inside existing
Message19. It reuses Response.Requested and the existing explicit failure envelope.
The bridge's unpublished epoch3 still requires matching OCaml/native sources;
this is not interoperability with older epoch3 binaries or a new capability bit.
No operation-count, command-byte or queue limit is raised.

`Gpuio_eio.Clipboard.Copy.create` accepts reactive validated text, optional disabled
state and an optional copied callback. Its state machine captures the current
application value when it reduces an activation, admits one write, and publishes
copied feedback only for the current successful reply. This is intentionally
asynchronous compared with the upstream Rust `value_fn` executed inside a click.
Busy and two-second copied feedback suppress duplicate activations. A text or
disabled change, or Bonsai deactivation, revokes the old feedback/callback token.
Already-dispatched OS writes remain committed operations. Failed writes expose a
typed error and require an explicit retry; no automatic clipboard polling occurs.

The copied deadline uses `Bonsai.Clock.at` with the shared v0.17 runtime clock.
There is no per-button polling fiber or callback object crossing FFI. Public
`Copy.view` composes an ordinary native button with Copy/Copied labels, optional
style and loading policy. `copy`, `is_busy`, `is_copied` and `error` allow icon and
tooltip compositions without a new native widget class. Native button focus,
keyboard and accessibility behavior is retained.

The public Assets gallery demonstrates a literal string and a current approval
value, Unicode/newline content, feedback, value changes and failure reporting.
Required evidence includes independent paired bytes, UTF-8/length/NUL rejection,
no mutation on malformed native requests, correlation, delayed/stale callbacks,
disabled/deactivation behavior, duplicate suppression, shared-clock expiry,
actual macOS clipboard readback/restoration and public keyboard/AX operation.
Linux build/unit/consumer checks remain required; X11/Wayland clipboard behavior
is deferred desktop qualification under OCH-47, not a compilation claim.
