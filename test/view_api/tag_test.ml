open Core
open Gpuio
module T = Presentation.Tag
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let key = Key.of_string_exn
let px = Length.px_exn
let appearance = Presentation.Appearance.dark

let custom =
  T.Palette.create
    ~background:(Color.rgb_exn 0x543c9d)
    ~foreground:(Color.rgb_exn 0xffffff)
    ~border:(Color.rgb_exn 0x917bd2)
;;

let variants =
  [ T.Variant.Primary; Secondary; Danger; Success; Warning; Info; Custom custom ]
;;

let fields ?(theme = Theme.default) view =
  let d = View.Expert.describe view in
  Style.Expert.to_wire d.style ~theme
  |> ok
  |> List.concat_map ~f:(function
    | W.Style.Fields f -> f
    | _ -> [])
;;

let contains fields field = List.mem fields field ~equal:W.Field.equal

let hover view =
  Style.Expert.to_wire (View.Expert.describe view).style ~theme:Theme.default
  |> ok
  |> List.find_map ~f:(function
    | W.Style.State (2L, f) -> Some f
    | _ -> None)
  |> Option.value ~default:[]
;;

let%expect_test "semantic and custom palettes map filled and outlined ink independently" =
  List.iter
    [ T.Variant.Primary, 0xa3b5ffffL, 0xa3b5ffffL, 0xa3b5ffffL
    ; Secondary, 0x272e3bffL, 0xe5eaf2ffL, 0x3e485bffL
    ; Danger, 0xffa0afffL, 0xffa0afffL, 0xffa0afffL
    ; Success, 0x8bd6afffL, 0x8bd6afffL, 0x8bd6afffL
    ; Warning, 0xf1c784ffL, 0xf1c784ffL, 0xf1c784ffL
    ; Info, 0xa3b5ffffL, 0xa3b5ffffL, 0xa3b5ffffL
    ; Custom custom, 0x543c9dffL, 0xffffffffL, 0x917bd2ffL
    ]
    ~f:(fun (variant, bg, ink, border) ->
      List.iter [ false; true ] ~f:(fun outline ->
        let f = fields (T.create appearance ~variant ~outline []) in
        let foreground =
          match variant with
          | Custom _ -> ink
          | Secondary -> if outline then 0xa3aebeffL else ink
          | Primary | Info | Danger | Success | Warning ->
            if outline then ink else 0x161b24ffL
        in
        assert (contains f (Foreground (Rgba foreground)));
        assert (contains f (Border_color (Rgba border)));
        assert (
          contains f (Background (Solid (Rgba (if outline then 0xffffff00L else bg)))))));
  print_endline "seven variants × filled/outline; custom foreground and border retained";
  [%expect {| seven variants × filled/outline; custom foreground and border retained |}]
;;

let%expect_test "size aliases and direct rich children have no forced text or wrapper" =
  List.iter [ T.Size.XSmall; Small; Medium; Large ] ~f:(fun size ->
    let view = T.create appearance ~size [] in
    assert (List.is_empty (View.Expert.describe view).children);
    assert (Option.is_none (View.Expert.describe view).accessibility);
    let f = fields view in
    let small =
      match size with
      | XSmall | Small -> true
      | Medium | Large -> false
    in
    assert (contains f (Padding_left (Px (if small then 6. else 10.))));
    assert (contains f (Padding_top (Px (if small then 2. else 4.))));
    assert (contains f (Top_left_radius (if small then 4. else 8.)));
    assert (contains f (Font_size 12.)));
  let view =
    T.create
      appearance
      ~style:(Style.create_exn [ Radius 16.; Gap (px 3.) ])
      [ View.button
          ~key:(key "action")
          ~style:(Style.create_exn [ Grow 1. ])
          ~on_click:(fun () -> ())
          "Remove"
      ]
  in
  let d = View.Expert.describe view in
  let child = View.Expert.describe (List.hd_exn d.children) in
  assert (List.length d.children = 1);
  assert (View.Expert.Kind.equal child.kind Button);
  assert (Option.equal Key.equal child.key (Some (key "action")));
  assert (contains (fields (List.hd_exn d.children)) (Grow 1.));
  assert (contains (fields view) (Top_left_radius 16.));
  print_endline
    "two size groups; empty root has no label/semantics; keyed action remains direct \
     flex child; custom radius";
  [%expect
    {| two size groups; empty root has no label/semantics; keyed action remains direct flex child; custom radius |}]
