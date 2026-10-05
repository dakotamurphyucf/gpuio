open Core
open Gpuio
module A = Animation
module W = Gpuio_protocol.Wire.Animation_program

let ok = Or_error.ok_exn
let target width opacity = A.Target.create [ Width, width; Opacity, opacity ] |> ok
let initial = target 0. 0.

let timed ?(delay = 0.) duration target =
  A.Stage.create
    ~delay:(Time_ns.Span.of_ms delay)
    ~timing:(A.Timing.tween ~easing:A.Easing.ease_out (Time_ns.Span.of_ms duration) |> ok)
    ~target
    ()
  |> ok
;;

let spring = A.Spring.create ~stiffness:100. ~damping:10. ~mass:1. () |> ok
let wire program = A.Expert.program_to_wire program ~generation:42L |> ok

let%expect_test "mixed sequence matches independent Rust fixture and controls are pure" =
  let stages =
    [ timed ~delay:10. 100. (target 120. 0.5)
    ; A.Stage.create
        ~delay:(Time_ns.Span.of_ms 20.)
        ~timing:(A.Timing.spring spring)
        ~target:(target 240. 1.)
        ()
      |> ok
    ]
  in
  let base = A.Program.create ~initial ~delay:(Time_ns.Span.of_ms 30.) stages |> ok in
  let restarted = A.Program.restart base |> ok |> A.Program.restart |> ok in
  let configured = A.Program.with_playback restarted Paused |> wire in
  assert (Int64.equal configured.restart 2L);
  assert (Int64.equal (wire base).restart 0L);
  assert (W.Program.equal configured.program (wire base).program);
  let bytes = Bin_prot.Utils.bin_dump [%bin_writer: W.Config.t] configured in
  let hex =
    Bigstring.to_string bytes
    |> String.concat_map ~f:(fun byte -> sprintf "%02x" (Char.to_int byte))
  in
  Eio_main.run (fun env ->
    let expected =
      Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / "animation-program.hex")
      |> String.strip
    in
    assert (String.equal hex expected));
  let pos_ref = ref 0 in
  assert (W.Config.equal configured (W.Config.bin_read_t bytes ~pos_ref));
  assert (Int.equal !pos_ref (Bigstring.length bytes));
  let restored =
    A.Program.with_playback restarted Cancelled |> A.Program.restart |> ok |> wire
  in
  assert (W.Playback.equal restored.playback Running);
  assert (Int64.equal restored.restart 3L);
  assert (Result.is_error (A.Expert.program_to_wire base ~generation:0L));
  print_endline
    "independent fixture; complete decode; immutable playback; explicit restart";
  [%expect
    {| independent fixture; complete decode; immutable playback; explicit restart |}]
;;

let%expect_test "sequences validate property sets, counts and total cycle time" =
  let invalid result = assert (Result.is_error result) in
  let stage = timed 100. (target 100. 1.) in
  invalid (A.Program.create []);
  invalid (A.Program.create ~initial (List.init 33 ~f:(fun _ -> stage)));
  ignore
    (A.Program.create ~initial (List.init 32 ~f:(fun _ -> stage)) |> ok : A.Program.t);
  invalid (A.Program.create [ stage; stage ]);
  invalid (A.Program.create ~initial:(A.Target.create [ Height, 0. ] |> ok) [ stage ]);
  let mismatch =
    A.Stage.create
      ~timing:(A.Timing.spring spring)
      ~target:(A.Target.create [ Height, 100. ] |> ok)
      ()
    |> ok
  in
  invalid (A.Program.create ~initial [ stage; mismatch ]);
  invalid (A.Timing.tween (Time_ns.Span.of_ns (-1.)));
  invalid
    (A.Stage.create
       ~delay:(Time_ns.Span.of_day 2.)
       ~timing:(A.Timing.spring spring)
       ~target:(target 100. 1.)
       ());
  invalid (A.Program.create ~initial [ timed 86_400_000. (target 100. 1.); stage ]);
  ignore
    (A.Program.create
       ~initial
       ~delay:(Time_ns.Span.of_day 1.)
       [ timed 86_400_000. (target 100. 1.) ]
     |> ok
     : A.Program.t);
  let fractional = timed 0.01 (target 100. 1.) in
  (match (wire (A.Program.create [ fractional ] |> ok)).program.stages with
   | [ { timing = Tween (duration, _); _ } ] -> assert (Int64.equal duration 1L)
   | _ -> assert false);
  print_endline
    "1..32 matched stages; initial values; cycle and initial delay separately bounded";
  [%expect
    {| 1..32 matched stages; initial values; cycle and initial delay separately bounded |}]
