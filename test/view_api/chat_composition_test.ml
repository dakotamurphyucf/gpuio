open Core
open Gpuio
module P = Presentation
module B = P.Bubble
module M = P.Message
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let key = Key.of_string_exn
let px = Length.px_exn
let p = P.Appearance.dark
let variants = [ B.Variant.Filled; Secondary; Muted; Tinted; Outline; Ghost; Destructive ]

let rec nodes view =
  let d = View.Expert.describe view in
  d :: List.concat_map d.children ~f:nodes
;;

let fields d =
  Style.Expert.to_wire d.View.Expert.style ~theme:Theme.default
  |> ok
  |> List.concat_map ~f:(function
    | W.Style.Fields f -> f
    | _ -> [])
;;

let has d field = List.mem (fields d) field ~equal:W.Field.equal

let named view name =
  List.find_exn (nodes view) ~f:(fun d -> Option.equal Key.equal d.key (Some (key name)))
;;

let text value = View.text ~key:(key "text") value

let%expect_test
    "typed reactions validate identity and own pill geometry; arbitrary controls keep \
     styles"
  =
  let button =
    View.button ~style:(Style.create_exn [ Radius 3. ]) ~on_click:(fun () -> ()) "React"
  in
  let ordinary = B.Reactions.Item.element ~key:(key "reaction") button in
  let action =
    B.Reactions.Item.action
      ~key:(key "reaction")
      ~style:(Style.create_exn [ Radius 3. ])
      ~on_click:(fun () -> ())
      "React"
  in
  assert (Result.is_error (B.Reactions.create [ ordinary; action ]));
  List.iter [ false; true ] ~f:(fun typed ->
    let reactions = B.Reactions.create [ (if typed then action else ordinary) ] |> ok in
    let view = B.create p ~reactions [ text "Body" ] |> B.view in
    let wrapper = named view "reactions" in
    assert (has wrapper (Bottom (Px (-20.))));
    assert (has wrapper (Right (Px 12.)));
    assert (Bool.equal (has wrapper (Padding_left (Px 6.))) (not typed));
    assert (has (named view "reaction") (Top_left_radius (if typed then 999. else 3.))));
  let reactions =
    B.Reactions.create
      ~side:Top
      ~alignment:Start
      ~style:(Style.create_exn [ Top (px (-30.)); Padding (px 7.) ])
      [ action ]
    |> ok
  in
  let wrapper = named (B.create p ~reactions [] |> B.view) "reactions" in
  assert (has wrapper (Top (Px (-30.))));
  assert (has wrapper (Left (Px 12.)));
  assert (has wrapper (Padding_top (Px 7.)));
  print_endline
    "duplicate keys rejected; typed button forces pill, suppresses decorative padding; \
     arbitrary button unchanged; edge/style override";
  [%expect
    {| duplicate keys rejected; typed button forces pill, suppresses decorative padding; arbitrary button unchanged; edge/style override |}]
;;

let%expect_test
    "bubble surfaces, optional alignment and Ghost width follow explicit source policy"
  =
  List.iter variants ~f:(fun variant ->
    let bubble = B.create p ~variant [ text "Body" ] in
    assert (B.Variant.equal (B.variant bubble) variant);
    let view = B.view bubble in
    let root = View.Expert.describe view in
    let surface = named view "content" in
    let ghost = B.Variant.equal variant Ghost in
    assert (has root (Max_width (Percent (if ghost then 100. else 80.))));
    assert (Bool.equal (has root (Width (Percent 100.))) ghost);
    assert (has surface (Border_top_width (if ghost then 0. else 1.)));
    assert (has surface (Padding_left (Px (if ghost then 0. else 12.))));
    assert (has surface (Line_height (Percent 162.5)));
    assert (Option.is_none root.accessibility));
  List.iter [ P.Alignment.Start; End ] ~f:(fun alignment ->
    let view = B.create p ~alignment [] |> B.view in
    let root = View.Expert.describe view in
    assert (
      has
        root
        (match alignment with
         | Start -> Margin_right Auto
         | End -> Margin_left Auto)));
  let view =
    B.create
      p
      ~variant:Ghost
      ~style:(Style.create_exn [ Max_width (px 400.) ])
      ~content_style:(Style.create_exn [ Padding (px 5.); Radius 9. ])
      []
    |> B.view
  in
  assert (has (View.Expert.describe view) (Max_width (Px 400.)));
  assert (has (named view "content") (Padding_left (Px 5.)));
  assert (has (named view "content") (Top_left_radius 9.));
  print_endline
    "seven surfaces; Ghost metadata/full width; surface padding/border; optional \
     auto-margin alignment; root/content styles refine defaults";
  [%expect
    {| seven surfaces; Ghost metadata/full width; surface padding/border; optional auto-margin alignment; root/content styles refine defaults |}]
