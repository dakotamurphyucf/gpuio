open Core
open Gpuio
open Gpuio_protocol
module Wire = Gpuio_protocol.Wire

let config fraction =
  let value =
    match fraction with
    | None -> Progress.Value.indeterminate
    | Some fraction -> Progress.Value.determinate ~fraction |> Or_error.ok_exn
  in
  Progress.Config.create ~label:"Download" ~value |> Or_error.ok_exn
;;

let window = Window_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn
let node = Node_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn

let%expect_test "progress value and label invariants" =
  List.iter [ Float.nan; Float.infinity; -0.1; 1.1 ] ~f:(fun fraction ->
    assert (Result.is_error (Progress.Value.determinate ~fraction)));
  List.iter
    [ ""; " "; "bad\000label"; "\255"; String.make 4097 'x' ]
    ~f:(fun label ->
      assert (
        Result.is_error
          (Progress.Config.create ~label ~value:Progress.Value.indeterminate)));
  List.iter [ Some 0.; Some 1.; None ] ~f:(fun fraction ->
    assert (
      Option.equal
        Float.equal
        (Progress.Expert.to_wire (config fraction)).fraction
        fraction));
  [%expect {| |}]
;;

let%expect_test "progress updates retain identity and need no callback binding" =
  let reconciler = Reconciler.create window in
  let update fraction =
    Reconciler.prepare
      reconciler
      ~theme:Theme.default
      (Some (View.progress ~config:(config fraction) ()))
    |> Or_error.ok_exn
  in
  let initial = update (Some 0.25) in
  let message = Option.value_exn (Reconciler.message initial) in
  let expected : Wire.Message.t =
    Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations =
          [ Create (node, Progress, "", None)
          ; Set_progress (node, Progress.Expert.to_wire (config (Some 0.25)))
          ; Set_root (Some node)
          ]
      }
  in
  (* Compare independent encodings while allowing the reconciler's stable ordering
     of root/style operations to be tested separately below. *)
  Eio_main.run (fun env ->
    let hex =
      Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / "progress-v1-request.hex")
      |> String.strip
    in
    let bytes =
      String.init
        (String.length hex / 2)
        ~f:(fun i ->
          Char.of_int_exn (Int.of_string ("0x" ^ String.sub hex ~pos:(i * 2) ~len:2)))
    in
    assert (String.equal bytes (Wire.Message.encode expected |> Or_error.ok_exn)));
  (match message with
   | Apply { operations; _ } ->
     assert (
       List.exists operations ~f:(function
         | Create (_, Progress, _, None) -> true
         | _ -> false));
     assert (
       List.exists operations ~f:(function
         | Set_progress (_, value) -> Option.equal Float.equal value.fraction (Some 0.25)
         | _ -> false))
   | _ -> assert false);
  Reconciler.accept reconciler initial |> Or_error.ok_exn;
  let changed = update None in
  (match Reconciler.message changed with
   | Some (Apply { operations = [ Set_progress (same_node, value) ]; _ }) ->
     assert (Node_id.equal same_node node && Option.is_none value.fraction)
   | _ -> assert false);
  Reconciler.accept reconciler changed |> Or_error.ok_exn;
  assert (Option.is_none (Reconciler.message (update None)));
  [%expect {| |}]
;;
