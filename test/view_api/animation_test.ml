open Core
open Gpuio

let target values = Animation.Target.create values |> Or_error.ok_exn

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
