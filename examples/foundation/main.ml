open Core
module Upstream_bonsai = Bonsai

module Bonsai = struct
  include Upstream_bonsai.Cont
  module Effect = Upstream_bonsai.Effect

  let state' initial graph =
    state_machine0
      ~default_model:initial
      ~apply_action:(fun _ model update -> update model)
      graph
  ;;
end

module Effect = Bonsai.Effect

let log fmt = Printf.ksprintf (fun s -> Eio.traceln "%s" s) fmt
let i64 = Int64.of_int
let lifecycle_activations = ref 0
let lifecycle_deactivations = ref 0
let ignored_events = ref 0
let async_requests = ref 0
let async_completions = ref 0
let pending_async = ref 0
let input_revision = ref 0L
let extra_ops = ref []
let start_async = ref (fun () -> ())

type node =
  { id : int
  ; kind : int
  ; text : string
  ; click : unit Effect.t option
  ; children : node list
  }

let node ?(children = []) ?click id kind text = { id; kind; text; click; children }
let button id text click = node id 2 text ~click
let text id text = node id 1 text
let box id children = node id 0 "" ~children

type output =
  { tree : node
  ; set_text : string -> unit Effect.t
  ; set_status : string -> unit Effect.t
  }

let component graph =
  let open Bonsai.Let_syntax in
  let count, change_count = Bonsai.state' 0 graph in
  let order, change_order = Bonsai.state' [ 1; 2; 3 ] graph in
  let next_id = ref 3 in
  let content, set_text = Bonsai.state "" graph in
  let status, set_status = Bonsai.state "Idle" graph in
  let row_map =
    let%arr order = order in
    Int.Map.of_alist_exn (List.map order ~f:(fun id -> id, ()))
  in
  let rows =
    Bonsai.assoc
      (module Int)
      row_map
      ~f:(fun key _ graph ->
        let clicks, change = Bonsai.state' 0 graph in
        let activate =
          let%arr key = key in
          Effect.of_thunk (fun () ->
            incr lifecycle_activations;
            log "row_activate=%d" key)
        in
        let deactivate =
          let%arr key = key in
          Effect.of_thunk (fun () ->
            incr lifecycle_deactivations;
            log "row_deactivate=%d" key)
        in
        if not (Array.exists (Sys.get_argv ()) ~f:(String.equal "--no-lifecycle"))
        then Bonsai.Edge.lifecycle ~on_activate:activate ~on_deactivate:deactivate graph;
        let%arr key = key
        and clicks = clicks
        and change = change in
        button
          (100 + key)
          (Printf.sprintf "Row %d · clicks %d" key clicks)
          (change Int.succ))
      graph
  in
  let%arr count = count
  and change_count = change_count
  and order = order
  and change_order = change_order
  and rows = rows
  and content = content
  and set_text = set_text
  and status = status
  and set_status = set_status in
  { tree =
      box
        1
        [ text 2 "GPUIO · Native Bonsai experiment"
        ; text 3 "OCaml owns application state · Rust owns GPUI and text editing"
        ; text 4 (Printf.sprintf "Counter: %d" count)
        ; button 10 "Increment counter" (change_count Int.succ)
        ; box
            5
            [ button 11 "Reverse keyed rows" (change_order List.rev)
            ; button
                12
                "Remove row 3 / newest row"
                (change_order (fun rows ->
                   List.filter rows ~f:(fun id -> id <> List.fold rows ~init:0 ~f:Int.max)))
            ; button
                15
                "Add keyed row"
                (Effect.bind
                   (Effect.of_thunk (fun () ->
                      incr next_id;
                      !next_id))
                   ~f:(fun id -> change_order (fun rows -> rows @ [ id ])))
            ]
        ; box 20 (List.map order ~f:(fun id -> Map.find_exn rows id))
        ; node 30 3 ""
        ; text 31 ("OCaml text: " ^ content)
        ; button
            14
            "Clear native text"
            (Effect.of_thunk (fun () ->
               extra_ops := [ Wire.Edit (30L, !input_revision, "") ]))
        ; button
            13
            "Run Eio operation (250 ms)"
            (Effect.of_thunk (fun () -> !start_async ()))
        ; text 40 ("Eio: " ^ status)
        ]
  ; set_text
  ; set_status
  }
