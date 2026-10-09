# Developer ID packaging tooling — 2026-10-08

The reference packager now accepts an explicit Developer ID Application
certificate fingerprint and Apple team. It signs the private assembled bundle
with hardened runtime and timestamping, verifies its certificate chain, exact
certificate/team/app identity and every architecture's runtime/timestamp metadata,
then hashes and archives the result. Signature failure leaves `complete=false`
and prevents archive creation. Runtime extraction repeats native verification.
Existing unsigned/ad-hoc callers retain their behavior.

On macOS 14.5 arm64, against base `e58cec44` plus this change:

```sh
python3 scripts/test_package_macos_reference.py
python3 scripts/test_package_runtime_inputs.py
python3 scripts/audit_example_docs.py
git diff --check
```

Results: **14 packaging tests and 4 runtime-input tests pass**; documentation
inventory passes for 430 sources / 267 reviewed groups; diff check passes.
The macOS-only test actually compiles the requirement using `/usr/bin/csreq`,
copies `/usr/bin/true` into a temporary fixture bundle, ad-hoc signs that copy,
and verifies that the native Developer ID requirement rejects it specifically
for failing the code requirement. It also checks the native per-architecture
display command. Nothing is launched and the temporary fixture is removed.
Linux runs the portable tests and skips that OS-specific test.

Portable tests cover malformed options before filesystem work, certificate/team/
app constraints, absent runtime/timestamp, duplicate or mismatched signature
metadata, an invalid second architecture, and incomplete reporting/no archive on
signing failure. Mocked positive metadata is not a real Developer ID signature.

No valid code-signing identity was available locally at preflight. **Actual
Developer ID signing and hardened-runtime behavior are unqualified.** Credentials,
notarization submission, stapling, quarantined receiver testing, reviewed notices
and final release acceptance remain required. This change imports no credentials,
changes no keychain policy and submits no artifact to Apple. See the commands and
Apple references in [distribution](../distribution.md).

Separately, the full isolated `GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2
@runtest @fmt` command completed successfully at `e58cec44`, including the shared
form change. It reports existing block 0.1.6 and duplicate system-library linker
warnings; this is not a clean rebuild or GUI qualification.

## Notarization finalization tooling — 2026-10-08

The [handoff procedure](../notarization.md) now covers submission with a saved
ID, bounded waits and recovery, followed by finalization of an accepted
submission into a separate stapled archive. The finalizer never uploads code.
It binds the accepted log to the original submission UUID and ZIP hash, verifies
the extracted Developer ID bundle, staples it and verifies the new archive after
round-trip extraction. Failed operations preserve incomplete reports. The input
package and signed build metadata remain unchanged.

Against `e41f5799` plus the recorded source hashes, macOS 14.5 arm64 checks pass:

```sh
python3 scripts/test_finalize_macos_notarization.py
python3 scripts/test_package_macos_reference.py
python3 scripts/test_package_runtime_inputs.py
python3 scripts/test_package_transfer.py
ruff check scripts/finalize_macos_notarization.py scripts/test_finalize_macos_notarization.py
python3 -m py_compile scripts/finalize_macos_notarization.py scripts/test_finalize_macos_notarization.py
```

Results: **5 finalization, 14 packaging, 4 runtime-input and 5 transfer tests
pass**, alongside Ruff, syntax, workflow lint and diff checks. The four test logs,
their hashes, source hashes and platform are in
[the evidence summary](notarization-tooling-och17/summary.json).

The five new tests simulate Apple's log, signing and stapler commands while
using actual temporary ZIP files and package validation. They cover preserving
the input, retaining warnings, rejecting wrong/malformed submission identity or
status/hash, rejecting a signature before service lookup, refusing ad-hoc input,
and detecting a ticket lost from the final archive. The existing packaging suite
also repeats the actual macOS rejection of an ad-hoc signature described above.
Foundation runs the portable finalization checks on both platform jobs.

No app was launched, artifact submitted or credential imported. **Actual Apple
service compatibility, Developer ID signing, stapling and quarantined receiver
acceptance remain unqualified.** An accepted mocked response is tooling evidence,
not a notarized release. The finalizer deliberately rejects unfamiliar log
formats rather than assuming an unverified archive was accepted.
