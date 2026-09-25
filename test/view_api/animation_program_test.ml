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