;;

let mounted_component mounted graph =
  let open Bonsai.Let_syntax in
  let active =
    let%arr mounted = mounted in
    if mounted then Int.Map.singleton 0 () else Int.Map.empty
  in
  let children =
    Bonsai.assoc (module Int) active ~f:(fun _ _ graph -> component graph) graph
  in
  let%arr children = children in
  Option.value
    (Map.find children 0)
    ~default:
      { tree = box 1 []
      ; set_text = (fun _ -> Effect.Ignore)
      ; set_status = (fun _ -> Effect.Ignore)
      }
;;

let flatten tree =
  let rec walk map node =
    List.fold node.children ~init:(Map.set map ~key:node.id ~data:node) ~f:walk
  in
  walk Int.Map.empty tree
;;

let ids node = List.map node.children ~f:(fun n -> n.id)
let handler node = Option.map node.click ~f:(fun _ -> i64 node.id)

let same_props a b =
  a.kind = b.kind
  && String.equal a.text b.text
  && Option.equal Int64.equal (handler a) (handler b)
;;

let diff old current =
  let upserts =
    Map.fold current ~init:[] ~f:(fun ~key ~data:n ops ->
      if
        Option.value_map (Map.find old key) ~default:false ~f:(fun old ->
          same_props old n)
      then ops
      else Wire.Upsert (i64 key, i64 n.kind, n.text, handler n) :: ops)
    |> List.rev
  in
  let children =
    Map.fold current ~init:[] ~f:(fun ~key ~data:n ops ->
      if
        Option.value_map
          (Map.find old key)
          ~default:(List.is_empty n.children)
          ~f:(fun old -> List.equal Int.equal (ids old) (ids n))
      then ops
      else Wire.Children (i64 key, List.map (ids n) ~f:i64) :: ops)
    |> List.rev
  in
  let removed =
    Map.keys old
    |> List.filter ~f:(fun key -> not (Map.mem current key))
    |> List.map ~f:(fun id -> Wire.Remove (i64 id))
  in
  upserts @ children @ removed @ if Map.is_empty old then [ Wire.Root 1L ] else []
;;

