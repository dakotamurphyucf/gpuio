open Core
open Gpuio

let target values = Animation.Target.create values |> Or_error.ok_exn

let%expect_test "stepped easing validates counts and pins all position tags" =
  let module A = Animation in
  let module W = Gpuio_protocol.Wire.Animation in
  let positions =
    [ "start", A.Easing.Step_position.Jump_start
    ; "end", A.Easing.Step_position.Jump_end
    ; "none", A.Easing.Step_position.Jump_none
    ; "both", A.Easing.Step_position.Jump_both
    ]
  in
  let encoded =
    List.map positions ~f:(fun (name, position) ->
      List.iter [ Int.min_value; -1; 0; 4_294_967_296; Int.max_value ] ~f:(fun count ->
        assert (Result.is_error (A.Easing.steps ~count ~position)));
      assert (Result.is_ok (A.Easing.steps ~count:4_294_967_295 ~position));
      let easing = A.Easing.steps ~count:4 ~position |> Or_error.ok_exn in
      let bytes =
        Bin_prot.Utils.bin_dump [%bin_writer: W.Easing.t] (A.Expert.easing_to_wire easing)
        |> Bigstring.to_string
      in
      name
      ^ "\t"
      ^ String.concat_map bytes ~f:(fun byte -> sprintf "%02x" (Char.to_int byte)))
    |> String.concat ~sep:"\n"
  in
  assert (Result.is_error (A.Easing.steps ~count:1 ~position:Jump_none));
  List.iter [ A.Easing.Step_position.Jump_start; Jump_end; Jump_both ] ~f:(fun position ->
    assert (Result.is_ok (A.Easing.steps ~count:1 ~position)));
  Eio_main.run (fun env ->
    let fixture =
      Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / "animation-steps.tsv") |> String.strip
    in
    assert (String.equal encoded fixture));
  print_endline "four step positions: validated counts; independent wire bytes";
  [%expect {| four step positions: validated counts; independent wire bytes |}]
;;

let%expect_test "polynomial easing presets use the independent native curve fixtures" =
  let module A = Animation in
  let module W = Gpuio_protocol.Wire.Animation in
  let curves =
    [ "in", A.Easing.ease_in_cubic
    ; "out", A.Easing.ease_out_cubic
    ; "in-out", A.Easing.ease_in_out_cubic
    ]
  in
  assert (not (A.Easing.equal A.Easing.ease_in_cubic A.Easing.ease_in));
  assert (not (A.Easing.equal A.Easing.ease_out_cubic A.Easing.ease_out));
  assert (not (A.Easing.equal A.Easing.ease_in_out_cubic A.Easing.ease_in_out));
  let encoded =
    List.map curves ~f:(fun (name, easing) ->
      let bytes =
        Bin_prot.Utils.bin_dump [%bin_writer: W.Easing.t] (A.Expert.easing_to_wire easing)
        |> Bigstring.to_string
      in
      let hex =
        String.concat_map bytes ~f:(fun byte -> sprintf "%02x" (Char.to_int byte))
      in
      name ^ "\t" ^ hex)
    |> String.concat ~sep:"\n"
  in
  Eio_main.run (fun env ->
    let fixture =
      Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / "animation-cubic-easing.tsv")
      |> String.strip
    in
    assert (String.equal encoded fixture));
  print_endline "three polynomial presets: shared native bytes; distinct from CSS presets";
  [%expect {| three polynomial presets: shared native bytes; distinct from CSS presets |}]
;;