;;

let%expect_test "shared clocks require timed repeats; group names are bounded UTF-8" =
  let invalid result = assert (Result.is_error result) in
  List.iter
    [ ""; "bad\000name"; "\255"; String.make 129 'x' ]
    ~f:(fun name -> invalid (A.Clock.group name));
  let clock = A.Clock.group "chat-pulse" |> ok in
  let stage = timed 100. (target 100. 1.) in
  ignore
    (A.Program.create ~initial ~repeat:Alternate ~clock [ stage ] |> ok : A.Program.t);
  ignore
    (A.Program.create ~initial ~repeat:Loop ~clock:A.Clock.application [ stage ] |> ok
     : A.Program.t);
  invalid (A.Program.create ~initial ~clock [ stage ]);
  invalid (A.Program.create ~repeat:Loop ~clock [ stage ]);
  invalid
    (A.Program.create
       ~initial
       ~repeat:Loop
       ~clock
       ~delay:(Time_ns.Span.of_ms 1.)
       [ stage ]);
  let physical =
    A.Stage.create ~timing:(A.Timing.spring spring) ~target:(target 100. 1.) () |> ok
  in
  invalid (A.Program.create ~initial ~repeat:Loop ~clock [ physical ]);
  ignore (A.Program.create ~initial ~repeat:Loop [ physical ] |> ok : A.Program.t);
  invalid (A.Program.create ~initial ~repeat:Loop [ timed 0. (target 100. 1.) ]);
  print_endline "bounded names; shared fixed periods; independent spring repeats";
  [%expect {| bounded names; shared fixed periods; independent spring repeats |}]
;;

let%expect_test "sequence reverse swaps endpoints and retains interval timing" =
  let a = timed ~delay:10. 100. (target 120. 0.5) in
  let b = timed ~delay:20. 200. (target 240. 1.) in
  let program = A.Program.create ~initial [ a; b ] |> ok in
  let reversed = A.Program.reverse program |> ok in
  assert (A.Program.equal program (A.Program.reverse reversed |> ok));
  assert (Result.is_error (A.Program.reverse (A.Program.create [ a ] |> ok)));
  let values = (wire reversed).program in
  print_s
    [%sexp (List.map values.stages ~f:(fun stage -> stage.W.Stage.delay_ms) : int64 list)];
  let first = List.hd_exn values.stages in
  print_s [%sexp (first.targets : Gpuio_protocol.Wire.Animation.Target.t list)];
  [%expect
    {|
    (20 10)
    (((property Width) (value 120)) ((property Opacity) (value 0.5)))
    |}]
;;

