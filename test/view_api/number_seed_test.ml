open Core
open Gpuio
module N = Number_input
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let node = Gpuio_protocol.Node_id.create ~slot:2L ~generation:3L |> ok
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok

let hex bytes =
  String.to_list bytes
  |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
  |> String.concat
;;

let%expect_test "numeric draft seed has independent request bytes and validated text" =
  let request draft =
    W.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations = [ Set_number_input_draft (node, draft) ]
      }
  in
  let encode value = W.Message.encode value |> ok |> hex in
  print_endline (encode (request (Some "1e-")));
  print_endline (encode (request None));
  List.iter
    [ ""; "-"; "invalid"; String.concat (List.init 2048 ~f:(fun _ -> "é")) ]
    ~f:(fun text ->
      assert (String.equal text (N.Draft.of_string text |> ok |> N.Draft.to_string)));
  List.iter
    [ "\000"; "a\nb"; "\r"; "\255"; String.make 4097 'x' ]
    ~f:(fun text -> assert (Or_error.is_error (N.Draft.of_string text)));
  assert (Int64.equal (Int64.bit_and W.capabilities 9007199254740992L) 9007199254740992L);
  [%expect
    {|
    0300010001013f0203010331652d
    0300010001013f020300
    |}]
;;

let%expect_test "numeric draft seeds only initialize new native placements" =
  let domain = Numeric.Domain.create ~min:0. ~max:10. ~step:1. |> ok in
  let config = N.Config.create ~domain ~label:"Attempts" () |> ok in
  let view seed =
    View.number_input
      ~controller:(Key.of_string_exn "attempts")
      ~config
      ~initial:(N.Value.of_float 3. |> ok)
      ?initial_draft:(Option.map seed ~f:(fun text -> N.Draft.of_string text |> ok))
      ~on_event:(fun _ -> ())
      ()
  in
  let reconciler = Reconciler.create window in
  let commit view =
    let update = Reconciler.prepare reconciler ~theme:Theme.default view |> ok in
    Reconciler.accept reconciler update |> ok;
    match Reconciler.message update with
    | Some (Apply tx) -> tx.operations
    | _ -> []
  in
  let initial = commit (Some (view (Some "1e-"))) in
  let seed operations =
    List.find_map_exn operations ~f:(function
      | W.Op.Set_number_input_draft (id, text) -> Some (id, text)
      | _ -> None)
  in
  let original, text = seed initial in
  assert (Option.equal String.equal text (Some "1e-"));
  assert (List.is_empty (commit (Some (view (Some "next draft")))));
  assert (List.is_empty (commit (Some (view None))));
  ignore (commit None : W.Op.t list);
  let replacement, text = commit (Some (view (Some ""))) |> seed in
  assert (not (Gpuio_protocol.Node_id.equal replacement original));
  assert (Option.equal String.equal text (Some ""));
  ignore (commit None : W.Op.t list);
  assert (
    List.for_all
      (commit (Some (view None)))
      ~f:(function
        | W.Op.Set_number_input_draft _ -> false
        | _ -> true));
  Reconciler.close reconciler;
  print_endline
    "draft updates are silent on live nodes; remount seeds fresh identity; None and \
     empty draft are distinct";
  [%expect
    {| draft updates are silent on live nodes; remount seeds fresh identity; None and empty draft are distinct |}]
;;
