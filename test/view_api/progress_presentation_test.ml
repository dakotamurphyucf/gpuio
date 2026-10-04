open Core
open Gpuio
module W = Gpuio_protocol.Progress_wire

let ok = Or_error.ok_exn

let encoded config =
  Bin_prot.Utils.bin_dump W.Presentation.bin_writer_t config
  |> Bigstring.to_string
  |> String.to_list
  |> List.map ~f:(fun char -> sprintf "%02x" (Char.to_int char))
  |> String.concat
;;

let fixture file config =
  Eio_main.run (fun env ->
    let expected = Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / file) |> String.strip in
    assert (String.equal (encoded config) expected))
;;

let%expect_test "progress presentation shares semantics and independent Rust bytes" =
  let linear =
    Progress.Config.create
      ~label:"Upload"
      ~value:(Progress.Value.determinate ~fraction:0.25 |> ok)
    |> ok
  in
  let linear =
    Progress.Expert.presentation_to_wire
      linear
      ~shape:Linear
      ~transition:Progress.Transition.immediate
  in
  fixture "progress-presentation-linear.hex" linear;
  let circle =
    Progress.Config.create ~label:"Indexing" ~value:Progress.Value.indeterminate |> ok
  in
  let circle =
    Progress.Expert.presentation_to_wire
      circle
      ~shape:Circle
      ~transition:(Progress.Transition.tween (Time_ns.Span.of_ms 200.) |> ok)
  in
  fixture "progress-presentation-circle.hex" circle;
  print_s [%sexp (linear : W.Presentation.t), (circle : W.Presentation.t)];
  [%expect
    {|
    (((progress ((label Upload) (fraction (0.25)))) (shape Linear)
      (transition Immediate))
     ((progress ((label Indexing) (fraction ()))) (shape Circle)
      (transition (Tween (duration_ms 200) (easing Ease_out)))))
    |}]
;;

let%expect_test "progress tween validates time and retains checked easing" =
  List.iter
    [ Time_ns.Span.min_value_representable
    ; Time_ns.Span.zero
    ; Time_ns.Span.of_ms 60000.001
    ; Time_ns.Span.max_value_representable
    ]
    ~f:(fun duration -> assert (Result.is_error (Progress.Transition.tween duration)));
  let config =
    Progress.Config.create ~label:"Downloading" ~value:Progress.Value.indeterminate |> ok
  in
  let easing = Animation.Easing.cubic_bezier ~x1:0.25 ~y1:(-2.) ~x2:0.75 ~y2:3. |> ok in
  List.iter
    [ 0.000001, 1; 200.5, 201; 60000., 60000 ]
    ~f:(fun (ms, expected) ->
      let transition = Progress.Transition.tween ~easing (Time_ns.Span.of_ms ms) |> ok in
      let wire = Progress.Expert.presentation_to_wire config ~shape:Circle ~transition in
      match wire.transition with
      | Tween { duration_ms; easing = actual } ->
        assert (duration_ms = expected);
        assert (
          Gpuio_protocol.Animation_wire.Easing.equal
            actual
            (Animation.Expert.easing_to_wire easing))
      | Immediate -> assert false);
  let default = Progress.Transition.tween (Time_ns.Span.of_ms 200.) |> ok in
  assert (
    Progress.Transition.equal
      default
      (Progress.Transition.tween
         ~easing:Animation.Easing.ease_out
         (Time_ns.Span.of_ms 200.)
       |> ok));
  assert (not (Progress.Transition.equal default Progress.Transition.immediate));
  print_endline "positive time, 60s bound, upward rounding and easing identity checked";
  [%expect {| positive time, 60s bound, upward rounding and easing identity checked |}]
;;

