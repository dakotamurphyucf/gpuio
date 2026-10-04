open Core
open Gpuio
module G = Style.Grid_location
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let key = Key.of_string_exn
let line value = G.Edge.Line (G.Line.of_int_exn value)
let span value = G.Edge.Span (G.Span.of_int_exn value)
let axis start end_ = G.Axis.create ~start ~end_

let item ?column ?label_width name =
  Form.Item.create ~key:(key name) ?column ?label_width [] |> ok
;;

let form column = Form.create ~columns:4 [ item ~column "a" ]

let%expect_test
    "form columns resolve native signed lines and spans within explicit bounds"
  =
  let cases =
    [ axis (line 1) (line (-1)), true
    ; axis (line 3) (line 1), true
    ; axis (line 2) (line 2), true
    ; axis (line 5) (line 5), false
    ; axis (line (-5)) (line (-1)), true
    ; axis (line (-6)) (line 1), false
    ; axis (line 4) G.Edge.Auto, true
    ; axis (line 5) G.Edge.Auto, false
    ; axis G.Edge.Auto (line 1), false
    ; axis G.Edge.Auto (line 5), true
    ; axis (line 2) (span 3), true
    ; axis (line 2) (span 4), false
    ; axis (span 3) (line 4), true
    ; axis (span 4) (line 4), false
    ; axis (span 2) (span 1024), true
    ; axis (span 5) (span 1), false
    ; axis G.Edge.Auto (span 4), true
    ; axis G.Edge.Auto (span 5), false
    ; axis (span 4) G.Edge.Auto, true
    ; G.Axis.auto, true
    ]
  in
  List.iter cases ~f:(fun (column, accepted) ->
    assert (Bool.equal (Result.is_ok (form column)) accepted));
  List.iter [ 0; -1; 1025; Int.max_value ] ~f:(fun columns ->
    assert (Result.is_error (Form.create ~columns [])));
  List.iter [ 1; 1024 ] ~f:(fun columns -> ignore (Form.create ~columns [] |> ok));
  assert (Result.is_error (Form.create [ item "same"; item "same" ]));
  List.iter
    [ Length.auto; Length.px_exn (-1.); Length.percent_exn (-1.) ]
    ~f:(fun label_width ->
      assert (Result.is_error (Form.Item.create ~key:(key "bad") ~label_width []));
      assert (Result.is_error (Form.create ~label_width [])));
  let two = item ~column:(G.Axis.span (G.Span.of_int_exn 2)) "two" in
  ignore (Form.create ~columns:2 [ two ] |> ok);
  assert (Result.is_error (Form.create ~columns:1 [ two ]));
  print_endline
    "20 placement cases; column/width/key bounds; shrinking columns rejects overflow";
  [%expect
    {| 20 placement cases; column/width/key bounds; shrinking columns rejects overflow |}]
;;

let rec descriptions view =
  let d = View.Expert.describe view in
  d :: List.concat_map d.children ~f:descriptions
;;

let named view name =
  List.find_exn (descriptions view) ~f:(fun d ->
    Option.equal Key.equal d.key (Some (key name)))
;;

let fields style =
  match Style.Expert.to_wire style ~theme:Theme.default |> ok with
  | [ W.Style.Fields fields ] -> fields
  | styles -> raise_s [%message "expected base field style" (styles : W.Style.t list)]
;;

let has view name field =
  List.mem (fields (named view name).style) field ~equal:W.Field.equal
;;

let%expect_test "form visual overrides preserve semantic fields and structural layout" =
  let metadata =
    Form.Field.create ~label:"Account" ~help:"Private" ~error:"Required" ~required:true ()
    |> ok
  in
  let control = View.checkbox ~state:Unchecked ~on_toggle:(fun () -> ()) "Original" in
  let rich =
    Form.Item.of_field
      metadata
      ~key:(key "rich")
      ~label_indent:false
      ~label:(View.text "Rich display")
      ~description:(View.text "Visual explanation")
      ~label_width:(Length.px_exn 90.)
      ~layout:Horizontal
      ~style:
        (Style.create_exn
           [ Direction Column
           ; Position Absolute
           ; Grid_location (G.create ~column:G.Axis.full ())
           ])
      ~control
      ()
    |> ok
  in
  let absent = Form.Item.create ~key:(key "absent") [] |> ok in
  let view =
    Form.create
      ~columns:2
      ~layout:Horizontal
      ~grid_style:(Style.create_exn [ Display Flex; Grid_columns 9 ])
      [ rich; absent ]
    |> ok
  in
  assert (has view "grid" (Grid_columns 2L));
  assert (has view "grid" (Display 2L));
  assert (has view "rich" (Position 0L));
  assert (has view "rich" (Direction 0L));
  assert (has view "rich" (Grid_location (G.Expert.to_wire (G.create ()))));
  let rich_view = List.hd_exn (named view "grid").children in
  assert (has rich_view "label" (Width (Px 90.)));
  assert (
    List.exists (descriptions rich_view) ~f:(fun d -> String.equal d.text "Rich display"));
  let semantic =
    List.find_map_exn (descriptions rich_view) ~f:(fun d ->
      Option.bind d.accessibility ~f:(fun a -> (Accessibility.Expert.to_wire a).field))
  in
  assert (String.equal semantic.label "Account");
  assert semantic.required;
  assert (Option.equal String.equal semantic.help (Some "Private"));
  assert (Option.equal String.equal semantic.error (Some "Required"));
  let absent_view = List.nth_exn (named view "grid").children 1 in
  assert (has absent_view "label" (Width (Px 160.)));
  let no_indent = Form.create ~layout:Horizontal ~label_indent:false [ absent ] |> ok in
  assert (
    not
      (List.exists (descriptions no_indent) ~f:(fun d ->
         Option.equal Key.equal d.key (Some (key "label")))));
  assert (
    Result.is_error
      (Form.Item.of_field metadata ~key:(key "bad") ~control:(View.column []) ()));
  print_endline
    "rich slots keep native name/help/error; present labels survive no-indent; typed \
     structure wins";
  [%expect
    {| rich slots keep native name/help/error; present labels survive no-indent; typed structure wins |}]