;;

let%expect_test
    "only typed Ghost metadata changes inherited message insets; explicit decisions win"
  =
  List.iter [ false; true ] ~f:(fun ghost ->
    List.iter [ false; true ] ~f:(fun typed ->
      List.iter [ None; Some false; Some true ] ~f:(fun explicit ->
        let bubble =
          B.create
            p
            ~variant:(if ghost then Ghost else Filled)
            ~alignment:End
            [ text "Body" ]
          |> fun bubble ->
          let metadata =
            Accessibility.create ~role:Group ~label:"Message bubble" () |> ok
          in
          let annotated = B.with_accessibility bubble metadata |> ok in
          assert (B.Variant.equal (B.variant annotated) (B.variant bubble));
          assert (Option.is_some (View.Expert.describe (B.view annotated)).accessibility);
          assert (
            Result.is_error
              (B.with_accessibility
                 annotated
                 (Accessibility.create ~role:Link ~label:"Invalid link" () |> ok)));
          annotated
        in
        let item =
          if typed
          then M.Content.Item.bubble ~key:(key "bubble") bubble
          else M.Content.Item.element ~key:(key "bubble") (B.view bubble)
        in
        let content = M.Content.create [ item ] |> ok in
        let view =
          M.create
            p
            ~alignment:Start
            ~content
            ~header:(M.Header.create ?content_inset:explicit [ text "Header" ])
            ~footer:(M.Footer.create ?content_inset:explicit [ text "Footer" ])
            ()
        in
        let expected = Option.value explicit ~default:(not (ghost && typed)) in
        List.iter [ "header"; "footer" ] ~f:(fun slot ->
          assert (Bool.equal (has (named view slot) (Padding_left (Px 12.))) expected));
        (* Message alignment must not rewrite the bubble's explicitly opposite edge. *)
        assert (has (named view "bubble") (Margin_left Auto)))));
  let item = M.Content.Item.element ~key:(key "duplicate") (text "A") in
  assert (Result.is_error (M.Content.create [ item; item ]));
  print_endline
    "typed Ghost alone suppresses inherited header/footer inset; explicit true/false \
     wins; independent bubble alignment; duplicate content keys rejected";
  [%expect
    {| typed Ghost alone suppresses inherited header/footer inset; explicit true/false wins; independent bubble alignment; duplicate content keys rejected |}]
;;

let%expect_test
    "message named slots retain separate parents and predictable avatar/footer overrides"
  =
  for mask = 0 to 15 do
    List.iter [ P.Alignment.Start; End ] ~f:(fun alignment ->
      let present bit = mask land bit <> 0 in
      let view =
        M.create
          p
          ~alignment
          ?avatar:(Option.some_if (present 1) (M.Avatar.create [ text "Avatar" ]))
          ?header:(Option.some_if (present 2) (M.Header.create [ text "Header" ]))
          ?content:
            (Option.some_if
               (present 4)
               (M.Content.create
                  [ M.Content.Item.element ~key:(key "item") (text "Body") ]
                |> ok))
          ?footer:(Option.some_if (present 8) (M.Footer.create [ text "Footer" ]))
          ()
      in
      let root = View.Expert.describe view in
      assert (List.length root.children = if present 8 then 2 else 1);
      let row = View.Expert.describe (List.hd_exn root.children) in
      assert (Option.equal Key.equal row.key (Some (key "row")));
      assert (List.length row.children = if present 1 then 2 else 1);
      if present 8
      then
        assert (
          Bool.equal
            (has
               (named view "footer")
               (match alignment with
                | Start -> Margin_left (Px 40.)
                | End -> Margin_right (Px 40.)))
            (present 1)))
  done;
  let view =
    M.create
      p
      ~alignment:Start
      ~avatar:(M.Avatar.create ~style:(Style.create_exn [ Width (px 48.) ]) [])
      ~footer:(M.Footer.create ~style:(Style.create_exn [ Margin_left (px 56.) ]) [])
      ()
  in
  assert (has (named view "footer") (Margin_left (Px 56.)));
  print_endline
    "32 optional-slot/alignment configurations; footer outside body row; avatar inset \
     only when present; caller margin overrides baseline";
  [%expect
    {| 32 optional-slot/alignment configurations; footer outside body row; avatar inset only when present; caller margin overrides baseline |}]
;;

let%expect_test
    "streaming and reaction/layout changes retain controls and fence removed events"
  =
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let r = Reconciler.create window in
  let commit view =
    let u = Reconciler.prepare r ~theme:Theme.default view |> ok in
    Reconciler.accept r u |> ok;
    match Reconciler.message u with
    | Some (Apply { operations; _ }) -> operations
    | None -> []
    | Some _ -> assert false
  in
  let view revision variant alignment side =
    let reactions =
      B.Reactions.create
        ~side
        ~alignment
        [ B.Reactions.Item.action
            ~key:(key "react")
            ~on_click:(fun () -> revision + 1000)
            "Like"
        ]
      |> ok
    in
    let bubble =
      B.create
        p
        ~variant
        ~reactions
        [ View.text ~key:(key "stream") (String.make (revision + 1) 'x')
        ; View.button ~key:(key "body-action") ~on_click:(fun () -> revision) "Copy"
        ]
    in
    let content =
      M.Content.create [ M.Content.Item.bubble ~key:(key "bubble") bubble ] |> ok
    in
    M.create
      p
      ~key:(key "message")
      ~alignment
      ~content
      ?avatar:(Option.some_if (revision % 2 = 0) (M.Avatar.create [ text "ME" ]))
      ~header:(M.Header.create [ text "Header" ])
      ?footer:(Option.some_if (revision % 3 = 0) (M.Footer.create [ text "Footer" ]))
      ()
  in
  let controls =
    commit (Some (view 0 Filled Start Bottom))
    |> List.filter_map ~f:(function
      | W.Op.Create (n, Button, _, Some h) -> Some (n, h)
      | _ -> None)
  in
  assert (List.length controls = 2);
  let count = ref 0 in
  List.iter variants ~f:(fun variant ->
    List.iter [ P.Alignment.Start; End ] ~f:(fun alignment ->
      List.iter [ B.Reactions.Side.Top; Bottom ] ~f:(fun side ->
        incr count;
        let next = view !count variant alignment side in
        let ops = commit (Some next) in
        List.iter ops ~f:(function
          | W.Op.Create (_, Button, _, _) -> failwith "control remounted"
          | Remove n ->
            assert (
              not
                (List.exists controls ~f:(fun (c, _) -> Gpuio_protocol.Node_id.equal c n)))
          | _ -> ());
        let values =
          List.map controls ~f:(fun (n, h) ->
            Reconciler.dispatch r (W.Event.Press (window, n, h, 1L)) |> Option.value_exn)
          |> List.sort ~compare:Int.compare
        in
        assert (List.equal Int.equal values [ !count; !count + 1000 ]);
        assert (List.is_empty (commit (Some next))))));
  ignore (commit None : W.Op.t list);
  List.iter controls ~f:(fun (n, h) ->
    assert (Option.is_none (Reconciler.dispatch r (W.Event.Press (window, n, h, 1L)))));
  printf
    "%d streaming/variant/alignment/reaction-side transitions; retained current \
     callbacks; optional avatar/footer changes; equal snapshots idle; late actions \
     rejected\n"
    !count;
  [%expect
    {| 28 streaming/variant/alignment/reaction-side transitions; retained current callbacks; optional avatar/footer changes; equal snapshots idle; late actions rejected |}]
;;
