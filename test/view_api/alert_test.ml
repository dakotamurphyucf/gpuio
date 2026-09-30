open Core
open Gpuio
module A = Presentation.Alert
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let key = Key.of_string_exn
let px = Length.px_exn
let p = Presentation.Appearance.dark

let rec nodes view =
  let d = View.Expert.describe view in
  d :: List.concat_map d.children ~f:nodes
;;

let by_key view name =
  List.find_exn (nodes view) ~f:(fun d -> Option.equal Key.equal d.key (Some (key name)))
;;

let fields d =
  Style.Expert.to_wire d.View.Expert.style ~theme:Theme.default
  |> ok
  |> List.concat_map ~f:(function
    | W.Style.Fields f -> f
    | _ -> [])
;;

let has d field = List.mem (fields d) field ~equal:W.Field.equal

let commit r view =
  let update = Reconciler.prepare r ~theme:Theme.default view |> ok in
  Reconciler.accept r update |> ok;
  match Reconciler.message update with
  | Some (Apply { operations; _ }) -> operations
  | None -> []
  | Some _ -> assert false
;;

let%expect_test "close labels are validated and title policies preserve body structure" =
  List.iter
    [ ""; "  \n"; "\000"; "\255"; String.make 1025 'x' ]
    ~f:(fun label ->
      assert (Result.is_error (A.Close.create ~label ~on_click:(fun () -> ()) ())));
  List.iter
    [ "关闭提醒"; String.make 1024 'x' ]
    ~f:(fun label -> ignore (A.Close.create ~label ~on_click:(fun () -> ()) () |> ok));
  let title = A.title "Title" in
  List.iter [ A.Layout.Card; Banner ] ~f:(fun layout ->
    let view = A.create p ~layout ~title [ View.text "Body" ] in
    let texts = List.map (nodes view) ~f:(fun d -> d.text) in
    assert (List.mem texts "Body" ~equal:String.equal);
    assert (
      Bool.equal (List.mem texts "Title" ~equal:String.equal) (A.Layout.equal layout Card));
    assert (has (View.Expert.describe view) (Border_top_width 1.)));
  let d = View.Expert.describe title in
  assert (has d (Font_weight 600L));
  assert (has d (Text_overflow 1L));
  print_endline
    "localized close labels validated; card title optional, banner title absent with \
     border retained; text title truncates";
  [%expect
    {| localized close labels validated; card title optional, banner title absent with border retained; text title truncates |}]
;;

let%expect_test "sizes, independent slot styles and live metadata are explicit" =
  List.iter
    [ A.Size.XSmall, 12., 6., 6.
    ; Small, 12., 8., 6.
    ; Medium, 16., 10., 12.
    ; Large, 20., 14., 12.
    ]
    ~f:(fun (size, x, y, gap) ->
      let root = A.create p ~size [] |> View.Expert.describe in
      assert (has root (Padding_left (Px x)));
      assert (has root (Padding_top (Px y)));
      assert (has root (Row_gap (Px gap))));
  let view =
    A.create
      p
      ~style:(Style.create_exn [ Radius 3.; Border_width 2. ])
      ~body_style:(Style.create_exn [ Gap (px 9.) ])
      ~title_style:(Style.create_exn [ Padding (px 7.) ])
      ~icon_style:(Style.create_exn [ Margin_top (px 1.) ])
      ~title:(View.text "Rich")
      [ View.text "Body" ]
  in
  let root = View.Expert.describe view in
  assert (has root (Top_left_radius 3.));
  assert (has root (Border_top_width 2.));
  assert (has (by_key view "body") (Row_gap (Px 9.)));
  assert (has (by_key view "title") (Padding_top (Px 7.)));
  assert (has (by_key view "icon") (Margin_top (Px 1.)));
  let config = Option.value_exn root.accessibility |> Accessibility.Expert.to_wire in
  assert (Gpuio_protocol.Accessibility_wire.Live.equal config.live Off);
  assert (
    Option.equal Gpuio_protocol.Accessibility_wire.Role.equal config.role (Some Alert));
  List.iter [ Accessibility.Live.Polite; Assertive ] ~f:(fun live ->
    let config =
      (A.create p ~live [] |> View.Expert.describe).accessibility
      |> Option.value_exn
      |> Accessibility.Expert.to_wire
    in
    assert (
      Gpuio_protocol.Accessibility_wire.Live.equal
        config.live
        (match live with
         | Off -> Off
         | Polite -> Polite
         | Assertive -> Assertive)));
  let hidden =
    A.create
      p
      ~visible:false
      ~style:(Style.create_exn [ Display Flex ])
      ~title:(View.button ~on_click:(fun () -> ()) "Hidden action")
      [ View.text "Hidden body" ]
    |> View.Expert.describe
  in
  assert (List.is_empty hidden.children);
  assert (Option.is_none hidden.accessibility);
  print_endline
    "four sizes; root/title/body/icon style precedence; Alert with explicit live \
     Off/Polite/Assertive; hidden content removed";
  [%expect
    {| four sizes; root/title/body/icon style precedence; Alert with explicit live Off/Polite/Assertive; hidden content removed |}]