let%expect_test "animation targets validate geometry and expand radius canonically" =
  let rejected values = assert (Result.is_error (Animation.Target.create values)) in
  rejected [];
  rejected [ Width, -1. ];
  rejected [ Opacity, 1.1 ];
  rejected [ Top, Float.nan ];
  rejected [ Height, Float.infinity ];
  rejected [ Radius, 2.; Top_left_radius, 3. ];
  rejected [ Width, 1.; Width, 2. ];
  let a = target [ Radius, 4.; Left, -20.; Opacity, 0.5 ] in
  let b =
    target
      [ Opacity, 0.5
      ; Left, -20.
      ; Bottom_right_radius, 4.
      ; Top_left_radius, 4.
      ; Bottom_left_radius, 4.
      ; Top_right_radius, 4.
      ]
  in
  assert (Animation.Target.equal a b);
  print_endline "bounded targets; radius expanded; order does not change identity";
  [%expect {| bounded targets; radius expanded; order does not change identity |}]
;;

let%expect_test "timing and initial range are validated before wire construction" =
  let initial = target [ Width, 0. ] in
  let target = target [ Width, 240. ] in
  let invalid result = assert (Result.is_error result) in
  invalid
    (Animation.Config.create
       ~initial:(Animation.Target.create [ Height, 0. ] |> Or_error.ok_exn)
       ~target
       ());
  invalid (Animation.Config.create ~duration:(Time_ns.Span.of_ms (-1.)) ~target ());
  invalid (Animation.Config.create ~delay:(Time_ns.Span.of_day 2.) ~target ());
  invalid (Animation.Config.create ~repeat:Loop ~target ());
  invalid
    (Animation.Config.create
       ~initial
       ~repeat:Alternate
       ~duration:Time_ns.Span.zero
       ~target
       ());
  invalid (Animation.Easing.cubic_bezier ~x1:(-0.1) ~y1:0. ~x2:1. ~y2:1.);
  invalid (Animation.Easing.cubic_bezier ~x1:0. ~y1:Float.nan ~x2:1. ~y2:1.);
  let easing =
    Animation.Easing.cubic_bezier ~x1:0. ~y1:(-1.) ~x2:1. ~y2:2. |> Or_error.ok_exn
  in
  let config =
    Animation.Config.create
      ~initial
      ~target
      ~easing
      ~repeat:Alternate
      ~duration:(Time_ns.Span.of_ms 0.1)
      ()
    |> Or_error.ok_exn
  in
  invalid (Animation.Expert.to_wire config ~generation:0L);
  let wire = Animation.Expert.to_wire config ~generation:42L |> Or_error.ok_exn in
  assert (Int64.equal wire.duration_ms 1L);
  assert (Int64.equal wire.delay_ms 0L);
  assert (Int64.equal wire.generation 42L);
  assert (Gpuio_protocol.Wire.Animation.Repeat.equal wire.repeat Alternate);
  print_endline
    "matched initial range; bounded timing/easing; positive generation; sub-ms rounds up";
  [%expect
    {| matched initial range; bounded timing/easing; positive generation; sub-ms rounds up |}]
;;

let%expect_test "animation configuration matches the independent native fixture" =
  let module W = Gpuio_protocol.Wire.Animation in
  let initial = target [ Width, 0.; Opacity, 0. ] in
  let target = target [ Width, 240.; Opacity, 0.5 ] in
  let easing =
    Animation.Easing.cubic_bezier ~x1:0.25 ~y1:0. ~x2:0.75 ~y2:1. |> Or_error.ok_exn
  in
  let config =
    Animation.Config.create
      ~initial
      ~target
      ~easing
      ~repeat:Alternate
      ~duration:(Time_ns.Span.of_ms 100.)
      ~delay:(Time_ns.Span.of_ms 10.)
      ()
    |> Or_error.ok_exn
  in
  let wire = Animation.Expert.to_wire config ~generation:42L |> Or_error.ok_exn in
  let encoded =
    Bin_prot.Utils.bin_dump [%bin_writer: W.Config.t] wire |> Bigstring.to_string
  in
  let hex =
    String.concat_map encoded ~f:(fun byte -> sprintf "%02x" (Char.to_int byte))
  in
  Eio_main.run (fun env ->
    let fixture =
      Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / "animation-v1-config.hex")
      |> String.strip
    in
    assert (String.equal hex fixture));
  let pos_ref = ref 0 in
  let decoded = W.Config.bin_read_t (Bigstring.of_string encoded) ~pos_ref in
  assert (W.Config.equal decoded wire);
  assert (!pos_ref = String.length encoded);
  print_endline "canonical config: OCaml and Rust encoding, full OCaml decode";
  [%expect {| canonical config: OCaml and Rust encoding, full OCaml decode |}]
