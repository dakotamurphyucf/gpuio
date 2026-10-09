# Notice input refresh — OCH-17

After `394039d`, the current locked macOS native, Signal and gallery-consumer
collections exposed three stale supplemental manifest identities:
`gpuio-native`, `gpuio-counter-backend` and `gpuio-signal-backend`.
Their [exact old/new manifests and deltas](notices-current-och17/notice-manifest-drift.json)
show only the maintained macOS/Apple path patches, optional presentation feature,
dev dependency and test target. Package identity/license declarations and the
attributed root LICENSE bytes are unchanged. The three selectors were revalidated
and updated; no other package identity, license alternative or source text changed.

Fresh offline/locked collections and independent byte/hash verification pass:

| Graph | Packages | Verified local + supplemental texts | Rows without collected text |
| --- | ---: | ---: | ---: |
| Native | 513 | 868 | 26 |
| Signal | 485 | 826 | 26 |
| Gallery/independent counter backend | 486 | 827 | 26 |

All2,521 copied texts match their recorded SHA-256 and byte counts. The new local
GPUI Apple source supplies its own retained license file, moving attribution from
a supplemental upstream record to local discovery without losing the text.
Ten collector regression tests pass.

[Verification summary](notices-current-och17/current-notices-verified.json) and
[complete inventory/metadata/lock/text archive](notices-current-och17/inventories.tar.gz)
retain exact identities, commands and source graphs. Collections used
`scripts/collect_rust_notices.py --target aarch64-apple-darwin` with roots
`gpuio-native`, `gpuio-signal-backend` and `gpuio-counter-backend`, respectively;
the two independent manifests are `examples/signal_studio/backend/Cargo.toml` and
`examples/extension_consumer/backend/Cargo.toml`. All use
`--supplemental third_party/notice-sources.json` and fresh output directories under
the agent's ignored scratch workspace. No compilation, dependency resolution
change or switch mutation was performed.

This keeps the audit reproducible at the current source; it does not resolve the
26 previously classified missing-text rows or complete distribution review.
Original discovery issues remain visible. Full notice/asset/system review and
reviewed release bundles are still required; these audit collections must not be
misrepresented as approved release-notice directories.