let%expect_test "retained program batches preserve run identity and reject stale delivery"
  =
  let module Wire = Gpuio_protocol.Wire in
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let reconciler = Reconciler.create window in
  let program =
    A.Program.create
      ~initial
      [ timed 100. (target 120. 0.5); timed 100. (target 240. 1.) ]
    |> ok
  in
  let view label config =
    View.animate_program config [] ~on_event:(fun event -> label, event)
  in
  let prepare v = Reconciler.prepare reconciler ~theme:Theme.default (Some v) |> ok in
  let first = prepare (view "first" program) in
  let id, handler =
    match Reconciler.message first with
    | Some (Apply tx) ->
      List.find_map_exn tx.operations ~f:(function
        | Create (id, Animation_program, _, Some handler) -> Some (id, handler)
        | _ -> None)
    | _ -> assert false
  in
  Reconciler.accept reconciler first |> ok;
  let unchanged = prepare (view "latest" program) in
  assert (Option.is_none (Reconciler.message unchanged));
  Reconciler.accept reconciler unchanged |> ok;
  let paused = prepare (view "paused" (A.Program.with_playback program Paused)) in
  let revision =
    match Reconciler.message paused with
    | Some (Apply tx) ->
      assert (
        List.exists tx.operations ~f:(function
          | Set_animation_program (_, c) -> Int64.equal c.generation 2L
          | _ -> false));
      assert (
        List.for_all tx.operations ~f:(function
          | Create _ | Remove _ | Bind _ -> false
          | _ -> true));
      tx.revision
    | _ -> assert false
  in
  Reconciler.accept reconciler paused |> ok;
  let signal generation index observation : W.Signal.t =
    { generation; index; observation }
  in
  let stage = signal 1L 1L (Stage_completed (0L, Played)) in
  let event signals =
    Wire.Event.Animation_program_event (window, id, handler, revision, signals)
  in
  let deliver signals = Reconciler.dispatch reconciler (event signals) in
  List.iter
    [ []
    ; [ stage; stage ]
    ; [ { stage with generation = 3L } ]
    ; [ { stage with index = 2L } ]
    ]
    ~f:(fun invalid -> assert (Option.is_none (deliver invalid)));
  let label, observed = deliver [ stage ] |> Option.value_exn in
  assert (String.equal label "paused");
  assert (Int64.equal (A.Run_id.to_int64 observed.run_id) 1L);
  print_s [%sexp (observed : A.Program.Event.t)];
  let terminal =
    [ stage; signal 1L 2L (Stage_completed (1L, Reduced_motion)); signal 1L 33L Finished ]
  in
  let _, observed = deliver terminal |> Option.value_exn in
  print_s [%sexp (observed : A.Program.Event.t)];
  assert (Option.is_none (deliver terminal));
  assert (Option.is_none (deliver [ stage ]));
  let restarted = prepare (view "restarted" (A.Program.restart program |> ok)) in
  Reconciler.accept reconciler restarted |> ok;
  let _, observed = deliver [ signal 3L 33L (Cancelled Requested) ] |> Option.value_exn in
  print_s [%sexp (observed : A.Program.Event.t)];
  let replacement = prepare (View.text "replacement") in
  Reconciler.accept reconciler replacement |> ok;
  assert (Option.is_none (deliver [ signal 3L 33L Finished ]));
  [%expect
    {|
    ((run_id 1) (observations ((Stage_completed 0 Played))))
    ((run_id 1) (observations ((Stage_completed 1 Reduced_motion) Finished)))
    ((run_id 3) (observations ((Cancelled Requested)))) |}]
;;

