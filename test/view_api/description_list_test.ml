open Core
open Gpuio
module P = Presentation
module D = P.Description_list
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let key = Key.of_int
let style = Style.create_exn
let p = P.Appearance.dark
let describe = View.Expert.describe

let fields view =
  Style.Expert.to_wire (describe view).style ~theme:Theme.default
  |> ok
  |> List.concat_map ~f:(function
    | W.Style.Fields values -> values
    | _ -> [])
;;

let has view field = List.mem (fields view) field ~equal:W.Field.equal
let children view = (describe view).children

let role view =
  Option.bind (describe view).accessibility ~f:(fun a ->
    (Accessibility.Expert.to_wire a).role)
;;

let item ?(span = 1) n =
  D.Item.create
    ~key:(key n)
    ~span
    ~term:[ View.text "Term" ]
    ~definition:[ View.text "Value" ]
    ()
  |> ok
;;

let%expect_test "description invariants reject ambiguous packing before reconciliation" =
  List.iter [ Int.min_value; -1; 0; 11; Int.max_value ] ~f:(fun span ->
    assert (Result.is_error (D.Item.create ~key:(key 0) ~span ~term:[] ~definition:[] ()));
    assert (Result.is_error (D.create p ~columns:span [])));
  for columns = 1 to 10 do
    for span = 1 to 10 do
      assert (
        Bool.equal (Result.is_ok (D.create p ~columns [ item ~span 0 ])) (span <= columns))
    done
  done;
  assert (Result.is_error (D.create p [ item 0; D.Item.separator ~key:(key 0) () ]));
  List.iter [ P.Axis.Horizontal; Vertical ] ~f:(fun axis ->
    List.iter
      [ Length.auto; Length.px_exn (-1.); Length.percent_exn (-0.1) ]
      ~f:(fun label_width -> assert (Result.is_error (D.create p ~axis ~label_width []))));
  assert (Result.is_ok (D.create p ~label_width:(Length.px_exn 0.) []));
  print_endline
    "columns/spans 1..10; oversized span, duplicate item/separator key and \
     indefinite/negative label widths rejected";
  [%expect
    {| columns/spans 1..10; oversized span, duplicate item/separator key and indefinite/negative label widths rejected |}]
;;

let%expect_test "source packing supplies row boundaries without reparenting rich entries" =
  let view =
    D.create p (List.mapi [ 1; 2; 1; 1; 1; 3; 1 ] ~f:(fun n span -> item ~span n)) |> ok
  in
  let entries = children view in
  assert (List.length entries = 7);
  assert (
    Option.equal
      Gpuio_protocol.Accessibility_wire.Role.equal
      (role view)
      (Some Description_list));
  List.iteri entries ~f:(fun n entry ->
    assert (Option.equal Key.equal (describe entry).key (Some (key n)));
    assert (has entry (Border_bottom_width (if n = 6 then 0. else 1.)));
    let term, definition =
      match children entry with
      | [ a; b ] -> a, b
      | _ -> assert false
    in
    assert (
      has
        term
        (Border_left_width (if List.mem [ 0; 2; 5; 6 ] n ~equal:Int.equal then 0. else 1.)));
    assert (
      Option.equal Gpuio_protocol.Accessibility_wire.Role.equal (role term) (Some Term));
    assert (
      Option.equal
        Gpuio_protocol.Accessibility_wire.Role.equal
        (role definition)
        (Some Definition)));
  let separated =
    D.create
      p
      [ D.Item.separator ~key:(key 0) ()
      ; item 1
      ; D.Item.separator ~key:(key 2) ()
      ; D.Item.separator ~key:(key 3) ()
      ]
    |> ok
  in
  List.iteri (children separated) ~f:(fun n entry ->
    if n <> 1
    then (
      assert (
        Option.equal
          Gpuio_protocol.Accessibility_wire.Role.equal
          (role entry)
          (Some Separator));
      assert (has entry (Height (Px (if n = 3 then 8. else 9.))))));
  print_endline
    "reference rows [1,2] [1,1,1] [3] [1]; direct keyed entries, ordered Term/Definition \
     slots; leading/consecutive separators occupy rows";
  [%expect
    {| reference rows [1,2] [1,1,1] [3] [1]; direct keyed entries, ordered Term/Definition slots; leading/consecutive separators occupy rows |}]
;;

