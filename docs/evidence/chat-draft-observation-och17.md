# Chat dates/colors draft observation

OCH-17 / C2, 2026-10-08. Foundation run
[37825342234](https://github.com/dakotamurphyucf/gpuio/actions/runs/37825342234)
at `1f360479` failed only the dates/colors walkthrough's final exact composer
assertion. The old assertion did not print the actual value. This failure stays
recorded; its cause is not established by a later local pass.

At local source `17424381`, rebuilding the debug chat executable and running the
unchanged walkthrough with read-value logging passed. The immediate read after
the initial AX write returned `''`; the final read returned the full expected
`Civil dates and colors preserve my draft λ`. This demonstrates asynchronous
observation, not a reproduced application data-loss defect.

The candidate harness now checks the exact draft after the initial write and
after closing the inspector. It polls read-only for up to eight seconds, with
the existing overall walkthrough deadline still enforced. It never retypes or
repairs a mismatching draft. A persistent mismatch raises an error containing
both the expected and actual text. No application or native runtime code changed.

Validation commands:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/agent_chat/main.exe
python3 scratch/agents/root-20261008-preview-feedback/diagnose_dates.py
python3 scripts/test_agent_chat_dates_colors.py
python3 scratch/agents/root-20261008-preview-feedback/check_draft_wait.py
python3 scripts/audit_example_docs.py
git diff --check
```

Both local native walkthroughs pass on macOS, including the exact final text and
nested-picker window cleanup. The focused fake-clock check accepts a delayed
exact value and rejects persistent wrong text; it has no write operation.
The example inventory remains 432 sources / 268 reviewed groups. This does not
establish the hosted failure's root cause or qualify performance. The changed
harness passes hosted validation in run `37848090787`; C2/R2 remain open for the
three separate failures recorded in the [CI follow-up](milestone-07-ci.md).

The [retained archive](foundation-37825342234-och17/reports.tar.gz) and
[manifest](foundation-37825342234-och17/manifest.json) contain the original failed
log, terminal job metadata, local before/after logs, diagnostic/check scripts,
binary/test hashes, hosted raw timing reports and receiver reports. Scratch paths
above are reproducibility provenance; the archived scripts preserve their exact
contents and are not build dependencies.