let worker notification_read ~self_test =
  Eio_main.run (fun env ->
    Eio.Switch.run (fun sw ->
      let clock = Bonsai.Time_source.create ~start:Time_ns.epoch in
      let mounted = Bonsai.Expert.Var.create true in
      let silent_active = Bonsai.Expert.Var.create false in
      let silent_starts = ref 0
      and silent_stops = ref 0 in
      let with_silent_lifecycle graph =
        let open Bonsai.Let_syntax in
        let keys =
          let%arr active = Bonsai.Expert.Var.value silent_active in
          if active then Int.Map.singleton 0 () else Int.Map.empty
        in
        let _ =
          Bonsai.assoc
            (module Int)
            keys
            ~f:(fun _ _ graph ->
              Bonsai.Edge.lifecycle
                ~on_activate:
                  (Bonsai.return (Effect.of_thunk (fun () -> incr silent_starts)))
                ~on_deactivate:
                  (Bonsai.return (Effect.of_thunk (fun () -> incr silent_stops)))
                graph;
              Bonsai.return ())
            graph
        in
        mounted_component (Bonsai.Expert.Var.value mounted) graph
      in
      let driver = Bonsai_driver.create ~clock with_silent_lifecycle in
      let stop_p, stop_r = Eio.Promise.create () in
      let start = Eio.Time.Mono.now (Eio.Stdenv.mono_clock env) in
      let now () =
        Mtime.Span.to_float_ns
          (Mtime.span start (Eio.Time.Mono.now (Eio.Stdenv.mono_clock env)))
        /. 1e9
      in
      let committed = ref Int.Map.empty in
      let revision = ref 0L in
      let inflight = ref None in
      let awaiting_display = ref false in
      let ready = ref false
      and stopped = ref false in
      let last_frame = ref (-1L) in
      let ack_count = ref 0
      and batch_bytes = ref 0
      and max_ops = ref 0
      and samples = ref [] in
      let input_seen = ref ""
      and ime_passed = ref false
      and stale_edit_passed = ref false
      and snapshot = ref "" in
      let rec pump () =
        if !ready && (not !stopped) && Option.is_none !inflight && not !awaiting_display
        then (
          Bonsai.Time_source.advance_clock
            clock
            ~to_:(Time_ns.add Time_ns.epoch (Time_ns.Span.of_sec (now ())));
          Bonsai_driver.flush driver;
          let result = Bonsai_driver.result driver in
          let current = flatten result.tree in
          let ops = diff !committed current @ !extra_ops in
          extra_ops := [];
          if not (List.is_empty ops)
          then (
            let next = Int64.succ !revision in
            let bytes = Wire.encode { version = 1; base = !revision; next; ops } in
            let error = Native.submit bytes |> Bytes.to_string in
            if not (String.is_empty error) then failwith error;
            batch_bytes := !batch_bytes + Bytes.length bytes;
            max_ops := Int.max !max_ops (List.length ops);
            inflight := Some (next, current, now ());
            awaiting_display := true;
            log
              "submit revision=%Ld ops=%d bytes=%d"
              next
              (List.length ops)
              (Bytes.length bytes))
          else (
            (* A logical display cycle can change lifecycles and callbacks even when
             the native tree is identical. Effects are flushed on the next tick. *)
            committed := current;
            Bonsai_driver.trigger_lifecycles driver))
      and schedule_effect event =
        Bonsai_driver.schedule_event driver event;
        pump ()
      in
      (start_async := fun () -> incr pending_async);
      let launch_pending () =
        while !pending_async > 0 do
          decr pending_async;
          incr async_requests;
          let request_number = !async_requests in
          Eio.Fiber.fork ~sw (fun () ->
            Eio.Fiber.first
              (fun () ->
                 schedule_effect ((Bonsai_driver.result driver).set_status "Working…");
                 Eio.Time.sleep
                   (Eio.Stdenv.clock env)
                   (if self_test && request_number = 2 then 60. else 0.25);
                 (* Real asynchronous file IO in the research workspace. *)
                 let payload =
                   Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / "async-message.txt")
                 in
                 incr async_completions;
                 if not !stopped
                 then
                   schedule_effect
                     ((Bonsai_driver.result driver).set_status (String.strip payload)))
              (fun () -> Eio.Promise.await stop_p))
        done
      in
      let handle = function
        | Wire.Ready ->
          ready := true;
          pump ()
        | Applied (next, nodes, ops) ->
          (match !inflight with
           | Some (expected, current, sent) when Int64.equal expected next ->
             revision := next;
             committed := current;
             inflight := None;
             incr ack_count;
             samples := ((now () -. sent) *. 1000.) :: !samples;
             log "applied revision=%Ld nodes=%Ld ops=%Ld" next nodes ops;
             ()
           | _ -> failwith "unexpected acknowledgement")
        | Click (id, handler_id, _) ->
          (match Map.find !committed (Int64.to_int_exn id) with
           | Some node when Int64.equal handler_id id ->
             Option.iter node.click ~f:schedule_effect
           | _ ->
             incr ignored_events;
             log "ignored_stale_handler=%Ld" handler_id)
        | Text (_, edit_revision, content, _) ->
          input_revision := edit_revision;
          input_seen := content;
          schedule_effect ((Bonsai_driver.result driver).set_text content)
        | Probe value ->
          log "probe=%s" value;
          if String.equal value "ime_utf16_selection_passed" then ime_passed := true;
          if String.is_prefix value ~prefix:"stale_edit_rejected:"
          then stale_edit_passed := true;
          if String.is_prefix value ~prefix:"snapshot:" then snapshot := value
        | Frame frame ->
          last_frame := frame;
          log "frame=%Ld" frame;
          if !awaiting_display && Option.is_none !inflight && Int64.(frame >= !revision)
          then (
            awaiting_display := false;
            Bonsai_driver.trigger_lifecycles driver)
        | Error e -> failwith ("Native: " ^ e)
        | Closed ->
          stopped := true;
          Eio.Promise.resolve stop_r ()
      in
      let get_text id =
        Option.value_map (Map.find !committed id) ~default:"" ~f:(fun n -> n.text)
      in
      let wait_for label condition =
        let deadline = now () +. 15. in
        while not (condition ()) do
          if !stopped || Float.(now () > deadline) then failwith ("timeout: " ^ label);
          Eio.Time.sleep (Eio.Stdenv.clock env) 0.01
        done
      in
      if self_test
      then
        Eio.Fiber.fork ~sw (fun () ->
          wait_for "initial commit" (fun () -> !ack_count >= 1);
          wait_for "initial lifecycle" (fun () -> not !awaiting_display);
          let commits_before = !ack_count in
          Bonsai.Expert.Var.set silent_active true;
          pump ();
          wait_for "lifecycle activation without tree diff" (fun () -> !silent_starts = 1);
          Bonsai.Expert.Var.set silent_active false;
          pump ();
          wait_for "lifecycle deactivation without tree diff" (fun () ->
            !silent_stops = 1);
          assert (!ack_count = commits_before);
          log "unchanged_tree_lifecycles_passed=true";
          let click id condition =
            wait_for "native frame" (fun () ->
              Int64.(!last_frame >= !revision) && Option.is_none !inflight);
            Native.probe id;
            wait_for ("click " ^ Int.to_string id) condition
          in
          click 10 (fun () -> String.equal (get_text 4) "Counter: 1");
          click 102 (fun () -> String.equal (get_text 102) "Row 2 · clicks 1");
          click 11 (fun () ->
            Option.value_map (Map.find !committed 20) ~default:false ~f:(fun n ->
              List.equal Int.equal (ids n) [ 103; 102; 101 ]));
          assert (String.equal (get_text 102) "Row 2 · clicks 1");
          click 12 (fun () -> not (Map.mem !committed 103));
          Native.probe (-3);
          wait_for "stale handler rejected" (fun () -> !ignored_events = 1);
          Native.probe (-1);
          wait_for "IME composition" (fun () ->
            !ime_passed
            && String.equal !input_seen "A日本語Z"
            && String.equal (get_text 31) "OCaml text: A日本語Z");
          extra_ops := [ Wire.Edit (30L, 0L, "stale overwrite") ];
          pump ();
          wait_for "stale native edit rejected" (fun () -> !stale_edit_passed);
          assert (String.equal !input_seen "A日本語Z");
          click 13 (fun () ->
            !async_completions = 1
            && String.equal (get_text 40) "Eio: Completed file read");
          (* Exercise native identity, closure disposal, Bonsai lifecycles and GC. *)
          for cycle = 1 to 20 do
            let id = 103 + cycle in
            click 15 (fun () -> Map.mem !committed id);
            click 12 (fun () -> not (Map.mem !committed id));
            Gc.full_major ()
          done;
          wait_for "quiescent" (fun () ->
            Option.is_none !inflight && not !awaiting_display);
          if not (Array.exists (Sys.get_argv ()) ~f:(String.equal "--no-lifecycle"))
          then (
            assert (!lifecycle_activations = 23);
            assert (!lifecycle_deactivations = 21));
          let panic_caught = Result.is_error (Result.try_with Native.test_panic) in
          assert panic_caught;
          log "rust_panic_contained=true";
          let malformed =
            Native.submit (Bytes.of_string "\001\000\001\000\000") |> Bytes.to_string
          in
          assert (String.is_substring malformed ~substring:"trailing");
          Native.probe (-2);
          wait_for "native snapshot" (fun () -> not (String.is_empty !snapshot));
          let sorted = List.sort !samples ~compare:Float.compare in
          let median = List.nth_exn sorted (List.length sorted / 2) in
          log
            "SELF_TEST_PASS commits=%d total_bytes=%d max_ops=%d median_ack_ms=%.3f \
             activations=%d deactivations=%d stale_events=%d async=%d"
            !ack_count
            !batch_bytes
            !max_ops
            median
            !lifecycle_activations
            !lifecycle_deactivations
            !ignored_events
            !async_completions;
          if Array.exists (Sys.get_argv ()) ~f:(String.equal "--hold")
          then Eio.Time.sleep (Eio.Stdenv.clock env) 5.;
          click 13 (fun () -> !async_requests = 2);
          wait_for "in-flight async operation" (fun () ->
            String.equal (get_text 40) "Eio: Working…");
          Native.quit ());
      (* Drive time and lifecycle-generated actions even without native input or a tree diff.
       Cancellation is tied to the host's existing stop promise. *)
      Eio.Fiber.fork ~sw (fun () ->
        Eio.Fiber.first
          (fun () ->
             while not !stopped do
               Eio.Time.sleep (Eio.Stdenv.clock env) (1. /. 60.);
               if not !stopped
               then (
                 pump ();
                 launch_pending ())
             done)
          (fun () -> Eio.Promise.await stop_p));
      let buffer = Cstruct.create 4096 in
      while not !stopped do
        ignore (Eio.Flow.single_read notification_read buffer : int);
        List.iter (Wire.decode (Native.drain ())) ~f:handle;
        launch_pending ()
      done;
      Bonsai.Expert.Var.set mounted false;
      Bonsai_driver.flush driver;
      Bonsai_driver.trigger_lifecycles driver;
      committed := Int.Map.empty;
      inflight := None;
      Bonsai_driver.Expert.invalidate_observers driver;
      assert (!lifecycle_activations = !lifecycle_deactivations);
      log
        "worker_shutdown_complete activations=%d deactivations=%d"
        !lifecycle_activations
        !lifecycle_deactivations))
