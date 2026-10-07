open Core
open Gpuio
module Rows = Presentation.Chart_inspection
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let key = Key.of_string_exn
let color = Color.rgb_exn 0x22aabb
let style = Style.create_exn

let%expect_test "inspection row keys are explicit and duplicates fail before mounting" =
  let row = Rows.Row.text ~key:(key "tokens") ~color ~label:"Tokens" ~value:"1,024" in
  print_s
    [%sexp
      (Rows.view Presentation.Appearance.light [ row; row ] |> Result.is_error : bool)];
  List.iter [ Presentation.Appearance.light; Presentation.Appearance.dark ] ~f:(fun p ->
    List.iter
      [ None; Some (View.text "名前 · العربية · 👩🏽‍💻") ]
      ~f:(fun title -> ignore (Rows.view p ?title [] |> ok : unit View.t)));
  print_endline "empty rows and optional rich title accepted in both appearances";
  [%expect
    {|
    true
    empty rows and optional rich title accepted in both appearances
    |}]
;;

let%expect_test "rich rows keep native identity and current callbacks inside a chart" =
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let owner = Chart_resource.Expert.Owner.create () in
  let source = Gpuio_protocol.Resource_id.create ~slot:0L ~generation:1L |> ok in
  let data = Chart_resource.Expert.handle ~owner source in
  let config = Chart.Config.create ~data () |> ok in
  let reconciler = Reconciler.create ~chart_owner:owner window in
  let commit view =
    let update = Reconciler.prepare reconciler ~theme:Theme.default view |> ok in
    Reconciler.accept reconciler update |> ok;
    match Reconciler.message update with
    | Some (Apply { operations; _ }) -> operations
    | Some _ -> assert false
    | None -> []
  in
  let make revision =
    let rows =
      List.mapi [ "title"; "rows"; "label" ] ~f:(fun index name ->
        Rows.Row.create
          ~key:(key name)
          ~color
          ~label:(View.text (name ^ " 名前"))
          ~value:
            (View.button
               ~key:(key "action")
               ~on_click:(fun () -> (revision * 10) + index)
               "Inspect"))
    in
    let content =
      Rows.view
        (if revision % 2 = 0
         then Presentation.Appearance.light
         else Presentation.Appearance.dark)
        ~style:(style [ Font_size (12. +. Float.of_int (revision % 3)) ])
        ~row_style:(style [ Gap (Length.px_exn (8. +. Float.of_int (revision % 4))) ])
        ?title:(Option.some_if (revision % 3 = 0) (View.text "Inspection title"))
        (if revision % 2 = 0 then rows else List.rev rows)
      |> ok
    in
    let inspection_content =
      Chart_inspection_content.create
        [ Chart_inspection_content.Entry.create
            ~target:
              (Chart_inspection_content.Target.slice
                 (Chart_data.Datum_id.of_int64 7L |> ok))
            content
        ]
      |> ok
    in
    View.chart ~inspection_content config
  in
  let controls =
    commit (Some (make 0))
    |> List.filter_map ~f:(function
      | W.Op.Create (node, Button, _, Some handler) -> Some (node, handler)
      | _ -> None)
  in
  assert (List.length controls = 3);
  for revision = 1 to 12 do
    let view = make revision in
    List.iter (commit (Some view)) ~f:(function
      | W.Op.Create (_, Button, _, _) -> failwith "row control remounted"
      | Set_chart _ -> failwith "presentation-only change updated chart metadata"
      | Remove node ->
        assert (
          not
            (List.exists controls ~f:(fun (old, _) ->
               Gpuio_protocol.Node_id.equal old node)))
      | _ -> ());
    let actions =
      List.map controls ~f:(fun (node, handler) ->
        Reconciler.dispatch reconciler (W.Event.Press (window, node, handler, 1L))
        |> Option.value_exn)
      |> List.sort ~compare:Int.compare
    in
    assert (
      List.equal
        Int.equal
        actions
        [ revision * 10; (revision * 10) + 1; (revision * 10) + 2 ]);
    assert (List.is_empty (commit (Some view)))
  done;
  ignore (commit None : W.Op.t list);
  List.iter controls ~f:(fun (node, handler) ->
    assert (
      Option.is_none
        (Reconciler.dispatch reconciler (W.Event.Press (window, node, handler, 1L)))));
  Reconciler.close reconciler;
  print_endline
    "12 reorder/title/theme/style changes retain three controls, refresh callbacks, \
     avoid chart metadata changes and reject callbacks after unmount";
  [%expect
    {| 12 reorder/title/theme/style changes retain three controls, refresh callbacks, avoid chart metadata changes and reject callbacks after unmount |}]
;;