let%expect_test "program operation and bounded observations match independent fixtures" =
  let module Wire = Gpuio_protocol.Wire in
  let load name =
    Eio_main.run (fun env ->
      Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / name) |> String.strip)
  in
  let hex bytes = String.concat_map bytes ~f:(fun c -> sprintf "%02x" (Char.to_int c)) in
  let bytes hex =
    String.init
      (String.length hex / 2)
      ~f:(fun index ->
        Int.of_string ("0x" ^ String.sub hex ~pos:(index * 2) ~len:2) |> Char.of_int_exn)
  in
  let config =
    A.Program.create
      ~initial
      ~delay:(Time_ns.Span.of_ms 30.)
      [ timed ~delay:10. 100. (target 120. 0.5)
      ; A.Stage.create
          ~delay:(Time_ns.Span.of_ms 20.)
          ~timing:(A.Timing.spring spring)
          ~target:(target 240. 1.)
          ()
        |> ok
      ]
    |> ok
    |> A.Program.restart
    |> ok
    |> A.Program.restart
    |> ok
    |> fun program -> A.Program.with_playback program Paused |> wire
  in
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok in
  let handler = Gpuio_protocol.Handler_id.create ~slot:0L ~generation:1L |> ok in
  let request =
    Wire.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations =
          [ Create (node, Animation_program, "", Some handler)
          ; Set_animation_program (node, config)
          ; Set_root (Some node)
          ]
      }
  in
  assert (
    String.equal
      (Wire.Message.encode request |> ok |> hex)
      (load "animation-program-request.hex"));
  let encoded = load "animation-program-events.hex" |> bytes in
  let events = Wire.Event.decode encoded |> ok in
  (match events with
   | [ Animation_program_event (w, n, h, 1L, signals) ] ->
     assert (
       Gpuio_protocol.Window_id.equal w window
       && Gpuio_protocol.Node_id.equal n node
       && Gpuio_protocol.Handler_id.equal h handler);
     print_s [%sexp (A.Expert.program_event_of_wire signals |> ok : A.Program.Event.t)]
   | _ -> assert false);
  for length = 0 to String.length encoded - 1 do
    assert (Result.is_error (Wire.Event.decode (String.prefix encoded length)))
  done;
  (* Header is envelope count, event tag, three generational handles, revision.
     The next byte is the bounded batch count; reject it before reading signals. *)
  let oversized = String.prefix encoded 9 ^ String.of_char (Char.of_int_exn 34) in
  assert (Result.is_error (Wire.Event.decode oversized));
  let pos_ref = ref 0 in
  let rejected =
    Result.try_with (fun () ->
      W.Batch.bin_read_t
        (Bigstring.of_string (String.of_char (Char.of_int_exn 34)))
        ~pos_ref)
  in
  assert (
    match rejected with
    | Error W.Invalid_wire_batch -> !pos_ref = 1
    | Error _ | Ok _ -> false);
  let trailing = encoded ^ "\000" in
  assert (Result.is_error (Wire.Event.decode trailing));
  [%expect
    {|
    ((run_id 42)
     (observations
      ((Stage_completed 0 Played) (Stage_completed 1 Reduced_motion) Finished)))
    |}]
;;

let%expect_test "advanced programs negotiate a capability above 32 bits" =
  let module Wire = Gpuio_protocol.Wire in
  assert (Int64.equal (Int64.bit_and Wire.capabilities 4294967296L) 4294967296L);
  let bytes = Wire.Message.encode (Hello (Wire.version, Wire.capabilities)) |> ok in
  String.to_list bytes
  |> List.map ~f:(fun char -> sprintf "%02x" (Char.to_int char))
  |> String.concat
  |> print_endline;
  [%expect {| 0003fcffffffffffffff7f |}]
;;

let%expect_test
    "signed initial delays round away from zero and retain stage/shared bounds"
  =
  let stage = timed 100. (target 100. 1.) in
  List.iter
    [ -86_400_000., -86_400_000L
    ; -10., -10L
    ; -0.01, -1L
    ; 0., 0L
    ; 0.01, 1L
    ; 86_400_000., 86_400_000L
    ]
    ~f:(fun (delay, expected) ->
      let delay = Time_ns.Span.of_ms delay in
      let program = A.Program.create ~initial ~delay [ stage ] |> ok |> wire in
      assert (Int64.equal program.program.delay_ms expected);
      let config = A.Config.create ~initial ~delay ~target:(target 100. 1.) () |> ok in
      let config = A.Expert.to_wire config ~generation:42L |> ok in
      assert (Int64.equal config.delay_ms expected));
  List.iter [ -86_400_001.; 86_400_001. ] ~f:(fun delay ->
    let delay = Time_ns.Span.of_ms delay in
    assert (Result.is_error (A.Program.create ~initial ~delay [ stage ]));
    assert (Result.is_error (A.Config.create ~initial ~delay ~target:(target 100. 1.) ())));
  assert (
    Result.is_error
      (A.Stage.create
         ~delay:(Time_ns.Span.of_ms (-1.))
         ~timing:(A.Timing.tween (Time_ns.Span.of_ms 100.) |> ok)
         ~target:(target 100. 1.)
         ()));
  assert (
    Result.is_error
      (A.Program.create
         ~initial
         ~repeat:Loop
         ~clock:A.Clock.application
         ~delay:(Time_ns.Span.of_ms (-1.))
         [ stage ]));
  print_endline
    "signed one-day initial bounds; nonzero fractions round away; stage/shared \
     constraints retained";
  [%expect
    {| signed one-day initial bounds; nonzero fractions round away; stage/shared constraints retained |}]
