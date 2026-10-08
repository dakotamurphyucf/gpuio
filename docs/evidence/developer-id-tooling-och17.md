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
