open Core
open Gpuio
module P = Gpuio_protocol
module W = P.Wire

let ok = Or_error.ok_exn
let id text = Choice.Id.of_string text |> ok
let style = Style.create_exn

let%expect_test "tab appearance validates geometry, target styles and bounded overrides" =
  List.iter [ Float.nan; Float.infinity; 0.; 15.9; 256.1 ] ~f:(fun height ->
    assert (Result.is_error (Tab_bar.Appearance.create ~height ())));
  List.iter
    [ style [ Display Hidden ]
    ; style [ Overflow_x Scroll ]
    ; style [ User_select true ]
    ; Style.with_state_exn Style.empty Checked [ Opacity 0.5 ]
    ]
    ~f:(fun tab_style ->
      assert (Result.is_error (Tab_bar.Appearance.create ~tab_style ())));
  assert (
    Result.is_error
      (Tab_bar.Appearance.create
         ~item_styles:[ id "a", Style.empty; id "a", Style.empty ]
         ()));
  assert (
    Result.is_error
      (Tab_bar.Appearance.create
         ~item_styles:
           (List.init 128 ~f:(fun i ->
              ( id (Int.to_string i)
              , style
                  [ Width (Length.px_exn 80.); Height (Length.px_exn 32.); Opacity 0.8 ] )))
         ()));
  let appearance =
    Tab_bar.Appearance.create ~variant:Pill ()
    |> ok
    |> fun t -> Tab_bar.Expert.to_wire t ~theme:Theme.default |> ok
  in
  let node = P.Node_id.create ~slot:0L ~generation:1L |> ok in
  let bytes =
    Bin_prot.Utils.bin_dump W.Op.bin_writer_t (Set_tab_appearance (node, Some appearance))
    |> Bigstring.to_string
  in
  let hex =
    String.to_list bytes
    |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
    |> String.concat
  in
  assert (
    String.equal hex "62000101020000000000004040000000000000104000000000000028400000");
  print_endline
    "checked geometry, states, identities and total declarations; paired Op98 bytes";
  [%expect
    {| checked geometry, states, identities and total declarations; paired Op98 bytes |}]
;;

let%expect_test "tab appearance updates and resets do not replace choice or label owners" =
  let window = P.Window_id.create ~slot:0L ~generation:1L |> ok in
  let reconciler = Reconciler.create window in
  let config =
    Choice.Config.create
      ~label:"Tabs"
      ~options:
        (Choice.Collection.create
           [ Choice.create ~id:(id "a") ~label:"Alpha" () |> ok
           ; Choice.create ~id:(id "b") ~label:"Beta" () |> ok
           ]
         |> ok)
      ~selected:(Some (id "a"))
      ()
    |> ok
  in
  let view appearance =
    View.tab_bar_with_labels
      ?appearance
      ~key:(Key.of_string_exn "tabs")
      ~config
      ~labels:[ id "a", View.text "Decorative A" ]
      ~on_select:Choice.Id.to_string
      ()
    |> ok
  in
  let commit appearance =
    let update =
      Reconciler.prepare reconciler ~theme:Theme.default (Some (view appearance)) |> ok
    in
    Reconciler.accept reconciler update |> ok;
    match Reconciler.message update with
    | Some (Apply tx) -> tx.operations
    | None -> []
    | Some _ -> assert false
  in
  let first = commit None in
  assert (
    not
      (List.exists first ~f:(function
         | W.Op.Set_tab_appearance _ -> true
         | _ -> false)));
  List.iter
    [ Tab_bar.Variant.Tab; Outline; Pill; Segmented; Underline ]
    ~f:(fun variant ->
      let appearance =
        Tab_bar.Appearance.create
          ~variant
          ~tab_style:
            (Style.with_state_exn
               Style.empty
               Selected
               [ Foreground (Color.rgb_exn 0x112233) ])
          ~item_styles:[ id "b", style [ Width (Length.px_exn 140.) ] ]
          ()
        |> ok
      in
      let ops = commit (Some appearance) in
      assert (List.length ops = 1);
      (match ops with
       | [ Set_tab_appearance (_, Some wire) ] ->
         let expected =
           match variant with
           | Tab_bar.Variant.Tab -> W.Tab_appearance.Variant.Tab
           | Outline -> Outline
           | Pill -> Pill
           | Segmented -> Segmented
           | Underline -> Underline
         in
         assert (P.Wire.Tab_appearance.Variant.equal wire.variant expected)
       | _ -> assert false);
      assert (List.is_empty (commit (Some appearance))));
  (match commit None with
   | [ Set_tab_appearance (_, None) ] -> ()
   | _ -> assert false);
  print_endline
    "five variants; one presentation operation per change; no remount or selection \
     rewrite; explicit reset";
  [%expect
    {| five variants; one presentation operation per change; no remount or selection rewrite; explicit reset |}]
;;
