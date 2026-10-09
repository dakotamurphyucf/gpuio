open Core
open Gpuio
module W = Gpuio_protocol.Wire
module P = Gpuio_protocol.Slider_presentation_wire

let ok = Or_error.ok_exn
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok
let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok
let domain = Numeric.Domain.create ~min:0. ~max:100. ~step:1. |> ok
let config = Slider.Config.create ~domain ~label:"Amount" () |> ok

let view ?appearance () =
  View.slider
    ?appearance
    ~controller:(Key.of_string_exn "amount")
    ~config
    ~initial:(Slider.Value.single 12. |> ok)
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
    "slider appearance is validated, theme-resolved and independent of native ownership"
  =
  List.iter [ Float.nan; Float.infinity; -1.; 257. ] ~f:(fun bad ->
    assert (Result.is_error (Slider.Appearance.create ~thumb_size:bad ()));
    assert (Result.is_error (Slider.Appearance.create ~target_size:bad ()));
    assert (Result.is_error (Slider.Appearance.create ~track_radius:bad ())));
  assert (Result.is_error (Slider.Appearance.create ~thumb_size:21. ()));
  assert (Result.is_error (Slider.Appearance.create ~ring_width:11. ()));
  let appearance =
    Slider.Appearance.create ~fill:Remaining ~track_color:(Color.token_exn "cell") ()
    |> ok
  in
  let theme value = Theme.create [ "cell", Color.rgb_exn value ] |> ok in
  let r = Reconciler.create window in
  let initial = commit r (theme 0x112233) (view ()) in
  let owner =
    List.find_map_exn initial ~f:(function
      | W.Op.Create (node, Slider, _, _) -> Some node
      | _ -> None)
  in
  let check expected = function
    | [ W.Op.Set_slider_appearance (node, Some value) ] ->
      assert (Gpuio_protocol.Node_id.equal owner node);
      assert (P.Fill.equal value.fill Remaining);
      assert (Option.equal Int64.equal value.track_color (Some expected))
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
  (match commit r Theme.default (view ~appearance:Slider.Appearance.default ()) with
   | [ W.Op.Set_slider_appearance (node, None) ] ->
     assert (Gpuio_protocol.Node_id.equal owner node)
   | _ -> assert false);
  assert (List.is_empty (commit r Theme.default (view ())));
  print_endline
    "only appearance changes; theme failure is atomic; default removes override without \
     replacing slider";
  [%expect
    {| only appearance changes; theme failure is atomic; default removes override without replacing slider |}]
;;

let%expect_test "independent slider presentation operation fixture" =
  let value =
    { P.default with
      fill = Remaining
    ; track_thickness = 6.
    ; track_radius = 3.
    ; thumb_size = 16.
    ; target_size = 28.
    ; track_color = Some 0x11223344L
    ; fill_color = Some 0xffffffffL
    ; thumb_color = Some 0L
    }
  in
  let message =
    W.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations =
          [ Set_slider_appearance (node, Some value); Set_slider_appearance (node, None) ]
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
      Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / "slider-presentation.hex")
      |> String.strip)
  in
  assert (String.equal hex expected);
  assert (P.valid value);
  assert (not (P.valid { value with thumb_color = Some (-1L) }));
  print_endline "tag 84, optional geometry/colors, and reset match independent bytes";
  [%expect {| tag 84, optional geometry/colors, and reset match independent bytes |}]
;;
