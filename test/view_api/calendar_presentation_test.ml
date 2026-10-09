open Core
open Gpuio
module W = Gpuio_protocol.Wire
module P = Gpuio_protocol.Calendar_presentation_wire

let ok = Or_error.ok_exn
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok
let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok
let config = Calendar.Config.create ~label:"Dates" () |> ok
let month = Calendar.Month.create ~year:2024 ~month:Month.Feb |> ok

let view ?appearance () =
  View.calendar
    ?appearance
    ~controller:(Key.of_string_exn "dates")
    ~config
    ~initial:Calendar.Selection.empty
    ~initial_month:month
    ~on_event:Fn.id
    ()
;;

let commit r theme view =
  let update = Reconciler.prepare r ~theme (Some view) |> ok in
  Reconciler.accept r update |> ok;
  match Reconciler.message update with
  | Some (W.Message.Apply tx) -> tx.operations
  | None -> []
  | Some _ -> assert false
;;

let%expect_test
    "calendar appearance is validated, theme-resolved and independent of native ownership"
  =
  List.iter [ Float.nan; Float.infinity; -1.; 129. ] ~f:(fun bad ->
    assert (Result.is_error (Calendar.Appearance.create ~cell_height:bad ())));
  List.iter [ Float.nan; Float.infinity; -1.; 65. ] ~f:(fun bad ->
    assert (Result.is_error (Calendar.Appearance.create ~cell_gap:bad ()));
    assert (Result.is_error (Calendar.Appearance.create ~cell_radius:bad ())));
  List.iter [ Int.min_value; 0; 13; Int.max_value ] ~f:(fun months ->
    assert (Result.is_error (Calendar.Appearance.create ~months ())));
  assert (Result.is_error (Calendar.Appearance.create ~outline_width:17. ()));
  let appearance =
    Calendar.Appearance.create ~months:2 ~selected_background:(Color.token_exn "cell") ()
    |> ok
  in
  let theme value = Theme.create [ "cell", Color.rgb_exn value ] |> ok in
  let r = Reconciler.create window in
  let initial = commit r (theme 0x112233) (view ()) in
  let owner =
    List.find_map_exn initial ~f:(function
      | W.Op.Create (node, Calendar, _, _) -> Some node
      | _ -> None)
  in
  let check expected = function
    | [ W.Op.Set_calendar_appearance (node, Some value) ] ->
      assert (Gpuio_protocol.Node_id.equal owner node);
      assert (Int64.equal value.months 2L);
      assert (Option.equal Int64.equal value.selected_background (Some expected))
    | _ -> assert false
  in
  check 0x112233ffL (commit r (theme 0x112233) (view ~appearance ()));
  check 0x445566ffL (commit r (theme 0x445566) (view ~appearance ()));
  assert (List.is_empty (commit r (theme 0x445566) (view ~appearance ())));
  let revision = Reconciler.revision r in
  assert (
    Result.is_error
      (Reconciler.prepare r ~theme:Theme.default (Some (view ~appearance ()))));
  assert (Int64.equal revision (Reconciler.revision r));
  (match commit r Theme.default (view ~appearance:Calendar.Appearance.default ()) with
   | [ W.Op.Set_calendar_appearance (node, None) ] ->
     assert (Gpuio_protocol.Node_id.equal owner node)
   | _ -> assert false);
  assert (List.is_empty (commit r Theme.default (view ())));
  print_endline
    "only appearance changes; theme failure is atomic; default removes override without \
     replacing calendar";
  [%expect
    {| only appearance changes; theme failure is atomic; default removes override without replacing calendar |}]
;;

let%expect_test "independent calendar presentation operation fixture" =
  let value =
    { P.default with
      months = 2L
    ; cell_height = 40.
    ; cell_gap = 6.
    ; month_gap = 20.
    ; padding = 10.
    ; cell_radius = 8.
    ; outline_width = 2.
    ; selected_background = Some 0x11223344L
    ; selected_foreground = Some 0xffffffffL
    }
  in
  let message =
    W.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations =
          [ Set_calendar_appearance (node, Some value)
          ; Set_calendar_appearance (node, None)
          ]
      }
  in
  let bytes =
    Bin_prot.Utils.bin_dump W.Message.bin_writer_t message |> Bigstring.to_string
  in
  let hex =
    String.to_list bytes
    |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
    |> String.concat
  in
  let expected =
    Eio_main.run (fun env ->
      Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / "calendar-presentation.hex")
      |> String.strip)
  in
  assert (String.equal hex expected);
  assert (P.valid value);
  assert (not (P.valid { value with focus_border = Some (-1L) }));
  print_endline "tag 86, optional geometry/colors, and reset match independent bytes";
  [%expect {| tag 86, optional geometry/colors, and reset match independent bytes |}]
;;