;;

let%expect_test "retained animation generations and ordered endpoint delivery" =
  let module W = Gpuio_protocol.Wire in
  let module Id = Gpuio_protocol.Node_id in
  let window =
    Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn
  in
  let reconciler = Reconciler.create window in
  let config width =
    Animation.Config.create
      ~initial:(target [ Width, 0. ])
      ~target:(target [ Width, width ])
      ()
    |> Or_error.ok_exn
  in
  let view label config =
    View.animate config [] ~on_event:(fun event ->
      label, Animation.Run_id.to_int64 event.run_id, event.outcome)
  in
  let prepare view =
    Reconciler.prepare reconciler ~theme:Theme.default (Some view) |> Or_error.ok_exn
  in
  let first = prepare (view "first" (config 100.)) in
  let id, handler =
    match Reconciler.message first with
    | Some (Apply tx) ->
      List.find_map_exn tx.operations ~f:(function
        | Create (id, Animated, _, Some handler) -> Some (id, handler)
        | _ -> None)
    | _ -> assert false
  in
  Reconciler.accept reconciler first |> Or_error.ok_exn;
  let unchanged = prepare (view "refreshed" (config 100.)) in
  assert (Option.is_none (Reconciler.message unchanged));
  Reconciler.accept reconciler unchanged |> Or_error.ok_exn;
  let second = prepare (view "latest" (config 200.)) in
  let revision =
    match Reconciler.message second with
    | Some (Apply tx) ->
      assert (
        List.exists tx.operations ~f:(function
          | Set_animation (found, config) ->
            Id.equal id found && Int64.equal config.generation 2L
          | _ -> false));
      assert (
        List.for_all tx.operations ~f:(function
          | Create _ | Bind _ | Remove _ -> false
          | _ -> true));
      tx.revision
    | _ -> assert false
  in
  Reconciler.accept reconciler second |> Or_error.ok_exn;
  let event generation outcome =
    W.Event.Animation_endpoint (window, id, handler, revision, { generation; outcome })
  in
  let cancelled = event 1L (Cancelled Replaced) in
  let label, run, outcome =
    Reconciler.dispatch reconciler cancelled |> Option.value_exn
  in
  assert (String.equal label "latest" && Int64.equal run 1L);
  assert (Animation.Outcome.equal outcome (Cancelled Replaced));
  assert (Option.is_none (Reconciler.dispatch reconciler cancelled));
  let finished = event 2L Finished in
  let _, run, outcome = Reconciler.dispatch reconciler finished |> Option.value_exn in
  assert (Int64.equal run 2L && Animation.Outcome.equal outcome Finished);
  assert (Option.is_none (Reconciler.dispatch reconciler finished));
  assert (Option.is_none (Reconciler.dispatch reconciler (event 3L Finished)));
  let replacement = prepare (View.text "replacement") in
  Reconciler.accept reconciler replacement |> Or_error.ok_exn;
  assert (Option.is_none (Reconciler.dispatch reconciler finished));
  print_endline
    "stable node/handler, latest closure, prior-run cancellation, once-only endpoints, \
     disposal";
  [%expect
    {| stable node/handler, latest closure, prior-run cancellation, once-only endpoints, disposal |}]
;;

let%expect_test "application motion preference wire tags" =
  List.iter
    [ Gpuio_protocol.Wire.Animation.Preference.System; Reduce; Full ]
    ~f:(fun preference ->
      let bytes =
        Gpuio_protocol.Wire.Message.encode (Set_motion preference) |> Or_error.ok_exn
      in
      print_s [%sexp (String.to_list bytes |> List.map ~f:Char.to_int : int list)]);
  [%expect
    {|
    (9 0)
    (9 1)
    (9 2) |}]
