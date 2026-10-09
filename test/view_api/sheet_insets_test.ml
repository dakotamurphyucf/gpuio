open Core
open Gpuio
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok

let%expect_test "checked sheet insets retain children and reset without reopening" =
  List.iter [ -1.; Float.nan; Float.infinity; 16385. ] ~f:(fun n ->
    List.iter
      [ Sheet.Insets.create ~top:n ()
      ; Sheet.Insets.create ~right:n ()
      ; Sheet.Insets.create ~bottom:n ()
      ; Sheet.Insets.create ~left:n ()
      ]
      ~f:(fun value -> assert (Or_error.is_error value)));
  let r = Reconciler.create window in
  let update insets =
    let view =
      View.sheet
        ~config:(Sheet.Config.create ~label:"Drawer" ~insets () |> ok)
        ~on_dismiss:ignore
        (Some (View.button ~on_click:(fun () -> ()) "Keep"))
    in
    let tx = Reconciler.prepare r ~theme:Theme.default (Some view) |> ok in
    Reconciler.accept r tx |> ok;
    match Reconciler.message tx with
    | Some (W.Message.Apply tx) -> tx.operations
    | None -> []
    | Some _ -> assert false
  in
  let first = update Sheet.Insets.zero in
  assert (
    not
      (List.exists first ~f:(function
         | W.Op.Set_sheet_insets _ -> true
         | _ -> false)));
  let insets = Sheet.Insets.create ~top:34. ~right:8. ~bottom:12. ~left:20. () |> ok in
  List.iter [ insets; Sheet.Insets.zero ] ~f:(fun insets ->
    let operations = update insets in
    assert (
      List.count operations ~f:(function
        | W.Op.Set_sheet_insets _ -> true
        | _ -> false)
      = 1);
    assert (
      not
        (List.exists operations ~f:(function
           | W.Op.Create _ | Remove _ | Set_overlay _ | Set_focus_scope _ -> true
           | _ -> false))));
  print_endline
    "finite bounded edges; default omitted; inset/reset retain sheet and children";
  [%expect
    {| finite bounded edges; default omitted; inset/reset retain sheet and children |}]
;;

let%expect_test "sheet inset bytes match independent fixture" =
  let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok in
  let insets = Sheet.Insets.create ~top:34. ~right:8. ~bottom:12. ~left:20. () |> ok in
  let operations =
    [ W.Op.Set_sheet_insets (node, Some (Sheet.Expert.insets_to_wire insets))
    ; Set_sheet_insets (node, Some (Sheet.Expert.insets_to_wire Sheet.Insets.zero))
    ; Set_sheet_insets (node, None)
    ]
  in
  let bytes =
    W.Message.encode (Apply { window; base = 0L; revision = 1L; operations }) |> ok
  in
  let hex =
    String.to_list bytes
    |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
    |> String.concat
  in
  Eio_main.run (fun env ->
    assert (
      String.equal
        hex
        (Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / "sheet-insets.hex") |> String.strip)));
  print_endline "Op94 inset/zero/reset match independent bytes";
  [%expect {| Op94 inset/zero/reset match independent bytes |}]
;;