;;

let%expect_test "native hover defaults permit explicit override or unset" =
  let base = Style.create_exn [ Opacity 0.65 ] in
  let regular = T.create appearance ~style:base [] in
  assert (contains (fields regular) (Opacity 0.65));
  assert (contains (hover regular) (Opacity 0.9));
  let override = Style.with_state_exn base Hovered [ Opacity 0.4 ] in
  assert (contains (hover (T.create appearance ~style:override [])) (Opacity 0.4));
  let unset = Style.unset base ~state:Hovered Opacity in
  let view = T.create appearance ~style:unset [] in
  let f = fields view in
  assert (contains f (Opacity 0.65));
  assert (
    not
      (List.exists (hover view) ~f:(function
         | W.Field.Opacity _ -> true
         | _ -> false)));
  let token = Color.token_exn "tag-color" in
  let palette = T.Palette.create ~background:token ~foreground:token ~border:token in
  let view = T.create appearance ~variant:(Custom palette) [] in
  assert (
    Result.is_error
      (Style.Expert.to_wire (View.Expert.describe view).style ~theme:Theme.default));
  let theme =
    Theme.create [ "tag-color", Color.rgba ~red:18 ~green:52 ~blue:86 ~alpha:128 |> ok ]
    |> ok
  in
  assert (contains (fields ~theme view) (Background (Solid (Rgba 0x12345680L))));
  print_endline
    "hover 0.9 absolute; override 0.4; unset inherits base 0.65; custom palette tokens \
     resolved with alpha and missing-token rejection";
  [%expect
    {| hover 0.9 absolute; override 0.4; unset inherits base 0.65; custom palette tokens resolved with alpha and missing-token rejection |}]
;;

let%expect_test
    "palette and layout changes preserve rich controls and fence removed actions"
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
  let view revision ~variant ~size ~outline ~reversed =
    let children =
      [ View.text ~key:(key "label") "State"
      ; View.button ~key:(key "action") ~on_click:(fun () -> revision) "Remove"
      ]
    in
    T.create
      appearance
      ~key:(key "tag")
      ~variant
      ~size
      ~outline
      (if reversed then List.rev children else children)
  in
  let node, handler =
    commit (Some (view 0 ~variant:Secondary ~size:Medium ~outline:false ~reversed:false))
    |> List.find_map_exn ~f:(function
      | W.Op.Create (n, Button, _, Some h) -> Some (n, h)
      | _ -> None)
  in
  let count = ref 0 in
  List.iter variants ~f:(fun variant ->
    List.iter [ T.Size.XSmall; Small; Medium; Large ] ~f:(fun size ->
      List.iter [ false; true ] ~f:(fun outline ->
        incr count;
        let next = view !count ~variant ~size ~outline ~reversed:(!count % 2 = 0) in
        let ops = commit (Some next) in
        List.iter ops ~f:(function
          | W.Op.Create (_, Button, _, _) -> failwith "remounted action"
          | Remove n -> assert (not (Gpuio_protocol.Node_id.equal n node))
          | _ -> ());
        assert (
          Option.equal
            Int.equal
            (Reconciler.dispatch r (W.Event.Press (window, node, handler, 1L)))
            (Some !count));
        assert (List.is_empty (commit (Some next))))));
  ignore (commit None : W.Op.t list);
  assert (
    Option.is_none (Reconciler.dispatch r (W.Event.Press (window, node, handler, 1L))));
  printf
    "%d variant/size/outline/reorder transitions; retained current callback; equal \
     snapshots idle; removed action rejected\n"
    !count;
  [%expect
    {| 56 variant/size/outline/reorder transitions; retained current callback; equal snapshots idle; removed action rejected |}]
;;