;;

let%expect_test "spring parameters validate before native admission" =
  let invalid result = assert (Result.is_error result) in
  let create ?epsilon ?max_duration ~stiffness ~damping ~mass () =
    Animation.Spring.create ?epsilon ?max_duration ~stiffness ~damping ~mass ()
  in
  List.iter [ Float.nan; Float.infinity; Float.neg_infinity; -1. ] ~f:(fun value ->
    invalid (create ~stiffness:value ~damping:10. ~mass:1. ());
    invalid (create ~stiffness:100. ~damping:value ~mass:1. ());
    invalid (create ~stiffness:100. ~damping:10. ~mass:value ());
    invalid (create ~epsilon:value ~stiffness:100. ~damping:10. ~mass:1. ()));
  List.iter
    [ Time_ns.Span.zero; Time_ns.Span.of_ns (-1.); Time_ns.Span.of_sec 61. ]
    ~f:(fun max_duration ->
      invalid (create ~max_duration ~stiffness:100. ~damping:10. ~mass:1. ()));
  invalid (create ~stiffness:10_001. ~damping:10. ~mass:1. ());
  invalid (create ~stiffness:100. ~damping:1_001. ~mass:1. ());
  invalid (create ~stiffness:100. ~damping:10. ~mass:1_001. ());
  invalid (create ~epsilon:1.01 ~stiffness:100. ~damping:10. ~mass:1. ());
  let lower =
    create
      ~epsilon:0.0001
      ~max_duration:(Time_ns.Span.of_ns 1.)
      ~stiffness:0.01
      ~damping:0.
      ~mass:0.01
      ()
    |> Or_error.ok_exn
    |> Animation.Expert.spring_to_wire
  in
  assert (Int64.equal lower.max_duration_ms 1L);
  ignore
    (create
       ~epsilon:1.
       ~max_duration:(Time_ns.Span.of_sec 60.)
       ~stiffness:10_000.
       ~damping:1_000.
       ~mass:1_000.
       ()
     |> Or_error.ok_exn
     : Animation.Spring.t);
  print_endline
    "finite physical parameters; admitted endpoints; undamped allowed; bounded deadline";
  [%expect
    {| finite physical parameters; admitted endpoints; undamped allowed; bounded deadline |}]
;;

let%expect_test "spring parameters share an independent Rust wire fixture" =
  let module W = Gpuio_protocol.Wire.Animation.Spring in
  let parameters =
    Animation.Spring.create ~stiffness:100. ~damping:10. ~mass:1. ()
    |> Or_error.ok_exn
    |> Animation.Expert.spring_to_wire
  in
  let bytes = Bin_prot.Utils.bin_dump [%bin_writer: W.t] parameters in
  let hex =
    Bigstring.to_string bytes
    |> String.concat_map ~f:(fun byte -> sprintf "%02x" (Char.to_int byte))
  in
  Eio_main.run (fun env ->
    let expected =
      Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / "animation-spring.hex") |> String.strip
    in
    assert (String.equal expected hex));
  let pos_ref = ref 0 in
  assert (W.equal parameters (W.bin_read_t bytes ~pos_ref));
  assert (Int.equal !pos_ref (Bigstring.length bytes));
  print_endline hex;
  [%expect {| 00000000000059400000000000002440000000000000f03ffca9f1d24d62503ffe1027 |}]
;;

