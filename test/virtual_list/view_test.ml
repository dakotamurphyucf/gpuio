open Core
open Gpuio
module W = Gpuio_protocol.Wire
module L = Gpuio_protocol.List_wire
module R = Reconciler

let key = Key.of_string_exn
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn
let config = Virtual_list.Config.create ~height:(Estimated 80.) () |> Or_error.ok_exn
let order keys = Virtual_list.Order.create (List.map keys ~f:key) |> Or_error.ok_exn

let managed ?(invalidated = []) ?(invalidation_revision = 0L) ?scroll source rows =
  View.Expert.managed_virtual_list
    ~config
    ~order:source
    ~invalidated:(List.map invalidated ~f:key)
    ~invalidation_revision
    ?scroll
    ~on_viewport:Fn.id
    ~on_retain:(fun _ -> failwith "unexpected retention")
    (List.map rows ~f:(fun (k, text) -> key k, View.text text))
  |> Or_error.ok_exn
;;

let prepare r view = R.prepare r ~theme:Theme.default (Some view) |> Or_error.ok_exn

let operations update =
  match R.message update with
  | Some (W.Message.Apply tx) -> tx.operations
  | None -> []
  | Some _ -> assert false
;;

let accept r update = R.accept r update |> Or_error.ok_exn

let source_order ops =
  List.find_map_exn ops ~f:(function
    | W.Op.Set_list_order (_, order) -> Some order
    | _ -> None)
;;

let row_mapping ops =
  List.find_map_exn ops ~f:(function
    | W.Op.Set_list_rows (_, rows) -> Some rows
    | _ -> None)
;;

let logical_ids (order : L.Order.t) =
  List.concat_map order.runs ~f:(fun run ->
    List.init (Int64.to_int_exn run.count) ~f:(fun offset ->
      Int64.(run.first + of_int offset)))
;;

let%expect_test "streaming sends no order; reorder keeps keyed identities and nodes" =
  let r = R.create window in
  let source = order [ "a"; "b"; "c" ] in
  let initial = prepare r (managed source [ "b", "draft" ]) in
  let initial_ops = operations initial in
  let ids = logical_ids (source_order initial_ops) in
  assert (List.equal Int64.equal ids [ 1L; 2L; 3L ]);
  let mounted = row_mapping initial_ops |> List.hd_exn in
  assert (Int64.equal mounted.id 2L);
  accept r initial;
  let streamed =
    prepare
      r
      (managed ~invalidated:[ "b" ] ~invalidation_revision:1L source [ "b", "complete" ])
  in
  (match operations streamed with
   | [ Set_text (_, "complete"); Invalidate_list_rows (_, [ 2L ]) ] -> ()
   | ops ->
     print_s [%sexp (ops : W.Op.t list)];
     assert false);
  accept r streamed;
  let reordered =
    prepare
      r
      (managed
         ~invalidated:[ "b" ]
         ~invalidation_revision:1L
         (order [ "c"; "b"; "a"; "d" ])
         [ "b", "complete" ])
  in
  assert (
    List.equal
      Int64.equal
      (logical_ids (source_order (operations reordered)))
      [ 3L; 2L; 1L; 4L ]);
  assert (
    not
      (List.exists (operations reordered) ~f:(function
         | Set_list_rows _ | Create _ | Remove _ -> true
         | _ -> false)));
  accept r reordered;
  let repeated =
    prepare
      r
      (managed
         ~invalidated:[ "b" ]
         ~invalidation_revision:2L
         (order [ "c"; "b"; "a"; "d" ])
         [ "b", "complete" ])
  in
  (match operations repeated with
   | [ Invalidate_list_rows (_, [ 2L ]) ] -> ()
   | _ -> assert false);
  accept r repeated;
  let bad =
    managed
      ~invalidated:[ "c" ]
      ~invalidation_revision:2L
      (order [ "c"; "b"; "a"; "d" ])
      [ "b", "complete" ]
  in
  assert (Or_error.is_error (R.prepare r ~theme:Theme.default (Some bad)));
  [%expect {| |}]
;;