let%expect_test "description styles refine both axes and size groups" =
  List.iter [ P.Axis.Horizontal; Vertical ] ~f:(fun axis ->
    List.iter [ false; true ] ~f:(fun bordered ->
      List.iter
        [ D.Size.XSmall, 2., 4., 2.
        ; Small, 2., 4., 2.
        ; Medium, 4., 8., 4.
        ; Large, 8., 12., 6.
        ]
        ~f:(fun (size, gap, x, y) ->
          let root =
            D.create
              p
              ~axis
              ~size
              ~bordered
              ~label_width:(Length.percent_exn 25.)
              [ item 0 ]
            |> ok
          in
          let entry = List.hd_exn (children root) in
          let term = List.hd_exn (children entry) in
          assert (has root (Row_gap (Px (if bordered then 0. else gap))));
          assert (has term (Padding_left (Px (if bordered then x else 0.))));
          assert (has term (Padding_top (Px (if bordered then y else 0.))));
          assert (
            Bool.equal (has term (Width (Percent 25.))) (P.Axis.equal axis Horizontal));
          assert (Bool.equal (has root (Border_top_width 1.)) bordered))));
  let custom =
    D.Item.create
      ~key:(key 0)
      ~style:(style [ Border_bottom_width 4. ])
      ~term_style:(style [ Width (Length.px_exn 33.); Padding (Length.px_exn 9.) ])
      ~definition_style:(style [ Padding (Length.px_exn 11.) ])
      ~term:[]
      ~definition:[]
      ()
    |> ok
  in
  let root = D.create p ~style:(style [ Radius 2. ]) [ custom ] |> ok in
  let entry = List.hd_exn (children root) in
  let term, definition =
    match children entry with
    | [ a; b ] -> a, b
    | _ -> assert false
  in
  assert (has root (Top_left_radius 2.));
  assert (has entry (Border_bottom_width 4.));
  assert (has term (Width (Px 33.)) && has term (Padding_left (Px 9.)));
  assert (has definition (Padding_top (Px 11.)));
  print_endline
    "16 axis/border/size cases; XS=S; unbordered zero padding; horizontal definite \
     width; root/item/slot overrides";
  [%expect
    {| 16 axis/border/size cases; XS=S; unbordered zero padding; horizontal definite width; root/item/slot overrides |}]
;;

let%expect_test "description reflow retains rich controls and current callbacks" =
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let reconciler = Reconciler.create window in
  let commit view =
    let update = Reconciler.prepare reconciler ~theme:Theme.default view |> ok in
    Reconciler.accept reconciler update |> ok;
    match Reconciler.message update with
    | Some (Apply { operations; _ }) -> operations
    | None -> []
    | Some _ -> assert false
  in
  let make revision columns =
    let entries =
      List.init 7 ~f:(fun n ->
        let button name offset =
          View.button
            ~key:(Key.of_string_exn name)
            ~on_click:(fun () -> (revision * 100) + (n * 2) + offset)
            name
        in
        D.Item.create
          ~key:(key n)
          ~span:(if n = 3 then columns else 1)
          ~term:[ button "term-action" 0 ]
          ~definition:
            [ View.text (String.make (revision + 1) 'x'); button "definition-action" 1 ]
          ()
        |> ok)
    in
    D.create
      p
      ~columns
      ~axis:(if revision % 2 = 0 then Horizontal else Vertical)
      ~bordered:(revision % 3 = 0)
      ((if revision % 2 = 0 then entries else List.rev entries)
       @ if revision % 4 = 0 then [ D.Item.separator ~key:(key 10) () ] else [])
    |> ok
  in
  let controls =
    commit (Some (make 0 3))
    |> List.filter_map ~f:(function
      | W.Op.Create (node, Button, _, Some handler) -> Some (node, handler)
      | _ -> None)
  in
  assert (List.length controls = 14);
  for revision = 1 to 40 do
    let next = make revision (1 + (revision % 10)) in
    List.iter (commit (Some next)) ~f:(function
      | W.Op.Create (_, Button, _, _) -> failwith "description control remounted"
      | Remove node ->
        assert (
          not
            (List.exists controls ~f:(fun (old, _) ->
               Gpuio_protocol.Node_id.equal old node)))
      | _ -> ());
    let values =
      List.map controls ~f:(fun (node, handler) ->
        Reconciler.dispatch reconciler (W.Event.Press (window, node, handler, 1L))
        |> Option.value_exn)
      |> List.sort ~compare:Int.compare
    in
    assert (List.equal Int.equal values (List.init 14 ~f:(fun n -> (revision * 100) + n)));
    assert (List.is_empty (commit (Some next)))
  done;
  ignore (commit None : W.Op.t list);
  List.iter controls ~f:(fun (node, handler) ->
    assert (
      Option.is_none
        (Reconciler.dispatch reconciler (W.Event.Press (window, node, handler, 1L)))));
  print_endline
    "40 columns/span/axis/order/border/separator/text changes preserve 14 controls and \
     latest callbacks; equal snapshots idle, retired events rejected";
  [%expect
    {| 40 columns/span/axis/order/border/separator/text changes preserve 14 controls and latest callbacks; equal snapshots idle, retired events rejected |}]
;;
