# Document link activation — OCH-41

`Document.Navigation.Link` carries `{ url; activation }`. Activation is optional
only for decoding the original URL-only wire variant; current native readers
always supply it. `Document.Activation` names Mouse (with button), Keyboard or
Touch (with long-press flag), plus the existing five Pointer modifier flags.
Mouse flags describe release, matching GPUI ClickEvent. Keyboard activation is
unmodified Enter in the reader; synthetic accessibility activation uses the
same Keyboard route. These two causes cannot be distinguished by the pinned
ClickEvent and must not be labelled as physical keyboard evidence. Touch uses
zero modifiers. Mouse auxiliary buttons are observations, not implicit URL opens.

No callback opens a URL automatically. The application decides asynchronously
whether to open a tab, show a menu or route elsewhere. The native click's current
source generation/revision, live presenter and modal focus guards still apply.
Core/Eio discard stale handler, source-generation and view events as before.
No synchronous OCaml callback is introduced. Coordinates/counts and raw native
event objects are not part of this routing contract.

The existing Event30 and Navigation tags0/1 stay unchanged. Navigation tag2
appends `Link_activated(url, { source; modifiers })`. Source tags are Mouse0
(with existing Pointer button), Keyboard1 and Touch2 (strict long-press Boolean).
Nonmouse modifier flags reject. Empty/invalid/oversized URLs reject under the
existing 4096-byte contract. Paired runtimes rebuild together. Public Link's
payload changes from string to record in this pre-release API; update callers to
match `Link { url; activation }`. Legacy wire payloads map to activation=None,
not fabricated input metadata.
