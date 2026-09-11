<!-- Imported from Linear 910e89ba-2d1c-43a9-a8af-04eae001c8fb on 2026-09-11.
Historical paths and evidence are references, never build inputs. -->

Current validated baseline: stock OCaml 5.3, Bonsai v0.17 and Dune 3.24.2 with native source selection and identifier edits. Actual runtime evidence is macOS-only. No lifecycle/Incremental runtime patch. Inventory correction: installed-packages.json records an opam invocation, not package versions; the archive includes a separately labelled publication-time inventory.

Published to GPUIO on 2026-09-10. [Download the source/evidence bundle](<https://uploads.linear.app/698151a6-07bd-4043-9a7b-15f84b8c23da/0a5f6f70-b343-4265-a61f-2aa405aeb710/0c24fc23-9d1e-4772-bf2b-9c2d5d9f2a05?signature=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJwYXRoIjoiLzY5ODE1MWE2LTA3YmQtNDA0My05YTdiLTE1Zjg0YjhjMjNkYS8wYTVmNmY3MC1iMzQzLTQyNjUtYTYxZi0yYWE0MDVhZWI3MTAvMGMyNGZjMjMtOWQxZS00NzcyLWJmMmItOWMyZDVkOWYyYTA1IiwiaWF0IjoxNzg5MTU1MTE2LCJleHAiOjE3ODkxNTU0MTZ9.mmZ2kuxAPBWsrqBY-72efCBNFvcgO48mY0S8fwUdtnQ>) for complete experiment sources, patches, original logs and file checksums. Historical local paths identify archive files; they are not setup instructions for a new machine.

## native-v017/README.md

# GPUIO: Bonsai v0.17 native baseline — 2026-09-10

The native experiment passes with stock OCaml 5.3.0, Bonsai v0.17.0, Dune 3.24.2 and Eio 1.3. There is no OxCaml dependency and no Bonsai lifecycle/Incremental runtime patch. This replaces the preview as the selected experimental baseline.

## Run and build

```sh
cd /Users/dakotamurphy/gpuio-research-2026-09-10/native-v017
./build.sh
./_build/default/lifecycle_check.exe
./run.sh --self-test
./run.sh
```

The build reads the compiler and installed libraries from the existing `/Users/dakotamurphy/.opam` switch `default`. It invokes the Dune 3.24.2 executable already present in the separate research root (`../opam-root/spike/bin/dune`). No package was installed, removed, pinned or upgraded in the existing switch; no default switch was changed. All Dune output is in this external workspace. Build concurrency is limited to four jobs.

The Rust static archive is a fixed copy from the successful original native spike, built for the same stock OCaml 5.3 runtime. Its SHA-256 is recorded in `evidence/manifest.json`. Rust code did not change in this migration; its source, locked Cargo dependencies, build recipe and earlier Rust unit-test results remain in `../native-spike`. Rebuilding Rust on another machine requires that recipe and the pinned research sources. This directory is a research build, not yet a standalone distributable package.

## Native library selection and small fork

Eight Jane Street v0.17 source packages are vendored here. `evidence/manifest.json` records their commits. Dune directory selection includes:

* Bonsai core, native driver and its PPX;
* virtual_dom.ui_effect;
* incr_dom.ui_incr and incr_dom.ui_time_source;
* Incremental and its step-function dependency, incr_map, incr_select, abstract_algebra and ppx_pattern_bind.

CSS generation, browser renderer, web components, web protocols and upstream test packages are outside the selected build. Dune 3.24.2 builds the selected libraries successfully. This does not make the original full `opam install bonsai` package compatible with Dune 3.24: its web build and broad package dependencies still exist. A distributable native package will need explicit packaging/dependency metadata.

Five OCaml source files have lexical renames of the reserved identifier `effect` to `ui_effect_value`: four in Bonsai and one in ui_effect. Some exposed labels/type aliases change; this is an experimental source compatibility fork, not a claim of identical API spelling. No lifecycle implementation or Incremental implementation is changed. Other changes are Dune directory selection. Per-package diffs are under `evidence/*-native-v017.patch`. The build uses the release profile; the initial dev-profile build also encountered OCaml 5.3 warning 67 in an upstream functor signature.

## Lifecycle contract and host corrections

The v0.17 driver documents `flush -> result -> trigger_lifecycles`. It runs deactivations, activations, then after-display effects. Effects can queue actions that need another flush. `Bonsai.assoc` retains keyed state across deactivation/reactivation; removal from the native tree does not imply that Bonsai forgets the model.

The native host now:

1. Flushes actions and reads the result before reconciling.
2. For a changed native tree, waits for the GPUI render callback acknowledging that revision before triggering lifecycle effects. Receipt/application of a batch alone is insufficient.
3. For an unchanged tree, still refreshes the callback registry and triggers lifecycle effects for the logical display cycle.
4. Uses a cancellable 60 Hz Eio tick to advance time and process lifecycle-generated actions even without native input or a tree diff.
5. Deactivates the mounted computation before invalidating driver observers and shutting down.

The periodic tick is an experimental scheduler. A future host can replace idle polling with demand-driven wakeups/deadlines. Its no-change cycle is a logical display cycle; GPUI's render callback is not a measured physical screen-presentation acknowledgement.

The previous host only triggered lifecycles after batches with tree mutations, which could miss invisible lifecycle transitions. That is an integration defect corrected here. It does not explain the preview's earlier crash during graph initialization; that cause remains unestablished. We do not infer an upstream Bonsai defect from that earlier experiment.

## Validation

`evidence/lifecycle-check.log`: standalone tests pass with optimization enabled and disabled, and on a worker domain. Checks cover keyed activation/deactivation, unchanged visible output, state retention after reactivation, action flush after activation, deactivation-before-activation ordering, repeated after-display callbacks, GC during key replacement, and explicit cleanup before observer invalidation.

`evidence/native-self-test-final.log`: real GPUI integration passes, including:

* invisible activation/deactivation with the native commit count unchanged;
* counter and keyed state preserved through reorder;
* row removal, stale callback rejection and twenty add/remove cycles with major GC;
* native UTF-16 selection/composition callback checks and stale edit rejection;
* Eio file read and cancellation of a second in-flight operation;
* Rust panic containment and malformed-message rejection;
* 23 row activations balanced by 23 deactivations at shutdown.

The test reports 50 commits before the second async operation, 1,525 serialized bytes and a maximum of 22 operations in a batch. Timing in the log is diagnostic only, not a benchmark. Text/IME checks exercise GPUI input-handler callbacks; they do not automate the operating system's IME.

`evidence/link-audit.json`: the final executable has 0 `camlAsync` symbols, with Bonsai and Eio symbols present. This native build does not need an Async scheduler. Installed-package metadata, source commits, patch hashes and archive provenance are recorded alongside the logs. The complete upstream Bonsai test suite was not run.

## Decision

Use stock OCaml and Bonsai v0.17 for the initial experimental release. Defer OxCaml. Keep the small native packaging/identifier fork explicit and reproducible. Do not carry forward the preview's lifecycle workaround. <issue id="45d6f53a-e819-4545-abf0-16087d587500" href="https://linear.app/ochat/issue/OCH-5/gpui-research">OCH-5</issue> remains open; implementation backlog creation still follows agreement on the overall design.

Primary source references: `vendor/bonsai/src/driver/bonsai_driver.mli`, `vendor/bonsai/src/driver/bonsai_driver.ml`, `vendor/bonsai/src/lifecycle.ml`, `vendor/bonsai/docs/how_to/lifecycles.md`, and `vendor/bonsai/test/driver.ml`.

---

## native-v017/evidence/manifest.json

```json
{
  "compiler": "stock OCaml 5.3.0",
  "dune": "3.24.2",
  "switch_root": "/Users/dakotamurphy/.opam",
  "switch": "default",
  "switch_mutations": false,
  "sources": {
    "bonsai": {
      "tag": "v0.17.0",
      "commit": "e929674585a67818734b06e12ea328872d185970"
    },
    "virtual_dom": {
      "tag": "v0.17.0",
      "commit": "e2c80cea41db6484825b5d4b487a042ef058d591"
    },
    "incr_dom": {
      "tag": "v0.17.0",
      "commit": "36459c50998a21c52affdebcc5e83f19c5312360"
    },
    "incremental": {
      "tag": "v0.17.0",
      "commit": "61baea591be9bfaf512bb2fcdf176753f67b2607"
    },
    "incr_map": {
      "tag": "v0.17.0",
      "commit": "5e41d2551023c6ca4b62ce9118637d26c6fe6f4f"
    },
    "incr_select": {
      "tag": "v0.17.0",
      "commit": "d44df84aff82062dce34a88333fb29e7a311d66c"
    },
    "ppx_pattern_bind": {
      "tag": "v0.17.0",
      "commit": "cd6d4da39d4d1023241e503689d98f8547868c31"
    },
    "abstract_algebra": {
      "tag": "v0.17.0",
      "commit": "a210611d14f385e11889d1df3aabd2cf34060ed8"
    }
  },
  "patches": {
    "bonsai": "9a50a092034f757060a70734e2d968294aada41f51d89679b2a3c2a9fbf399b3",
    "virtual_dom": "d4910dd3277b95ae940a639174b71e163ed42b5d5356779148559634b299f1dd",
    "incr_dom": "4ea70525774d21e3d36606044ac218796e5efdf2df44d4e84ba918ceaaf7a591",
    "incremental": "1825e78054f09eb7f36a2f5d4ebb66e8e658052ae1bb7abe3b2d204837dbad5b",
    "incr_map": "07e13404b8c2dda9774dd61d670cb149cf6b2deadfaa3bd4969c6830c9e6097b",
    "incr_select": "07e13404b8c2dda9774dd61d670cb149cf6b2deadfaa3bd4969c6830c9e6097b",
    "ppx_pattern_bind": "87196e4e6b2d89bde99a1c58883180e990b1aff708175e36d713da6f8a4b419a",
    "abstract_algebra": "07e13404b8c2dda9774dd61d670cb149cf6b2deadfaa3bd4969c6830c9e6097b"
  },
  "rust_archive_reused_from": "../native-spike/libgpuio_spike.a",
  "rust_archive_sha256": "d8b38e7c85b89400e711fd569b0532fe226305d8a9eaf01724e7d365f1611305"
}
```

---

## native-v017/evidence/lifecycle-check.log

```text
LIFECYCLE_PASS optimize=true state-retention reactivation unchanged-view effect-flush ordering cleanup gc
LIFECYCLE_PASS optimize=false state-retention reactivation unchanged-view effect-flush ordering cleanup gc
LIFECYCLE_PASS optimize=true state-retention reactivation unchanged-view effect-flush ordering cleanup gc
```

---

## native-v017/evidence/native-self-test-final.log

```text
frame=0
submit revision=1 ops=22 bytes=425
applied revision=1 nodes=18 ops=22
frame=1
row_activate=1
row_activate=2
row_activate=3
unchanged_tree_lifecycles_passed=true
submit revision=2 ops=1 bytes=19
applied revision=2 nodes=18 ops=1
frame=2
submit revision=3 ops=1 bytes=27
applied revision=3 nodes=18 ops=1
frame=3
submit revision=4 ops=1 bytes=10
applied revision=4 nodes=18 ops=1
frame=4
submit revision=5 ops=2 bytes=11
applied revision=5 nodes=17 ops=2
frame=5
row_deactivate=3
ignored_stale_handler=103
submit revision=6 ops=1 bytes=27
probe=ime_utf16_selection_passed
applied revision=6 nodes=17 ops=1
frame=6
submit revision=7 ops=1 bytes=32
applied revision=7 nodes=17 ops=1
frame=7
submit revision=8 ops=1 bytes=23
probe=stale_edit_rejected:0:3
applied revision=8 nodes=17 ops=1
frame=8
submit revision=9 ops=1 bytes=24
applied revision=9 nodes=17 ops=1
frame=9
submit revision=10 ops=1 bytes=33
applied revision=10 nodes=17 ops=1
frame=10
submit revision=11 ops=2 bytes=33
applied revision=11 nodes=18 ops=2
frame=11
row_activate=4
submit revision=12 ops=2 bytes=11
applied revision=12 nodes=17 ops=2
frame=12
row_deactivate=4
submit revision=13 ops=2 bytes=33
applied revision=13 nodes=18 ops=2
frame=13
row_activate=5
submit revision=14 ops=2 bytes=11
applied revision=14 nodes=17 ops=2
frame=14
row_deactivate=5
submit revision=15 ops=2 bytes=33
applied revision=15 nodes=18 ops=2
frame=15
row_activate=6
submit revision=16 ops=2 bytes=11
applied revision=16 nodes=17 ops=2
frame=16
row_deactivate=6
submit revision=17 ops=2 bytes=33
applied revision=17 nodes=18 ops=2
frame=17
row_activate=7
submit revision=18 ops=2 bytes=11
applied revision=18 nodes=17 ops=2
frame=18
row_deactivate=7
submit revision=19 ops=2 bytes=33
applied revision=19 nodes=18 ops=2
frame=19
row_activate=8
submit revision=20 ops=2 bytes=11
applied revision=20 nodes=17 ops=2
frame=20
row_deactivate=8
submit revision=21 ops=2 bytes=33
applied revision=21 nodes=18 ops=2
frame=21
row_activate=9
submit revision=22 ops=2 bytes=11
applied revision=22 nodes=17 ops=2
frame=22
row_deactivate=9
submit revision=23 ops=2 bytes=34
applied revision=23 nodes=18 ops=2
frame=23
row_activate=10
submit revision=24 ops=2 bytes=11
applied revision=24 nodes=17 ops=2
frame=24
row_deactivate=10
submit revision=25 ops=2 bytes=34
applied revision=25 nodes=18 ops=2
frame=25
row_activate=11
submit revision=26 ops=2 bytes=11
applied revision=26 nodes=17 ops=2
frame=26
row_deactivate=11
submit revision=27 ops=2 bytes=34
applied revision=27 nodes=18 ops=2
frame=27
row_activate=12
submit revision=28 ops=2 bytes=11
applied revision=28 nodes=17 ops=2
frame=28
row_deactivate=12
submit revision=29 ops=2 bytes=34
applied revision=29 nodes=18 ops=2
frame=29
row_activate=13
submit revision=30 ops=2 bytes=11
applied revision=30 nodes=17 ops=2
frame=30
row_deactivate=13
submit revision=31 ops=2 bytes=34
applied revision=31 nodes=18 ops=2
frame=31
row_activate=14
submit revision=32 ops=2 bytes=11
applied revision=32 nodes=17 ops=2
frame=32
row_deactivate=14
submit revision=33 ops=2 bytes=34
applied revision=33 nodes=18 ops=2
frame=33
row_activate=15
submit revision=34 ops=2 bytes=11
applied revision=34 nodes=17 ops=2
frame=34
row_deactivate=15
submit revision=35 ops=2 bytes=34
applied revision=35 nodes=18 ops=2
frame=35
row_activate=16
submit revision=36 ops=2 bytes=11
applied revision=36 nodes=17 ops=2
frame=36
row_deactivate=16
submit revision=37 ops=2 bytes=34
applied revision=37 nodes=18 ops=2
frame=37
row_activate=17
submit revision=38 ops=2 bytes=11
applied revision=38 nodes=17 ops=2
frame=38
row_deactivate=17
submit revision=39 ops=2 bytes=34
applied revision=39 nodes=18 ops=2
frame=39
row_activate=18
submit revision=40 ops=2 bytes=11
applied revision=40 nodes=17 ops=2
frame=40
row_deactivate=18
submit revision=41 ops=2 bytes=34
applied revision=41 nodes=18 ops=2
frame=41
row_activate=19
submit revision=42 ops=2 bytes=11
applied revision=42 nodes=17 ops=2
frame=42
row_deactivate=19
submit revision=43 ops=2 bytes=34
applied revision=43 nodes=18 ops=2
frame=43
row_activate=20
submit revision=44 ops=2 bytes=11
applied revision=44 nodes=17 ops=2
frame=44
row_deactivate=20
submit revision=45 ops=2 bytes=34
applied revision=45 nodes=18 ops=2
frame=45
row_activate=21
submit revision=46 ops=2 bytes=11
applied revision=46 nodes=17 ops=2
frame=46
row_deactivate=21
submit revision=47 ops=2 bytes=34
applied revision=47 nodes=18 ops=2
frame=47
row_activate=22
submit revision=48 ops=2 bytes=11
applied revision=48 nodes=17 ops=2
frame=48
row_deactivate=22
submit revision=49 ops=2 bytes=34
applied revision=49 nodes=18 ops=2
frame=49
row_activate=23
submit revision=50 ops=2 bytes=11
applied revision=50 nodes=17 ops=2
frame=50
row_deactivate=23

thread '<unnamed>' (262188217) panicked at src/bridge.rs:113:9:
intentional GPUIO boundary test
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
rust_panic_contained=true
probe=snapshot:50:17:|GPUIO · Native Bonsai experiment|OCaml owns application state · Rust owns GPUI and text editing|Counter: 1||Increment counter|Reverse keyed rows|Remove row 3 / newest row|Run Eio operation (250 ms)|Clear native text|Add keyed row|||OCaml text: A日本語Z|Eio: Completed file read|Row 1 · clicks 0|Row 2 · clicks 1
SELF_TEST_PASS commits=50 total_bytes=1525 max_ops=22 median_ack_ms=0.081 activations=23 deactivations=21 stale_events=1 async=1
submit revision=51 ops=1 bytes=24
applied revision=51 nodes=17 ops=1
frame=51
row_deactivate=1
row_deactivate=2
worker_shutdown_complete activations=23 deactivations=23
inflight_eio_cancelled=true
shutdown_complete
```

---

## native-v017/evidence/link-audit.json

```json
{
  "camlAsync": 0,
  "camlBonsai": 4715,
  "camlEio": 3560
}
```

---

## native-v017/lifecycle_check.ml

```ocaml
open Core
module B = Bonsai.Cont
module E = Bonsai.Effect

let run ~optimize () =
  let events = ref [] in
  let record name key = E.of_thunk (fun () -> events := !events @ [name,key]) in
  let keys = B.Expert.Var.create (Int.Map.of_alist_exn [1,(); 2,()]) in
  let component graph =
    let open B.Let_syntax in
    let rows = B.assoc (module Int) (B.Expert.Var.value keys) ~f:(fun key _ graph ->
      let count, bump = B.state_machine0 ~default_model:0
        ~apply_action:(fun _ model () -> model+1) graph in
      let activated, set_activated = B.state false graph in
      let on_activate = let%arr key and set_activated in
        E.Many [record "activate" key; set_activated true] in
      let on_deactivate = let%arr key in record "deactivate" key in
      let after_display = let%arr key in record "display" key in
      B.Edge.lifecycle ~on_activate ~on_deactivate ~after_display graph;
      let%arr count and bump and activated in count, bump, activated) graph in
    (* The visible output is deliberately unchanged by membership or state. *)
    let%arr rows in "unchanged native view", rows
  in
  let clock = Bonsai.Time_source.create ~start:Time_ns.epoch in
  let driver = Bonsai_driver.create ~optimize ~clock component in
  let cycle () =
    Bonsai_driver.flush driver;
    let view, rows = Bonsai_driver.result driver in
    assert (String.equal view "unchanged native view");
    Bonsai_driver.trigger_lifecycles driver;
    rows
  in
  let count_event name = List.count !events ~f:(fun (n,_) -> String.equal n name) in
  let rows = cycle () in
  assert (count_event "activate" = 2);
  assert (count_event "display" = 2);
  let _,_,activated = Map.find_exn rows 1 in assert (not activated);
  let rows = cycle () in
  let _,bump,activated = Map.find_exn rows 1 in assert activated;
  assert (count_event "activate" = 2);
  Bonsai_driver.schedule_event driver (bump ());
  let rows = cycle () in
  let count,_,_ = Map.find_exn rows 1 in assert (count=1);
  B.Expert.Var.set keys (Int.Map.singleton 2 ());
  ignore (cycle ());
  assert (count_event "deactivate" = 1);
  B.Expert.Var.set keys (Int.Map.of_alist_exn [2,();1,()]);
  let rows = cycle () in
  let count,_,_ = Map.find_exn rows 1 in assert (count=1);
  assert (count_event "activate" = 3);
  events := [];
  B.Expert.Var.set keys (Int.Map.singleton 3 ());
  ignore (cycle ());
  assert (List.equal (fun (a,b) (c,d) -> String.equal a c && Int.equal b d)
    !events ["deactivate",1;"deactivate",2;"activate",3;"display",3]);
  for key=4 to 23 do
    B.Expert.Var.set keys (Int.Map.singleton key ());
    ignore (cycle ());
    Gc.full_major ()
  done;
  B.Expert.Var.set keys Int.Map.empty;
  assert (Map.is_empty (cycle ()));
  Bonsai_driver.Expert.invalidate_observers driver;
  printf "LIFECYCLE_PASS optimize=%b state-retention reactivation unchanged-view effect-flush ordering cleanup gc\n%!" optimize

let () =
  Stdlib.Printexc.record_backtrace true;
  run ~optimize:true ();
  run ~optimize:false ();
  Domain.join (Domain.spawn (fun () -> run ~optimize:true ()))
```