;;

let%expect_test "negative initial-delay configs match independent native wire fixtures" =
  let easing = A.Easing.cubic_bezier ~x1:0.25 ~y1:0. ~x2:0.75 ~y2:1. |> ok in
  let config =
    A.Config.create
      ~initial
      ~target:(target 240. 0.5)
      ~easing
      ~duration:(Time_ns.Span.of_ms 100.)
      ~delay:(Time_ns.Span.of_ms (-10.))
      ~repeat:Alternate
      ()
    |> ok
    |> fun config -> A.Expert.to_wire config ~generation:42L |> ok
  in
  let program =
    A.Program.create
      ~initial
      ~delay:(Time_ns.Span.of_ms (-30.))
      [ timed ~delay:10. 100. (target 120. 0.5)
      ; A.Stage.create
          ~delay:(Time_ns.Span.of_ms 20.)
          ~timing:(A.Timing.spring spring)
          ~target:(target 240. 1.)
          ()
        |> ok
      ]
    |> ok
    |> A.Program.restart
    |> ok
    |> A.Program.restart
    |> ok
    |> fun program -> A.Program.with_playback program Paused |> wire
  in
  let hex writer value =
    Bin_prot.Utils.bin_dump writer value
    |> Bigstring.to_string
    |> String.concat_map ~f:(fun byte -> sprintf "%02x" (Char.to_int byte))
  in
  let encoded =
    "config\t"
    ^ hex [%bin_writer: Gpuio_protocol.Wire.Animation.Config.t] config
    ^ "\nprogram\t"
    ^ hex [%bin_writer: W.Config.t] program
  in
  Eio_main.run (fun env ->
    let fixture =
      Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / "animation-negative-delay.tsv")
      |> String.strip
    in
    assert (String.equal encoded fixture));
  print_endline "negative int64 delays preserve baseline and program field order";
  [%expect {| negative int64 delays preserve baseline and program field order |}]
;;

let%expect_test "changing initial delay preserves playback and monotonic restart identity"
  =
  let original =
    A.Program.create ~initial [ timed 100. (target 100. 1.) ]
    |> ok
    |> A.Program.restart
    |> ok
    |> fun program -> A.Program.with_playback program Paused
  in
  let changed = A.Program.with_initial_delay original (Time_ns.Span.of_ms (-25.)) |> ok in
  let encoded = wire changed in
  assert (Int64.equal encoded.program.delay_ms (-25L));
  assert (Int64.equal encoded.restart 1L);
  assert (W.Playback.equal encoded.playback Paused);
  let restarted = A.Program.restart changed |> ok |> wire in
  assert (Int64.equal restarted.restart 2L);
  assert (W.Playback.equal restarted.playback Running);
  assert (
    A.Program.equal original (A.Program.with_initial_delay changed Time_ns.Span.zero |> ok));
  let shared =
    A.Program.create
      ~initial
      ~repeat:Loop
      ~clock:A.Clock.application
      [ timed 100. (target 100. 1.) ]
    |> ok
  in
  assert (Result.is_error (A.Program.with_initial_delay shared (Time_ns.Span.of_ms (-1.))));
  assert (Result.is_error (A.Program.with_initial_delay original (Time_ns.Span.of_day 2.)));
  print_endline
    "delay edits preserve playback/restart; restart advances; shared and time bounds \
     enforced";
  [%expect
    {| delay edits preserve playback/restart; restart advances; shared and time bounds enforced |}]
;;
