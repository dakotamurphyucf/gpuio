# Native scrollbar preference snapshot

OCH-41: expose the preference read used by the pinned Component theme helper
through the asynchronous desktop request lane. This is a snapshot, not a
subscription or an application-global theme mutation.

`Gpuio_eio.Desktop.scrollbar_preference app` returns
`(Gpuio.Desktop.Scrollbar_preference.t, Desktop.Error.t) result Effect.t`.
The values are `Auto_hide` and `Always_visible`. Read the preference when the
native main thread services the request. No desktop identity is required. The
existing bounded request correlation, shutdown and late-response rules apply.
No window activation, timer, filesystem operation or callback into OCaml occurs.

On macOS the pinned GPUI getter reads `NSScroller.preferredScrollerStyle`:
overlay maps to `Auto_hide`, legacy to `Always_visible`. This reports the resolved
style, not the three-way System Settings choice or the reason it resolved that
way. On Linux the pinned getter returns a field initialized to false with no
updater; return `Unsupported` instead of representing that constant as OS evidence.

Applications choose how to apply the snapshot. The gallery's Collections page
provides an explicit “Use system preference” action: auto-hide selects `Scrolling`,
always-visible selects `Always`. An error preserves the preceding mode. This
intentionally differs from upstream's legacy-to-`Hover` theme mapping, honoring
the snapshot's always-visible meaning. Only the collection previews change.
Refreshing is explicit after changing system settings; no live tracking is claimed.
One request can be pending per preview activation. Leaving the page invalidates
its completion; later activations/windows cannot inherit an old response. Explicit
mode buttons are disabled during that single request so a late reply cannot
overwrite a newer manual choice.

Wire additions append request tag 7 and response tag 6 with a two-value variant
(auto-hide 0, always-visible 1). Existing tags remain unchanged. Both sides must
come from the same release, as for other bridge additions. Test independent
bytes, malformed response tags/truncations, unsupported backend policy and
asynchronous lifetime behavior. TestPlatform cannot certify the real macOS setting;
physical settings/preview behavior remains a separate release check.