let%expect_test "viewport translation rejects an obsolete order and removed handlers" =
  let r = R.create window in
  let initial = prepare r (managed (order [ "a"; "b" ]) []) in
  let node, handler =
    List.find_map_exn (operations initial) ~f:(function
      | Create (node, Virtual_list, _, Some handler) -> Some (node, handler)
      | _ -> None)
  in
  accept r initial;
  let viewport : L.Viewport.t =
    { order_revision = 1L
    ; visible_first = 0L
    ; visible_last = 1L
    ; requested = [ 1L; 2L ]
    ; pinned = []
    ; anchor = Some (1L, 3.)
    ; following_tail = false
    ; at_start = false
    ; at_end = false
    ; budget_exhausted = false
    }
  in
  let event viewport = W.Event.List_viewport (window, node, handler, 1L, viewport) in
  let first = R.dispatch r (event viewport) |> Option.value_exn in
  assert (List.equal Key.equal first.requested [ key "a"; key "b" ]);
  accept r (prepare r (managed (order [ "b"; "a" ]) []));
  assert (Option.is_none (R.dispatch r (event viewport)));
  assert (
    Option.is_none
      (R.dispatch r (event { viewport with order_revision = 2L; requested = [ 99L ] })));
  assert (Option.is_none (R.dispatch r (event { viewport with order_revision = 2L })));
  assert (
    Option.is_some
      (R.dispatch
         r
         (W.Event.List_viewport
            (window, node, handler, R.revision r, { viewport with order_revision = 2L }))));
  R.close r;
  assert (Option.is_none (R.dispatch r (event { viewport with order_revision = 2L })));
  [%expect {| |}]
;;

let%expect_test "horizontal configurations validate extents and retain keyed native rows" =
  let module V = Virtual_list in
  let horizontal = V.Config.horizontal ~width:(Fixed 80.) () |> Or_error.ok_exn in
  assert (V.Axis.equal (V.Config.axis horizontal) Horizontal);
  assert (V.Extent.equal (V.Config.extent horizontal) (Fixed 80.));
  assert (V.Height.equal (V.Config.height horizontal) (Fixed 80.));
  assert (V.Axis.equal (V.Config.axis config) Vertical);
  List.iter [ Float.nan; Float.infinity; 0.; 1_000_001. ] ~f:(fun value ->
    assert (Or_error.is_error (V.Config.horizontal ~width:(Estimated value) ()));
    assert (Or_error.is_error (V.Config.horizontal ~width:(Fixed value) ())));
  List.iter [ 1.; 1_000_000. ] ~f:(fun value ->
    V.Config.horizontal ~width:(Estimated value) () |> Or_error.ok_exn |> ignore);
  let r = R.create window in
  let view config =
    View.virtual_list ~config [ key "a", View.text "A"; key "b", View.text "B" ]
    |> Or_error.ok_exn
  in
  let initial = prepare r (view horizontal) in
  let ops = operations initial in
  let node =
    List.find_map_exn ops ~f:(function
      | Set_list_axis (id, Horizontal) -> Some id
      | _ -> None)
  in
  let rows = row_mapping ops in
  accept r initial;
  assert (List.is_empty (operations (prepare r (view horizontal))));
  let vertical = V.Config.create ~height:(Fixed 80.) () |> Or_error.ok_exn in
  List.iter [ vertical; horizontal ] ~f:(fun next ->
    let change = prepare r (view next) in
    let ops = operations change in
    assert (
      List.exists ops ~f:(function
        | Set_list_axis (id, axis) ->
          Gpuio_protocol.Node_id.equal id node
          && L.Axis.equal axis (V.Expert.axis_to_wire (V.Config.axis next))
        | _ -> false));
    assert (
      not
        (List.exists ops ~f:(function
           | Create _ | Remove _ | Set_list_order _ | Set_list_rows _ | Set_list_config _
             -> true
           | _ -> false)));
    (* The existing wrappers change clipping and main extent; their identities
       and the logical order remain intact across the axis replacement. *)
    assert (
      List.for_all rows ~f:(fun row ->
        List.exists ops ~f:(function
          | Set_style (id, _) -> Gpuio_protocol.Node_id.equal id row.L.Row.node
          | _ -> false)));
    accept r change);
  assert (
    Or_error.is_error
      (View.Expert.managed_virtual_list
         ~config:horizontal
         ~order:(order [ "a" ])
         ~invalidated:[]
         ~invalidation_revision:0L
         ~on_viewport:Fn.id
         ~on_retain:(fun _ -> failwith "unexpected")
         ~on_tree_input:(fun _ -> failwith "unexpected")
         []));
  [%expect {| |}]
;;