;;

let%expect_test
    "variant, banner, size and icon changes preserve actions; hidden actions retire"
  =
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let r = Reconciler.create window in
  let view revision ?(visible = true) ~variant ~size ~layout ~icon () =
    A.create
      p
      ~key:(key "alert")
      ~variant
      ~size
      ~layout
      ~icon
      ~visible
      ~title:(A.title "Title")
      ~close:
        (A.Close.create ~label:"Dismiss" ~on_click:(fun () -> revision + 1000) () |> ok)
      [ View.button ~key:(key "retry") ~on_click:(fun () -> revision) "Retry" ]
  in
  let controls =
    commit r (Some (view 0 ~variant:Default ~size:Medium ~layout:Card ~icon:Default ()))
    |> List.filter_map ~f:(function
      | W.Op.Create (n, Button, _, Some h) -> Some (n, h)
      | _ -> None)
  in
  assert (List.length controls = 2);
  let count = ref 0 in
  List.iter [ A.Variant.Default; Info; Success; Warning; Error ] ~f:(fun variant ->
    List.iter [ A.Layout.Card; Banner ] ~f:(fun layout ->
      List.iter [ A.Size.XSmall; Small; Medium; Large ] ~f:(fun size ->
        incr count;
        let icon =
          if !count % 2 = 0 then A.Icon.Hidden else Custom (View.text "Custom")
        in
        let next = view !count ~variant ~size ~layout ~icon () in
        let ops = commit r (Some next) in
        List.iter ops ~f:(function
          | W.Op.Create (_, Button, _, _) -> failwith "button remounted"
          | Set_accessibility _ -> failwith "cosmetic change rewrote live metadata"
          | Remove n ->
            assert (
              not
                (List.exists controls ~f:(fun (c, _) -> Gpuio_protocol.Node_id.equal c n)))
          | _ -> ());
        let callbacks =
          List.map controls ~f:(fun (n, h) ->
            Reconciler.dispatch r (W.Event.Press (window, n, h, 1L)) |> Option.value_exn)
          |> List.sort ~compare:Int.compare
        in
        assert (List.equal Int.equal callbacks [ !count; !count + 1000 ]);
        assert (List.is_empty (commit r (Some next))))));
  ignore
    (commit
       r
       (Some
          (view
             99
             ~visible:false
             ~variant:Default
             ~size:Medium
             ~layout:Card
             ~icon:Default
             ()))
     : W.Op.t list);
  List.iter controls ~f:(fun (n, h) ->
    assert (Option.is_none (Reconciler.dispatch r (W.Event.Press (window, n, h, 1L)))));
  let recreated =
    commit r (Some (view 100 ~variant:Default ~size:Medium ~layout:Card ~icon:Default ()))
  in
  assert (
    List.count recreated ~f:(function
      | W.Op.Create (_, Button, _, _) -> true
      | _ -> false)
    = 2);
  printf
    "%d transitions retain both controls/current callbacks; equal snapshots idle; hide \
     rejects late actions; show remounts\n"
    !count;
  [%expect
    {| 40 transitions retain both controls/current callbacks; equal snapshots idle; hide rejects late actions; show remounts |}]
;;

let%expect_test "semantic colors retain application tokens and caller alpha" =
  let c = Color.token_exn "ink" in
  let appearance =
    Presentation.Appearance.create
      ~surface:(Color.rgb_exn 0xffffff)
      ~raised:(Color.rgb_exn 0xeeeeee)
      ~foreground:c
      ~muted:c
      ~border:(Color.rgb_exn 0x445566)
      ~on_solid:c
      ~accent:c
      ~success:c
      ~warning:c
      ~danger:c
  in
  let theme =
    Theme.create [ "ink", Color.rgba ~red:18 ~green:52 ~blue:86 ~alpha:128 |> ok ] |> ok
  in
  List.iter [ A.Variant.Info; Success; Warning; Error ] ~f:(fun variant ->
    let d = A.create appearance ~variant [] |> View.Expert.describe in
    let styles = Style.Expert.to_wire d.style ~theme |> ok in
    let fields =
      List.concat_map styles ~f:(function
        | W.Style.Fields f -> f
        | _ -> [])
    in
    assert (List.mem fields (Foreground (Rgba 0x12345680L)) ~equal:W.Field.equal);
    assert (List.mem fields (Background (Solid (Rgba 0x12345605L))) ~equal:W.Field.equal);
    assert (List.mem fields (Border_color (Rgba 0x12345626L)) ~equal:W.Field.equal));
  let close =
    A.Close.create
      ~label:"Dismiss"
      ~style:(Style.create_exn [ Padding (px 9.) ])
      ~on_click:(fun () -> ())
      ()
    |> ok
  in
  assert (has (by_key (A.create p ~close []) "close") (Padding_top (Px 9.)));
  print_endline
    "Info/Success/Warning/Error resolve application color and multiplied alpha; close \
     styles refine native button defaults";
  [%expect
    {| Info/Success/Warning/Error resolve application color and multiplied alpha; close styles refine native button defaults |}]
;;