;;

let%expect_test
    "collection changes retain native editor identity and never resend its initial draft"
  =
  let config = Text_input.Config.create ~mode:Single_line ~label:"Name" () |> ok in
  let control =
    View.text_input
      ~controller:(key "editor")
      ~initial_text:"draft"
      ~config
      ~on_event:(fun _ -> ())
      ()
    |> ok
  in
  let metadata ?error () = Form.Field.create ~label:"Name" ?error () |> ok in
  let view ~columns ~layout ~reverse ~annotated ~footer ~hidden =
    let entry =
      Form.Item.of_field
        (metadata ?error:(if annotated then Some "Invalid" else None) ())
        ~key:(key "name")
        ~column:G.Axis.full
        ~style:(Style.create_exn (if hidden then [ Display Hidden ] else []))
        ~control
        ()
      |> ok
    in
    let other = item "other" in
    Form.create
      ~columns
      ~layout
      ?footer:(if footer then Some (View.text "Save") else None)
      (if reverse then [ other; entry ] else [ entry; other ])
    |> ok
  in
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let reconciler = Reconciler.create window in
  let commit view =
    let update = Reconciler.prepare reconciler ~theme:Theme.default (Some view) |> ok in
    Reconciler.accept reconciler update |> ok;
    match Reconciler.message update with
    | Some (Apply tx) -> tx.operations
    | _ -> []
  in
  let initial =
    commit
      (view
         ~columns:1
         ~layout:Vertical
         ~reverse:false
         ~annotated:false
         ~footer:false
         ~hidden:false)
  in
  let node =
    List.find_map_exn initial ~f:(function
      | W.Op.Create (id, Input, _, _) -> Some id
      | _ -> None)
  in
  List.iteri
    [ 2, Form.Layout.Horizontal; 4, Vertical; 1, Horizontal; 3, Vertical ]
    ~f:(fun index (columns, layout) ->
      let operations =
        commit
          (view
             ~columns
             ~layout
             ~reverse:(index mod 2 = 0)
             ~annotated:(index mod 2 = 0)
             ~footer:(index mod 2 = 1)
             ~hidden:(index = 2))
      in
      List.iter operations ~f:(function
        | W.Op.Remove id when Gpuio_protocol.Node_id.equal id node ->
          failwith "editor removed"
        | Create (_, Input, _, _) -> failwith "editor recreated"
        | Set_text (id, _) when Gpuio_protocol.Node_id.equal id node ->
          failwith "draft reset"
        | _ -> ()));
  let removed = commit (Form.create [] |> ok) in
  assert (
    List.exists removed ~f:(function
      | W.Op.Remove id -> Gpuio_protocol.Node_id.equal id node
      | _ -> false));
  let remounted =
    commit
      (view
         ~columns:1
         ~layout:Vertical
         ~reverse:false
         ~annotated:false
         ~footer:false
         ~hidden:false)
  in
  assert (
    List.exists remounted ~f:(function
      | W.Op.Create (id, Input, _, _) -> not (Gpuio_protocol.Node_id.equal id node)
      | _ -> false));
  Reconciler.close reconciler;
  print_endline
    "columns/orientation/reorder/error/footer/hiding retain editor; removal and remount \
     replace it";
  [%expect
    {| columns/orientation/reorder/error/footer/hiding retain editor; removal and remount replace it |}]
;;

