open Core
open Gpuio
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let describe = View.Expert.describe
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok

let model =
  Pagination.create ~total_pages:Pagination.max_pages ~current:500_000_000 () |> ok
;;

let config = Overlay.Config.create ~label:"Choose page" () |> ok

let%expect_test "gap popup requires a native actionable anchor even in compact layout" =
  List.iter [ Navigation.Pagination_layout.Full; Compact ] ~f:(fun layout ->
    let result =
      Navigation.pagination
        model
        ~layout
        ~gap_popup:(fun ~first:_ ~last:_ -> assert false)
        ~on_request:ignore
        ()
    in
    assert (Or_error.is_error result));
  print_endline "invalid popup without on_gap rejected before invoking builders";
  [%expect {| invalid popup without on_gap rejected before invoking builders |}]
;;

let%expect_test "each gap retains its direct button; policy and layout bound construction"
  =
  let calls = ref [] in
  let render ?(layout = Navigation.Pagination_layout.Full) model content =
    Navigation.pagination
      model
      ~layout
      ~on_request:ignore
      ~on_gap:(fun ~first:_ ~last:_ -> ())
      ~gap_popup:(fun ~first ~last ->
        calls := (first, last) :: !calls;
        Navigation.Gap_popup.create ~config ~on_dismiss:ignore content)
      ()
    |> ok
  in
  let wrappers view =
    List.filter (describe view).children ~f:(fun v -> (describe v).popover)
  in
  let closed = render model None in
  assert (List.length !calls = 2);
  List.iter (wrappers closed) ~f:(fun wrapper ->
    let anchor = List.hd_exn (describe wrapper).children |> describe in
    assert (List.length (describe wrapper).children = 1);
    assert (View.Expert.Kind.equal anchor.kind Button);
    assert (String.equal anchor.text "…");
    assert (Option.is_some anchor.accessibility));
  let r = Reconciler.create window in
  let commit view =
    let update = Reconciler.prepare r ~theme:Theme.default (Some view) |> ok in
    Reconciler.accept r update |> ok;
    match Reconciler.message update with
    | Some (W.Message.Apply tx) -> tx.operations
    | None -> []
    | Some _ -> assert false
  in
  let initial = commit closed in
  let anchors =
    List.filter_map initial ~f:(function
      | W.Op.Create (id, Button, "…", _) -> Some id
      | _ -> None)
  in
  let owners =
    List.filter_map initial ~f:(function
      | W.Op.Set_popover (id, true) -> Some id
      | _ -> None)
  in
  assert (List.length owners = 2 && List.length anchors = 2);
  List.iter
    [ Some (View.text "Draft"); None ]
    ~f:(fun content ->
      let updates = commit (render model content) in
      assert (
        not
          (List.exists updates ~f:(function
             | W.Op.Remove id ->
               List.mem (anchors @ owners) id ~equal:Gpuio_protocol.Node_id.equal
             | W.Op.Set_popover _ -> true
             | _ -> false))));
  let disabled =
    render (Pagination.with_disabled model true) (Some (View.text "Hidden"))
  in
  assert (
    List.for_all (wrappers disabled) ~f:(fun v -> List.length (describe v).children = 1));
  calls := [];
  let compact = render ~layout:Compact model (Some (View.text "Hidden")) in
  assert (List.is_empty !calls && List.is_empty (wrappers compact));
  print_endline
    "two bounded direct anchors; identities survive open/close; disabled suppresses \
     content; compact skips builders";
  [%expect
    {| two bounded direct anchors; identities survive open/close; disabled suppresses content; compact skips builders |}]
;;
