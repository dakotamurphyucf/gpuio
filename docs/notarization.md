# macOS notarization handoff

This procedure starts with a Developer ID reference package from the
[distribution guide](distribution.md). It prepares a separate stapled archive;
it does not complete runtime, license or clean-machine release acceptance.
The implementation has portable tests with simulated Apple commands. An actual
credentialed submission and finalization have not yet been qualified for GPUIO.

## Prerequisites and submission

Build and package the intended release source, supply reviewed notices and select
the Developer ID certificate/team explicitly. Retain the build evidence linking
the executable hash to that source. Keep the resulting `package.json` and ZIP
unchanged. The original package's loose `.app` is not the finalizer's input.

Configure an existing `notarytool` Keychain profile on the signing Mac using the
organization's credential procedure. Use the profile name in commands; do not
put passwords, private keys or API credentials in this repository or logs.
Apple documents the available credential methods and submission workflow in
[Customizing the notarization workflow](https://developer.apple.com/documentation/security/customizing-the-notarization-workflow).

For a gallery package, the following commands submit the ZIP explicitly and save
the response. Adapt the package path and profile to the actual prepared inputs;
use a new evidence directory for each submission. This uploads the archive to
Apple. The finalization script below never submits or resubmits it.

```sh
mkdir scratch/notary-gallery-submission-001
cp scratch/package-gallery-signed-001/package.json \
  scratch/notary-gallery-submission-001/submitted-package.json
xcrun notarytool submit \
  'scratch/package-gallery-signed-001/GPUIO Component Studio.zip' \
  --keychain-profile "$GPUIO_NOTARY_PROFILE" --output-format json \
  > scratch/notary-gallery-submission-001/submission.json
```

Check the command's exit status and preserve the returned submission `id` before
waiting. `submit` without `--wait` returns after upload; upload success is not
acceptance. Record that UUID as `GPUIO_NOTARY_SUBMISSION`, alongside the original
archive SHA-256 in the saved package report. Keep submission records private if
they include account or internal build information.

```sh
xcrun notarytool wait "$GPUIO_NOTARY_SUBMISSION" \
  --keychain-profile "$GPUIO_NOTARY_PROFILE" --timeout 5m \
  --output-format json > scratch/notary-gallery-submission-001/wait-001.json
```

A wait timeout does not cancel Apple's processing. Resume `wait` with the same
ID and a fresh log filename. If upload returned ambiguously before an ID was
saved, inspect `notarytool history` and correlate the submission before uploading
again. For a rejected submission, retrieve its log with `notarytool log`, repair
the signed package and create a new submission. Never reinterpret a timeout or
an `In Progress` result as success. Apple's workflow guide also recommends
reviewing warnings in accepted logs.

## Finalize the accepted submission

After the service reports acceptance:

```sh
python3 scripts/finalize_macos_notarization.py \
  --package scratch/package-gallery-signed-001 \
  --output scratch/package-gallery-notarized-001 \
  --submission-id "$GPUIO_NOTARY_SUBMISSION" \
  --keychain-profile "$GPUIO_NOTARY_PROFILE"
```

The command requires macOS and a fresh output directory outside the submitted
package. It reads the archived bytes, rather than trusting the loose input app:

1. Check the archive hash, paths, app metadata, executable and notice hashes;
   extract a private copy and verify its exact Developer ID certificate, team,
   app ID, hardened runtime and timestamp.
2. Retrieve the existing submission's log. Require format version 1, the same
   submission ID, status `Accepted` and the exact submitted archive SHA-256.
   Preserve warnings; reject errors and unexpected log structures. These are
   explicit tool admission requirements. If Apple's response omits the hash or
   changes format, investigate it rather than bypassing the binding check.
3. Staple and validate the copied app, then verify its signature again and check
   that its executable bytes are unchanged. ZIP archives cannot themselves be
   stapled; the ticket belongs to the contained app.
4. Create a new ZIP, extract it again and repeat package/signature checks and
   ticket validation. Mark output complete only after this round trip passes.

The output contains the app, final ZIP, `notary-log.json`, `notarization.json`
and a new `package.json`. Both reports must have `complete=true`. Preserve and
review `warnings` and the original log. The final ZIP has its own hash; the
reports retain both that hash and the hash of the ZIP Apple accepted. Transfer
the **final** ZIP with its corresponding final package report, not the original
submission ZIP or a mixture of reports from the two directories.

The bundle's sealed `Contents/Resources/Build.json` remains the packaging-time
record. Finalization updates the external reports, not signed bundle metadata.
Do not re-sign or edit the app after finalization; changed app code requires a
new signed package and submission.

Each Apple command has a bounded two-minute tool timeout. Failures after output
creation preserve incomplete reports and available artifacts for diagnosis.
Keep that directory; retry finalization into a new directory with the same
submission ID and original input when the failure is transient. Preflight
failures occur before output creation. The tool does not overwrite a prior run,
import credentials, change Keychain settings, launch the app or upload code.

## What still needs release evidence

Actual Developer ID signing and Apple's acceptance must be exercised with the
selected release identity; portable tests do not prove either. Test the final
downloaded archive on a supported clean Mac with quarantine retained, recording
Gatekeeper assessment and actual app launch/behavior. Do not remove quarantine
to obtain a passing result. Follow the distribution guide's receiver checks,
and retain build, notice-review, signing, notarization and transfer evidence for
each reference application. A valid stapled ticket alone is not release approval.
