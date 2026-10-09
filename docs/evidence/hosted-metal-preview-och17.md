# Hosted Metal timing availability for the preview

The owner’s developer-preview scope separates hosted timing availability from
required physical workload qualification. This change implements that distinction
for the two small macOS CI probes. It does not change the renderer, the physical
workload collectors, their budgets, or required macOS/Linux correctness tests.

## Classification contract

`scripts/metal_timing_availability.py` distinguishes:

| Result | Required evidence | CI treatment |
| --- | --- | --- |
| Passed | Complete positive timing records and the probe's normal accounting, visibility and cleanup assertions | Success; scoped probe qualification only |
| Unavailable | Every expected callback is present, **all** presentation timestamps are zero, and all other required accounting/visibility/cleanup checks succeed | Success with an explicit warning and `availability.json` saying `unavailable`; no timing acceptance |
| Failed | Crash, timeout, absent/malformed/duplicate JSON fields, incomplete workload, missing callback, invalid clock, mixed zero/positive timing, or failed ownership/cleanup | Failure, preserving raw output |

The GPUI probe must complete at least 90 admitted records in each of two distinct
windows/sessions, stop admission, close both windows, settle pending callbacks to
zero, and retain every trace record. Loss, saturation, duplicate callbacks, invalid
clocks, truncated traces and histogram overflow are rejected. The normal positive
case still needs its animation-sample minimum. A wholly zero-time report has no
positive timing or animation samples; it is explicitly unavailable.

The standalone API probe must complete all 120 distinct frame records and GPU
completions. Schema 2 records window closure and release of its timer/layer/queue
references after performing cleanup. The unavailable case permits only the exact
clock-order failures caused by 120 zero presentation timestamps. Other errors and
unexpected process exits are not converted. Older API reports without cleanup
evidence do not satisfy this new contract.

`test_metal_presentation.py` remains strict by default. Foundation opts into
`--allow-unavailable-timing`; native subprocess failures are still raised before
classification. The API wrapper bounds the child to 25 seconds, preserving its
stdout report and stderr log. Python's `subprocess.run` kills and reaps a timed-out
child. Both probes retain their original positive qualification result in the raw
report rather than rewriting it to pass.

A final CI step revalidates both raw reports against their classifications. A
zero-time GPUI hook **fails** when the independent API probe has valid positive
timestamps: that discrepancy cannot be attributed to general hosted timing
unavailability. Missing output from either probe also fails. A positive hook with
an unavailable API probe remains explicitly unqualified as a combined calibration.

## Local validation checkpoint

The nine portable classifier tests pass, including mutations of closure,
pending work, callback accounting, sequence/identity, clocks and input fields;
mixed timing; malformed/non-finite JSON; and child crash/timeout handling.
Replaying the archived hosted GPUI all-zero report yields `unavailable`. Replaying
the older standalone API report fails because it lacks schema-2 cleanup evidence.
This replay is not a fresh hosted/native run.

The unchanged physical presentation report suite passes its thirteen tests.
Actionlint and `git diff --check` pass. Commands:

```sh
python3 scripts/test_metal_timing_availability.py
python3 scripts/test_presentation_report.py
actionlint .github/workflows/foundation.yml
git diff --check
```

## Fresh local negative cases

After the paired list comparison completed and its children were reaped, the
standalone Swift probe compiled with `xcrun swiftc -O`. The new API wrapper ran
once. All 120 GPU completions arrived, all four cleanup fields are true, and the
process exited 1: the first presentation timestamp is zero and the remaining 119
are positive. The wrapper correctly rejects this mixed result. This is a failed
calibration, not an unavailable-clock or successful timing result.

The GPUI wrapper then built and ran once. Both windows complete, stop admission,
close and settle to zero pending callbacks. Window 1 has 90 valid presentations.
Window 2 has 92 admitted records: 90 presented, one zero and one invalid-clock
outcome, with two input-bearing records. The report does not identify the input
producer. The wrapper correctly rejects the invalid-clock/input accounting;
it does not classify this as unavailable. No replacement attempt was run.

These actual failures exercise rejection and cleanup reporting, supplementing the
portable positive/all-zero/malformed cases. They do not establish fresh positive
short-probe calibration. The final pair check has no successful classifier outputs
to combine; its positive/unavailable/discrepancy paths are covered by portable
tests. Hosted candidate execution remains required before accepting this CI change.
The [physical list batch](preview-list-comparison-och17.md) is separate accepted
workload evidence. The raw [local records](hosted-metal-preview-och17/reports.tar.gz)
and [manifest](hosted-metal-preview-och17/manifest.json) preserve these failures.
This record does not close R2 or qualify the release.
