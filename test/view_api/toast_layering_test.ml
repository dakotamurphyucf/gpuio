open Core
open Gpuio
open Gpuio_protocol
module Wire = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let window = Window_id.create ~slot:0L ~generation:1L |> ok
let node = Node_id.create ~slot:0L ~generation:1L |> ok
let layering () = Toast.Stack.Layering.default

let%expect_test "layering checked bounds" =
  List.iter [ Float.nan; Float.infinity; -1.; 16385. ] ~f:(fun n ->
    assert (Result.is_error (Toast.Stack.Layering.create ~peek:n ()));
    assert (Result.is_error (Toast.Stack.Layering.create ~gap:n ())));
  List.iter [ Float.nan; Float.infinity; -0.1; 0.1001 ] ~f:(fun n ->
    assert (Result.is_error (Toast.Stack.Layering.create ~width_step:n ())));
  List.iter [ 0; 9 ] ~f:(fun visible ->
    assert (Result.is_error (Toast.Stack.Layering.create ~visible ())));
  assert (Result.is_ok (Toast.Stack.Layering.create ~width_step:0.1 ~visible:8 ()));
  [%expect {| |}]
;;

let%expect_test "independent layering operation bytes and reset" =
  let config = Toast.Stack.create ~layering:(layering ()) () |> ok in
  let message : Wire.Message.t =
    Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations =
          [ Set_toast_layering (node, Toast.Expert.layering config)
          ; Set_toast_layering (node, None)
          ]
      }
  in
  Eio_main.run (fun env ->
    let hex =
      Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / "toast-layering.hex") |> String.strip
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

let%expect_test "layering-only updates preserve toast identity and emit explicit reset" =
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
  let config = Toast.Stack.create ~layering:(layering ()) () |> ok in
  (match update config with
   | Some (Apply { operations = [ Set_toast_layering (id, Some _) ]; _ }) ->
     assert (Node_id.equal id node)
   | _ -> assert false);
  assert (Option.is_none (update config));
  (match update Toast.Stack.default with
   | Some (Apply { operations = [ Set_toast_layering (id, None) ]; _ }) ->
     assert (Node_id.equal id node)
   | _ -> assert false);
  [%expect {| |}]
;;