let%expect_test "linear easing resolves positions and shares independent native bytes" =
  let module E = Animation.Easing in
  let stop ?input output = E.Linear_stop.create ?input ~output () |> Or_error.ok_exn in
  List.iter [ Float.nan; Float.infinity; Float.neg_infinity ] ~f:(fun output ->
    assert (Result.is_error (E.Linear_stop.create ~output ())));
  List.iter [ Float.nan; Float.infinity; -0.1; 1.1 ] ~f:(fun input ->
    assert (Result.is_error (E.Linear_stop.create ~input ~output:0. ())));
  List.iter
    [ []
    ; [ stop 0. ]
    ; List.init 257 ~f:(fun _ -> stop 0.)
    ; [ stop ~input:0.8 0.; stop ~input:0.2 1. ]
    ]
    ~f:(fun stops -> assert (Result.is_error (E.linear_stops stops)));
  assert (Result.is_ok (E.linear_stops (List.init 256 ~f:(fun _ -> stop 0.))));
  let resolved =
    E.linear_stops [ stop 0.; stop ~input:0.25 1.; stop 2.; stop ~input:0.75 3.; stop 4. ]
    |> Or_error.ok_exn
    |> Animation.Expert.easing_to_wire
  in
  assert (
    Gpuio_protocol.Wire.Animation.Easing.equal
      resolved
      (Linear_stops [ 0., 0.; 0.25, 1.; 0.5, 2.; 0.75, 3.; 1., 4. ]));
  let collapsed =
    E.linear_stops [ stop ~input:0.5 0.; stop 1.; stop ~input:0.5 2. ]
    |> Or_error.ok_exn
    |> Animation.Expert.easing_to_wire
  in
  assert (
    Gpuio_protocol.Wire.Animation.Easing.equal
      collapsed
      (Linear_stops [ 0.5, 0.; 0.5, 1.; 0.5, 2. ]));
  let fixtures =
    [ "inferred", [ stop 0.; stop 0.75; stop 0.25; stop 1. ]
    ; ( "jump"
      , [ stop 0.
        ; stop ~input:0.25 0.
        ; stop ~input:0.25 0.75
        ; stop ~input:0.75 0.75
        ; stop 1.
        ] )
    ; "endpoints", [ stop ~input:0.25 (-0.25); stop ~input:0.75 1.25 ]
    ]
  in
  let encoded =
    List.map fixtures ~f:(fun (name, stops) ->
      let easing = E.linear_stops stops |> Or_error.ok_exn in
      let bytes =
        Bin_prot.Utils.bin_dump
          [%bin_writer: Gpuio_protocol.Wire.Animation.Easing.t]
          (Animation.Expert.easing_to_wire easing)
        |> Bigstring.to_string
      in
      name
      ^ "\t"
      ^ String.concat_map bytes ~f:(fun byte -> sprintf "%02x" (Char.to_int byte)))
    |> String.concat ~sep:"\n"
  in
  Eio_main.run (fun env ->
    let expected =
      Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / "animation-linear-stops.tsv")
      |> String.strip
    in
    assert (String.equal encoded expected));
  print_endline
    "inferred positions; duplicate jumps; held endpoints; invalid inputs and count bounds";
  [%expect
    {| inferred positions; duplicate jumps; held endpoints; invalid inputs and count bounds |}]
;;

let%expect_test "large easing curves respect the complete animation program byte budget" =
  let module A = Animation in
  let easing =
    A.Easing.linear_stops
      (List.init 256 ~f:(fun index ->
         A.Easing.Linear_stop.create ~output:(Float.of_int index /. 255.) ()
         |> Or_error.ok_exn))
    |> Or_error.ok_exn
  in
  let timing = A.Timing.tween ~easing (Time_ns.Span.of_sec 1.) |> Or_error.ok_exn in
  let initial = target [ Width, 0. ] in
  let stage =
    A.Stage.create ~target:(target [ Width, 100. ]) ~timing () |> Or_error.ok_exn
  in
  assert (Result.is_ok (A.Program.create ~initial [ stage; stage; stage ]));
  assert (Result.is_error (A.Program.create ~initial [ stage; stage; stage; stage ]));
  print_endline
    "256-stop curves supported; oversized multi-stage programs rejected before transport";
  [%expect
    {| 256-stop curves supported; oversized multi-stage programs rejected before transport |}]
;;