let%expect_test "existing progress payload bytes are unchanged" =
  let config =
    Progress.Config.create
      ~label:"Upload"
      ~value:(Progress.Value.determinate ~fraction:0.25 |> ok)
    |> ok
  in
  let legacy = Progress.Expert.to_wire config in
  let wire =
    Progress.Expert.presentation_to_wire
      config
      ~shape:Linear
      ~transition:Progress.Transition.immediate
  in
  let old_bytes =
    Bin_prot.Utils.bin_dump Gpuio_protocol.Wire.Progress.bin_writer_t legacy
    |> Bigstring.to_string
  in
  let new_bytes =
    Bin_prot.Utils.bin_dump W.Presentation.bin_writer_t wire |> Bigstring.to_string
  in
  assert (String.equal new_bytes (old_bytes ^ "\000\000"));
  print_endline
    "legacy label/fraction prefix unchanged; shape and transition are appended";
  [%expect
    {| legacy label/fraction prefix unchanged; shape and transition are appended |}]
;;

let%expect_test "circle children retain identity and legacy reset is explicit" =
  let module Wire = Gpuio_protocol.Wire in
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let reconciler = Reconciler.create window in
  let config =
    Progress.Config.create ~label:"Upload" ~value:Progress.Value.indeterminate |> ok
  in
  let commit view =
    let update = Reconciler.prepare reconciler ~theme:Theme.default (Some view) |> ok in
    Reconciler.accept reconciler update |> ok;
    match Reconciler.message update with
    | Some (Apply { operations; _ }) -> operations
    | None -> []
    | Some _ -> assert false
  in
  let circle callback =
    View.progress_circle ~config [ View.button ~on_click:(fun () -> callback) "Cancel" ]
  in
  let initial = commit (circle `First) in
  let root =
    List.find_map_exn initial ~f:(function
      | Wire.Op.Create (node, Progress, "", None) -> Some node
      | _ -> None)
  in
  let child, handler =
    List.find_map_exn initial ~f:(function
      | Wire.Op.Create (node, Button, "Cancel", Some handler) -> Some (node, handler)
      | _ -> None)
  in
  assert (
    List.exists initial ~f:(function
      | Set_progress_presentation
          ( _
          , { shape = Circle
            ; transition = Tween { duration_ms = 200; easing = Ease_out }
            ; _
            } ) -> true
      | _ -> false));
  assert (List.is_empty (commit (circle `Latest)));
  let event = Wire.Event.Press (window, child, handler, 1L) in
  assert (
    match Reconciler.dispatch reconciler event with
    | Some `Latest -> true
    | _ -> false);
  let reset = commit (View.progress ~config ()) in
  assert (
    List.exists reset ~f:(function
      | Set_progress (id, _) -> Gpuio_protocol.Node_id.equal id root
      | _ -> false));
  assert (
    List.exists reset ~f:(function
      | Remove id -> Gpuio_protocol.Node_id.equal id child
      | _ -> false));
  assert (Option.is_none (Reconciler.dispatch reconciler event));
  assert (
    not
      (List.exists reset ~f:(function
         | Create _ -> true
         | _ -> false)));
  print_endline
    "same progress root; retained center callback; atomic legacy reset retires child";
  [%expect
    {| same progress root; retained center callback; atomic legacy reset retires child |}]
;;

let%expect_test "extended progress opcode and capability match Rust" =
  let module Wire = Gpuio_protocol.Wire in
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok in
  let config =
    Progress.Config.create
      ~label:"Upload"
      ~value:(Progress.Value.determinate ~fraction:0.25 |> ok)
    |> ok
  in
  let config =
    Progress.Expert.presentation_to_wire
      config
      ~shape:Linear
      ~transition:Progress.Transition.immediate
  in
  let hex text =
    String.to_list text
    |> List.map ~f:(fun char -> sprintf "%02x" (Char.to_int char))
    |> String.concat
  in
  let bytes =
    Wire.Message.encode
      (Apply
         { window
         ; base = 0L
         ; revision = 1L
         ; operations = [ Set_progress_presentation (node, config) ]
         })
    |> ok
    |> hex
  in
  Eio_main.run (fun env ->
    assert (
      String.equal
        bytes
        (Eio.Path.load
           Eio.Path.(Eio.Stdenv.fs env / "progress-presentation-operation.hex")
         |> String.strip)));
  assert (
    String.equal
      (Wire.Message.encode (Hello (Wire.version, 576460752303423488L)) |> ok |> hex)
      "0003fc0000000000000008");
  print_endline "operation 66; progress presentation capability 59";
  [%expect {| operation 66; progress presentation capability 59 |}]
;;