let%expect_test
    "form shared policy and per-item overrides refine slots without resizing controls"
  =
  let declared =
    View.column
      ~key:(key "declared-control")
      ~style:(Style.create_exn [ Height (Length.px_exn 37.) ])
      []
  in
  let make ?size ?layout ?label_width ?alignment ?(label_style = Style.empty) () =
    Form.Item.create
      ~key:(key "item")
      ?size
      ?layout
      ?label_width
      ?alignment
      ~label_style
      ~label:(View.text "Label")
      [ declared ]
    |> ok
  in
  List.iter
    [ Form.Size.XSmall, 4., 12.; Small, 6., 13.; Medium, 8., 14.; Large, 12., 16. ]
    ~f:(fun (size, gap, font) ->
      let inherited =
        Form.create ~size ~layout:Horizontal ~label_width:(Length.px_exn 120.) [ make () ]
        |> ok
      in
      assert (has inherited "item" (Row_gap (Px gap)));
      assert (has inherited "label" (Font_size font));
      assert (has inherited "label" (Width (Px 120.)));
      assert (has inherited "declared-control" (Height (Px 37.)));
      let overridden =
        Form.create
          ~size
          ~layout:Horizontal
          ~label_width:(Length.px_exn 120.)
          ~label_style:(Style.create_exn [ Font_weight 700; Font_size 19. ])
          [ make
              ~size:Small
              ~layout:Vertical
              ~label_width:(Length.px_exn 90.)
              ~alignment:Center
              ~label_style:(Style.create_exn [ Font_size 21. ])
              ()
          ]
        |> ok
      in
      assert (has overridden "item" (Row_gap (Px 6.)));
      assert (has overridden "item" (Direction 1L));
      assert (has overridden "item" (Align_items 4L));
      assert (has overridden "label" (Font_size 21.));
      assert (has overridden "label" (Font_weight 700L));
      assert (
        not
          (List.exists (fields (named overridden "label").style) ~f:(function
             | W.Field.Width _ -> true
             | _ -> false)));
      assert (has overridden "declared-control" (Height (Px 37.))));
  print_endline
    "four size policies; item layout/size and slot overrides; control height unchanged";
  [%expect
    {| four size policies; item layout/size and slot overrides; control height unchanged |}]
;;

let%expect_test
    "rich form slots retire queued actions while surviving controls use current closures"
  =
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let reconciler = Reconciler.create window in
  let commit view =
    let update = Reconciler.prepare reconciler ~theme:Theme.default view |> ok in
    Reconciler.accept reconciler update |> ok;
    match Reconciler.message update with
    | Some (Apply transaction) -> transaction.operations
    | None -> []
    | Some message -> raise_s [%message "unexpected message" (message : W.Message.t)]
  in
  let view mask ~columns ~layout revision =
    let slot index name =
      Option.some_if
        (mask land (1 lsl index) <> 0)
        (View.button ~on_click:(fun () -> name) name)
    in
    let content = View.checkbox ~state:Checked ~on_toggle:(fun () -> revision) "Body" in
    let item =
      Form.Item.create
        ~key:(key "item")
        ?label:(slot 0 "label")
        ?description:(slot 1 "description")
        ?error:(slot 2 "error")
        [ content ]
      |> ok
    in
    Form.create ~columns ~layout ?footer:(slot 3 "footer") [ item ] |> ok
  in
  let initial = commit (Some (view 0 ~columns:1 ~layout:Vertical "original")) in
  let body_node, body_handler =
    List.find_map_exn initial ~f:(function
      | W.Op.Create (node, Checkbox, _, Some handler) -> Some (node, handler)
      | _ -> None)
  in
  let body_event = W.Event.Press (window, body_node, body_handler, 1L) in
  let slot_events = ref [] in
  List.iter [ Form.Layout.Vertical; Horizontal ] ~f:(fun layout ->
    List.iter [ 1; 3 ] ~f:(fun columns ->
      List.iter
        (List.range 0 16 @ List.range 0 16 |> List.rev)
        ~f:(fun mask ->
          let current = view mask ~columns ~layout "latest" in
          let operations = commit (Some current) in
          List.iter operations ~f:(function
            | W.Op.Remove node when Gpuio_protocol.Node_id.equal node body_node ->
              failwith "body removed"
            | Create (_, Checkbox, _, _) -> failwith "body recreated"
            | Create (node, Button, label, Some handler) ->
              let event = W.Event.Press (window, node, handler, 1L) in
              assert (
                Option.equal
                  String.equal
                  (Reconciler.dispatch reconciler event)
                  (Some label));
              slot_events := event :: !slot_events
            | _ -> ());
          assert (
            Option.equal
              String.equal
              (Reconciler.dispatch reconciler body_event)
              (Some "latest"));
          List.iter !slot_events ~f:(fun event ->
            Option.iter (Reconciler.dispatch reconciler event) ~f:(fun name ->
              let bit =
                match name with
                | "label" -> 0
                | "description" -> 1
                | "error" -> 2
                | "footer" -> 3
                | _ -> failwith "unexpected slot action"
              in
              assert (mask land (1 lsl bit) <> 0)));
          assert (List.is_empty (commit (Some current))))));
  ignore (commit None : W.Op.t list);
  List.iter (body_event :: !slot_events) ~f:(fun event ->
    assert (Option.is_none (Reconciler.dispatch reconciler event)));
  Reconciler.close reconciler;
  print_endline
    "128 slot/layout/column transitions; latest body closure; removed slot actions \
     fenced; repeats idle; full unmount fenced";
  [%expect
    {| 128 slot/layout/column transitions; latest body closure; removed slot actions fenced; repeats idle; full unmount fenced |}]
;;
