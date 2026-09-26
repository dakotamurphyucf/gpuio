open Core
open Gpuio
module W = Gpuio_protocol.Navigation_stack_wire
module Wire = Gpuio_protocol.Wire

let ok = Or_error.ok_exn

let%expect_test "navigation kind and configuration operation match independent fixture" =
  let id = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok in
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let config =
    Navigation_stack.Expert.presentation_config
      Navigation_stack.empty
      ~hidden:Retain
      ~motion:Navigation_stack.Motion.default
  in
  let message =
    Wire.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations =
          [ Create (id, Navigation_stack, "Routes", None)
          ; Set_navigation_stack (id, config)
          ; Set_root (Some id)
          ]
      }
  in
  Eio_main.run (fun env ->
    let expected =
      Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / "navigation-stack-request.hex")
      |> String.strip
    in
    let encoded = Wire.Message.encode message |> ok in
    let hex =
      String.to_list encoded
      |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
      |> String.concat
    in
    assert (String.equal expected hex);
    let pos_ref = ref 0 in
    assert (
      Wire.Message.equal
        message
        (Wire.Message.bin_read_t (Bigstring.of_string encoded) ~pos_ref));
    assert (!pos_ref = String.length encoded));
  print_endline "navigation kind 45 and operation 48 agree with independent fixture";
  [%expect {| navigation kind 45 and operation 48 agree with independent fixture |}]
;;

let%expect_test "native presentation policy keeps history positions under Unmount" =
  let entries =
    List.init 3 ~f:(fun i ->
      let label = Int.to_string i in
      let id = Navigation_stack.Id.of_string label |> ok in
      Navigation_stack.Entry.create ~id ~label () |> ok)
  in
  let stack = Navigation_stack.create entries |> ok in
  let config =
    Navigation_stack.Expert.presentation_config
      stack
      ~hidden:Content_policy.Retain
      ~motion:Navigation_stack.Motion.default
  in
  assert (W.Config.valid_children config ~count:3);
  let unmounted =
    Navigation_stack.Expert.presentation_config
      (Navigation_stack.pop stack)
      ~hidden:Content_policy.Unmount
      ~motion:Navigation_stack.Motion.immediate
  in
  print_s [%sexp (config : W.Config.t), (unmounted : W.Config.t)];
  Eio_main.run (fun env ->
    let expected =
      Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / "navigation-stack-config.hex")
      |> String.strip
    in
    let bytes =
      Bin_prot.Utils.bin_dump [%bin_writer: W.Config.t] config |> Bigstring.to_string
    in
    let hex =
      String.to_list bytes
      |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
      |> String.concat
    in
    assert (String.equal expected hex));
  [%expect
    {|
    (((selected (2)) (retain true) (motion Slide) (duration_ms 200))
     ((selected (1)) (retain false) (motion Immediate) (duration_ms 0)))
    |}]
;;

let%expect_test "mounted retained routes reconcile selection without rebuilding children" =
  let entry name =
    let id = Navigation_stack.Id.of_string name |> ok in
    Navigation_stack.Entry.create ~id ~label:name () |> ok
  in
  let model = Navigation_stack.create [ entry "first"; entry "second" ] |> ok in
  let model = Navigation_stack.pop model in
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let reconciler = Reconciler.create window in
  let built = ref [] in
  let view hidden model =
    View.navigation_stack
      model
      ~hidden
      ~label:"Routes"
      ~content:(fun entry ->
        let label = Navigation_stack.Entry.label entry in
        built := label :: !built;
        [ View.button ~key:(Key.of_string_exn "action") ~on_click:(fun () -> label) label
        ])
      ()
  in
  let commit hidden model =
    let update =
      Reconciler.prepare reconciler ~theme:Theme.default (Some (view hidden model)) |> ok
    in
    Reconciler.accept reconciler update |> ok;
    match Reconciler.message update with
    | Some (Wire.Message.Apply tx) -> tx.operations
    | None -> []
    | Some _ -> assert false
  in
  let first = commit Retain model in
  let first_node, first_handler =
    List.find_map_exn first ~f:(function
      | Wire.Op.Create (id, Button, "first", Some handler) -> Some (id, handler)
      | _ -> None)
  in
  assert (List.equal String.equal (List.rev !built) [ "first"; "second" ]);
  built := [];
  let next = commit Retain (Navigation_stack.forward model) in
  assert (List.equal String.equal (List.rev !built) [ "first"; "second" ]);
  assert (List.length next = 1);
  assert (
    List.exists next ~f:(function
      | Set_navigation_stack (_, config) ->
        Option.equal Int64.equal config.selected (Some 1L)
      | _ -> false));
  built := [];
  let removed = commit Unmount (Navigation_stack.forward model) in
  assert (List.equal String.equal !built [ "second" ]);
  assert (
    List.exists removed ~f:(function
      | Remove id -> Gpuio_protocol.Node_id.equal id first_node
      | _ -> false));
  assert (
    Option.is_none
      (Reconciler.dispatch
         reconciler
         (Wire.Event.Press (window, first_node, first_handler, 1L))));
  built := [];
  ignore (commit Unmount Navigation_stack.empty : Wire.Op.t list);
  assert (List.is_empty !built);
  print_endline
    "retained pages keep keys; selection-only update; Unmount skips inactive builders \
     and retires callbacks; empty builds nothing";
  [%expect
    {| retained pages keep keys; selection-only update; Unmount skips inactive builders and retires callbacks; empty builds nothing |}]
;;

let%expect_test "bounded motion duration and empty history" =
  List.iter [ -1.; 10_001. ] ~f:(fun ms ->
    assert (Result.is_error (Navigation_stack.Motion.slide (Time_ns.Span.of_ms ms))));
  List.iter [ 0.; 0.1; 10_000. ] ~f:(fun ms ->
    let motion = Navigation_stack.Motion.fade (Time_ns.Span.of_ms ms) |> ok in
    let config =
      Navigation_stack.Expert.presentation_config
        Navigation_stack.empty
        ~hidden:Content_policy.Retain
        ~motion
    in
    assert (W.Config.valid_children config ~count:0);
    assert (not (W.Config.valid_children config ~count:1));
    print_s [%sexp (config : W.Config.t)]);
  [%expect
    {|
    ((selected ()) (retain true) (motion Fade) (duration_ms 0))
    ((selected ()) (retain true) (motion Fade) (duration_ms 1))
    ((selected ()) (retain true) (motion Fade) (duration_ms 10000)) |}]
;;
