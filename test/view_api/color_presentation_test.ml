open Core
open Gpuio
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok
let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok
let labels = Color_input.Labels.english ~control:"Accent" |> ok

let entry label hex =
  Color_input.Palette_entry.create ~label ~color:(Color_value.Rgba.of_hex hex |> ok) |> ok
;;

let red = entry "Red" "#FF0000"
let blue = entry "Blue" "#0000FF"
let entries = [ red; red; blue ]
let featured = Color_input.Palette_section.featured ~label:"Favorites" [ red; red ] |> ok
let group = Color_input.Palette_section.group ~label:"Blues" [ blue ] |> ok
let flat = Color_input.Config.create ~labels ~palette:entries () |> ok

let grouped =
  Color_input.Config.create ~labels ~palette_sections:[ featured; group ] () |> ok
;;

let view ?appearance config =
  View.color_input
    ?appearance
    ~controller:(Key.of_string_exn "accent")
    ~config
    ~initial:Color_value.Value.Empty
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

let%expect_test "sections validate complete palettes, duplicates and bounded geometry" =
  assert (Result.is_error (Color_input.Palette_section.group ~label:"Empty" []));
  List.iter
    [ ""; " \t"; "bad\000label"; String.make 257 'a' ]
    ~f:(fun label ->
      assert (Result.is_error (Color_input.Palette_section.group ~label [ red ])));
  assert (
    Result.is_error
      (Color_input.Config.create ~labels ~palette:[] ~palette_sections:[] ()));
  List.iter
    [ [ group; featured ]
    ; [ featured; featured ]
    ; List.init 33 ~f:(fun _ -> group)
    ; [ Color_input.Palette_section.group ~label:"Full" (List.init 256 ~f:(fun _ -> red))
        |> ok
      ; group
      ]
    ]
    ~f:(fun sections ->
      assert (
        Result.is_error (Color_input.Config.create ~labels ~palette_sections:sections ())));
  assert (
    Gpuio_protocol.Color_input_wire.Config.equal
      (Color_input.Expert.config_to_wire flat)
      (Color_input.Expert.config_to_wire grouped));
  List.iter [ Float.nan; Float.infinity; -1.; 129. ] ~f:(fun size ->
    assert (Result.is_error (Color_input.Appearance.create ~swatch_size:size ()));
    assert (Result.is_error (Color_input.Appearance.create ~featured_size:size ()));
    assert (Result.is_error (Color_input.Appearance.create ~channel_height:size ())));
  assert (Result.is_error (Color_input.Appearance.create ~swatch_radius:15. ()));
  assert (Result.is_error (Color_input.Appearance.create ~outline_width:15. ()));
  assert (Result.is_error (Color_input.Appearance.create ~padding:65. ()));
  print_endline
    "bounded sections preserve duplicate slots and original color configuration bytes";
  [%expect
    {| bounded sections preserve duplicate slots and original color configuration bytes |}]
;;

let%expect_test
    "group and theme changes keep owner and do not resubmit color configuration"
  =
  let r = Reconciler.create window in
  let first = commit r Theme.default (view flat) in
  let owner =
    List.find_map_exn first ~f:(function
      | W.Op.Create (n, Color_input, _, _) -> Some n
      | _ -> None)
  in
  let appearance =
    Color_input.Appearance.create ~selected_border:(Color.token_exn "ring") () |> ok
  in
  let theme rgb = Theme.create [ "ring", Color.rgb_exn rgb ] |> ok in
  let check rgb ops =
    match ops with
    | [ W.Op.Set_color_presentation (n, Some p) ] ->
      assert (Gpuio_protocol.Node_id.equal owner n);
      assert (List.length p.sections = 2);
      assert (Option.equal Int64.equal p.selected_border (Some rgb))
    | _ -> assert false
  in
  check 0x112233ffL (commit r (theme 0x112233) (view ~appearance grouped));
  check 0x445566ffL (commit r (theme 0x445566) (view ~appearance grouped));
  assert (List.is_empty (commit r (theme 0x445566) (view ~appearance grouped)));
  let revision = Reconciler.revision r in
  assert (
    Result.is_error
      (Reconciler.prepare r ~theme:Theme.default (Some (view ~appearance grouped))));
  assert (Int64.equal revision (Reconciler.revision r));
  (match commit r Theme.default (view flat) with
   | [ W.Op.Set_color_presentation (n, None) ] ->
     assert (Gpuio_protocol.Node_id.equal owner n)
   | _ -> assert false);
  print_endline "group/style/theme/reset preserve owner; unresolved theme is atomic";
  [%expect {| group/style/theme/reset preserve owner; unresolved theme is atomic |}]
;;

let%expect_test "independent color presentation bytes" =
  let appearance =
    Color_input.Appearance.create
      ~swatch_size:32.
      ~featured_size:48.
      ~swatch_gap:8.
      ~section_gap:12.
      ~swatch_radius:6.
      ~channel_height:40.
      ~control_gap:12.
      ~padding:12.
      ~selected_border:(Color.rgba ~red:17 ~green:34 ~blue:51 ~alpha:68 |> ok)
      ~panels:
        (Color_input.Panels.tabs
           ~palette_label:"Swatches"
           ~channels_label:"HSLA"
           ~initial:Channels
           ()
         |> ok)
      ()
    |> ok
  in
  let p =
    Color_input.Expert.presentation_to_wire grouped ~appearance ~theme:Theme.default |> ok
  in
  let message =
    W.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations =
          [ Set_color_presentation (node, Some p); Set_color_presentation (node, None) ]
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
      Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / "color-presentation.hex")
      |> String.strip)
  in
  assert (String.equal hex expected);
  print_endline "tag 87, sections, geometry, color and reset match independent bytes";
  [%expect {| tag 87, sections, geometry, color and reset match independent bytes |}]
;;

let%expect_test "tabs validate labels and reconcile as retained presentation" =
  List.iter
    [ ""; " \t"; "bad\000label"; "\255"; String.make 257 'a' ]
    ~f:(fun label ->
      assert (
        Result.is_error
          (Color_input.Panels.tabs ~palette_label:label ~channels_label:"HSLA" ()));
      assert (
        Result.is_error
          (Color_input.Panels.tabs ~palette_label:"Palette" ~channels_label:label ())));
  let r = Reconciler.create window in
  ignore (commit r Theme.default (view flat) : W.Op.t list);
  let panels =
    Color_input.Panels.tabs ~palette_label:"Colors" ~channels_label:"Channels" () |> ok
  in
  let appearance = Color_input.Appearance.create ~panels () |> ok in
  (match commit r Theme.default (view ~appearance flat) with
   | [ W.Op.Set_color_presentation
         ( _
         , Some
             { panels =
                 Tabs
                   { palette_label = "Colors"
                   ; channels_label = "Channels"
                   ; initial = Palette
                   }
             ; _
             } )
     ] -> ()
   | _ -> assert false);
  assert (List.is_empty (commit r Theme.default (view ~appearance flat)));
  (match commit r Theme.default (view flat) with
   | [ W.Op.Set_color_presentation (_, None) ] -> ()
   | _ -> assert false);
  print_endline
    "bounded localized labels; default palette tab; same owner on mode change/reset";
  [%expect
    {| bounded localized labels; default palette tab; same owner on mode change/reset |}]
;;
