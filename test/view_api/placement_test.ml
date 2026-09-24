open Core
open Gpuio
open Gpuio_protocol

let window = Window_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn

let placement =
  Placement.create ~side:Left ~align:End ~offset:(-3.5) () |> Or_error.ok_exn
;;

let%expect_test "placement is bounded and matches the independent Rust update" =
  List.iter [ Float.nan; Float.infinity; -16385.; 16385. ] ~f:(fun offset ->
    assert (Placement.create ~offset () |> Result.is_error));
  let node = Node_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn in
  let message =
    Wire.Message.Apply
      { window
      ; base = 1L
      ; revision = 2L
      ; operations = [ Set_placement (node, Some (Placement.Expert.to_wire placement)) ]
      }
  in
  Eio_main.run (fun env ->
    let hex =
      Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / "placement-v1-request.hex")
      |> String.strip
    in
    let bytes =
      String.init
        (String.length hex / 2)
        ~f:(fun i ->
          Char.of_int_exn (Int.of_string ("0x" ^ String.sub hex ~pos:(i * 2) ~len:2)))
    in
    assert (String.equal bytes (Wire.Message.encode message |> Or_error.ok_exn)));
  [%expect {| |}]
;;

let%expect_test "placement updates preserve open overlay and child identity" =
  let reconciler = Reconciler.create window in
  let commit placement =
    let config =
      Overlay.Config.create ~label:"Details" ~placement () |> Or_error.ok_exn
    in
    let view =
      View.popover
        ~config
        ~anchor:(View.text "Anchor")
        ~on_dismiss:Fn.id
        (Some (View.text "Details"))
    in
    let update =
      Reconciler.prepare reconciler ~theme:Theme.default (Some view) |> Or_error.ok_exn
    in
    Reconciler.accept reconciler update |> Or_error.ok_exn;
    match Reconciler.message update with
    | Some (Wire.Message.Apply tx) -> tx.operations
    | _ -> []
  in
  ignore (commit Placement.default : Wire.Op.t list);
  print_s [%sexp (commit placement : Wire.Op.t list)];
  print_s [%sexp (commit placement : Wire.Op.t list)];
  [%expect
    {|
    ((Set_placement ((slot 2) (generation 1))
      (((side Left) (align End) (offset -3.5)))))
    ()
    |}]
;;