;;

let run env ~self_test =
  Eio.Switch.run (fun sw ->
    let notification_read, notification_write = Eio_unix.pipe sw in
    Eio_unix.Fd.use_exn
      "initialize native bridge"
      (Eio_unix.Resource.fd notification_write)
      Native.init;
    Eio.Flow.close notification_write;
    let worker_domain =
      Domain.spawn (fun () ->
        try worker notification_read ~self_test with
        | exn ->
          let backtrace = Stdlib.Printexc.get_raw_backtrace () in
          Native.quit ();
          Stdlib.Printexc.raise_with_backtrace exn backtrace)
    in
    let native_result = Result.try_with Native.run in
    let worker_result = Result.try_with (fun () -> Domain.join worker_domain) in
    Native.cleanup ();
    Result.ok_exn native_result;
    Result.ok_exn worker_result;
    if self_test
    then (
      assert (!async_requests = 2 && !async_completions = 1);
      log "inflight_eio_cancelled=true");
    Eio.Flow.copy_string "shutdown_complete\n" (Eio.Stdenv.stdout env))
;;

let () =
  Stdlib.Printexc.record_backtrace true;
  let self_test = Array.exists (Sys.get_argv ()) ~f:(String.equal "--self-test") in
  Eio_main.run (fun env ->
    if Array.exists (Sys.get_argv ()) ~f:(String.equal "--two-windows")
    then Native.run_two_windows ()
    else run env ~self_test)
;;
