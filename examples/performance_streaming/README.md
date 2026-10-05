# Concurrent streaming and typing qualification

OCH-17 qualification driver. The native four-second smoke passes content,
typing, source restoration and teardown, with 40 input-to-submitted-frame samples
for 40 dispatched keys after the optional native-text profiler repair. All three full
optimized runs now pass the declared budgets; see the
[raw reports and scoped evidence](../../docs/evidence/streaming-typing-och17.md). The
model's expect test and collector rejection fixtures pass. Use the existing
optimized paired performance backend and stock Core/Bonsai/Eio environment.

Four independent Eio producers each schedule 128-byte UTF-8 fragments at absolute
50 ms deadlines for 120 seconds: 2,400 fragments per stream, 1,228,800 bytes in
total. The managed history groups sixteen fragments into a 2 KiB block, yielding
600 retained history blocks. Existing blocks use collection point updates; new
blocks append with stable keys. This represents a segmented streamed transcript,
not four monolithic rich-Markdown documents. Rich/source document performance is
qualified separately. The history follows the tail and admits at most 32 active
rows, with 200 logical pixels of overscan and measured variable heights.

The external driver must deliver 1,200 native `asdf` key events at 10 Hz to the
labelled multiline composer while all four producers run. It must preserve the
owner's input-source selection and inspect the final native text independently
of the application's controller snapshot. Neither AXValue replacement nor an
editor replacement command counts as typing. Per-second observations record all
four stream counts and the observed composer length; final validation checks
every retained history block against the deterministic source. After closing the
timing interval, each stream's final block is revealed for an external canonical
AX content check. All application work/resource counters must retire.

Every actual UI update timestamp is retained relative to the common application
start, separately from native frame histograms and the collector's key dispatch
times. Absolute scheduling prevents accumulated sleep drift; late updates are
recorded, never silently dropped. The native input histogram includes dispatched
input events and can coalesce multiple events into a submitted frame. Retain its
counts and dropped-event count. These are native submission measurements, not
hardware key latency, GPU completion or physical presentation.

The four-second smoke uses 80 fragments per stream and 40 keys. Full acceptance
requires an explicit warm-up followed by three optimized runs: draw p95 ≤16.7 ms,
p99 ≤33.4 ms; input→submitted-frame p95 ≤50 ms, p99 ≤100 ms; at least 1,000 draw
and relevant input samples; peak RSS ≤1 GiB; exact text and cleanup. Record pacing
and duration separately so a delayed catch-up burst is not called 20 Hz streaming.
Before measured runs, pacing also requires every UI update to arrive within two
nominal update periods (100 ms) of its absolute deadline, and each key dispatch
within half a key period (50 ms). These test-driver tolerances are separate from
input→submitted-frame latency. The collector polls its owned log every 5 ms;
that process's overhead is outside application CPU/RSS totals.
See the [predeclared plan](../../docs/design/performance-qualification.md).
