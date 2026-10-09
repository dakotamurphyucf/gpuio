open Core
open Gpuio
open Gpuio_protocol
module Wire = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let window = Window_id.create ~slot:0L ~generation:1L |> ok
let node = Node_id.create ~slot:0L ~generation:1L |> ok
let motion () = Toast.Stack.Motion.default

let%expect_test "motion checked bounds and duration rounding" =
  List.iter [ Float.nan; Float.infinity; -1.; 16385. ] ~f:(fun offset ->
    assert (Result.is_error (Toast.Stack.Motion.create ~offset ())));
  List.iter
    [ Time_ns.Span.of_ns (-1.); Time_ns.Span.of_sec 60.001 ]
    ~f:(fun duration ->
      assert (Result.is_error (Toast.Stack.Motion.create ~enter:duration ()));
      assert (Result.is_error (Toast.Stack.Motion.create ~exit:duration ())));
  let motion =
    Toast.Stack.Motion.create ~enter:(Time_ns.Span.of_ns 1.) ~exit:Time_ns.Span.zero ()
    |> ok
  in
  let config = Toast.Stack.create ~motion () |> ok in
  let wire = Toast.Expert.motion config |> Option.value_exn in
  assert (Int64.equal wire.enter_ms 1L);
  assert (Int64.equal wire.exit_ms 0L);
  [%expect {| |}]
;;

let%expect_test "independent motion operation bytes and reset" =
  let config = Toast.Stack.create ~motion:(motion ()) () |> ok in
  let message : Wire.Message.t =
    Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations =
          [ Set_toast_motion (node, Toast.Expert.motion config)
          ; Set_toast_motion (node, None)
          ]
      }
  in
  Eio_main.run (fun env ->
    let hex =
      Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / "toast-motion.hex") |> String.strip
    in
    let expected =
      String.init
        (String.length hex / 2)
        ~f:(fun i ->
          Char.of_int_exn (Int.of_string ("0x" ^ String.sub hex ~pos:(i * 2) ~len:2)))
    in
    assert (String.equal expected (Wire.Message.encode message |> ok)));
  [%expect {| |}]
;;

let%expect_test "motion-only updates preserve toast identity and emit explicit reset" =
  let reconciler = Reconciler.create window in
  let view config =
    View.toast_stack
      ~config
      [ View.toast
          ~key:(Key.of_string_exn "saved")
          ~config:(Toast.Config.create ~label:"Saved" () |> ok)
          ~on_dismiss:Fn.id
          [ View.text "Saved" ]
      ]
    |> ok
  in
  let update config =
    let pending =
      Reconciler.prepare reconciler ~theme:Theme.default (Some (view config)) |> ok
    in
    let message = Reconciler.message pending in
    Reconciler.accept reconciler pending |> ok;
    message
  in
  ignore (update Toast.Stack.default : Wire.Message.t option);
  let config = Toast.Stack.create ~motion:(motion ()) () |> ok in
  (match update config with
   | Some (Apply { operations = [ Set_toast_motion (id, Some _) ]; _ }) ->
     assert (Node_id.equal id node)
   | _ -> assert false);
  assert (Option.is_none (update config));
  (match update Toast.Stack.default with
   | Some (Apply { operations = [ Set_toast_motion (id, None) ]; _ }) ->
     assert (Node_id.equal id node)
   | _ -> assert false);
  [%expect {| |}]
;;
