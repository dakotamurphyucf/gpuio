open Core
open Gpuio
module W = Gpuio_protocol.Wire
module R = Reconciler

let ok = Or_error.ok_exn
let key = Key.of_string_exn
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok
let config = Virtual_list.Config.create ~height:(Fixed 24.) () |> ok

let view ?(enabled = true) ?(semantics = true) keys =
  let order = Virtual_list.Order.create (List.map keys ~f:key) |> ok in
  let view =
    View.Expert.managed_virtual_list
      ~config
      ~order
      ~on_viewport:(fun _ -> None)
      ~on_retain:(fun _ -> None)
      ?on_tree_input:(if enabled then Some Option.some else None)
      (List.map keys ~f:(fun name -> key name, View.column [ View.text name ]))
    |> ok
  in
  if semantics
  then
    View.with_accessibility
      view
      (Accessibility.create ~role:(Tree false) ~label:"Projects" () |> ok)
    |> ok
  else view
;;

let prepare reconciler view = R.prepare reconciler ~theme:Theme.default (Some view) |> ok

let operations update =
  match R.message update with
  | Some (Apply tx) -> tx.operations
  | None -> []
  | Some _ -> assert false
;;

let accept reconciler update = R.accept reconciler update |> ok

let%expect_test "tree input uses stable row IDs and opt-in handler epochs" =
  let reconciler = R.create window in
  assert (
    Result.is_error
      (R.prepare reconciler ~theme:Theme.default (Some (view ~semantics:false [ "a" ]))));
  let initial = prepare reconciler (view [ "a"; "b" ]) in
  let node, handler =
    List.find_map_exn (operations initial) ~f:(function
      | Create (node, Virtual_list, _, Some handler) -> Some (node, handler)
      | _ -> None)
  in
  assert (
    List.exists (operations initial) ~f:(function
      | Set_tree_input (_, true) -> true
      | _ -> false));
  accept reconciler initial;
  let dispatch ?(handler = handler) request =
    R.dispatch reconciler (W.Event.Tree_input (window, node, handler, 1L, request))
    |> Option.join
  in
  let selected () =
    match dispatch (Select (2L, Replace)) with
    | Some (Tree_input.Select (key, _)) -> Key.to_string key
    | _ -> "ignored"
  in
  assert (String.equal (selected ()) "b");
  accept reconciler (prepare reconciler (view [ "b"; "a" ]));
  assert (String.equal (selected ()) "b");
  accept reconciler (prepare reconciler (view [ "a" ]));
  assert (String.equal (selected ()) "ignored");
  accept reconciler (prepare reconciler (view [ "a"; "b" ]));
  assert (String.equal (selected ()) "ignored");
  assert (Option.is_some (dispatch (Navigate (Next, None))));
  accept reconciler (prepare reconciler (view ~enabled:false [ "a"; "b" ]));
  assert (Option.is_none (dispatch Activate_active));
  let enabled = prepare reconciler (view [ "a"; "b" ]) in
  let current =
    List.find_map_exn (operations enabled) ~f:(function
      | Bind (_, Some handler) -> Some handler
      | _ -> None)
  in
  assert (not (Gpuio_protocol.Handler_id.equal current handler));
  accept reconciler enabled;
  assert (Option.is_none (dispatch Activate_active));
  assert (Option.is_some (dispatch ~handler:current Activate_active));
  R.close reconciler;
  assert (Option.is_none (dispatch ~handler:current Activate_active));
  print_endline
    "reorder preserves key; deletion/reinsert retires ID; input disable/re-enable \
     rotates handler; close rejects";
  [%expect
    {| reorder preserves key; deletion/reinsert retires ID; input disable/re-enable rotates handler; close rejects |}]
;;
