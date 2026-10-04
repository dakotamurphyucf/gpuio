open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View
module S = List_selection
module Driver = Gpuio_runtime_core.Window_driver
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let key = Key.of_string_exn
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok
let source = List_collection.of_alist (module String) [ "a", (); "b", (); "c", () ] |> ok
let catalog = S.Catalog.create (List_collection.identity source) () |> ok
let empty () = S.create catalog ~mode:Multiple () |> ok

module Model = struct
  type t =
    { selection : (string, String.comparator_witness) S.t
    ; epoch : int
    ; confirmed : string
    }

  let empty = { selection = empty (); epoch = 0; confirmed = "none" }
end

module Action = struct
  type t =
    | Request of int * Key.t List_input.t
    | Reset
end

let target key = List_collection.item_ref source key

let reduce (m : Model.t) request =
  let update f = { m with selection = f m.selection } in
  let with_target key f =
    Option.value_map (target key) ~default:m ~f:(fun target ->
      update (fun state -> f state target))
  in
  let confirm key kind =
    { m with
      confirmed =
        (key
         ^
         match kind with
         | S.Confirmation.Primary -> ":primary"
         | Secondary -> ":secondary")
    }
  in
  match request with
  | List_input.Navigate (direction, selection) ->
    update (fun state -> S.navigate state catalog ?selection direction)
  | Select (key, gesture) ->
    with_target key (fun state target -> S.select state catalog target gesture)
  | Focus key -> with_target key (fun state target -> S.focus state catalog target)
  | Set_selected (key, selected) ->
    with_target key (fun state target -> S.set_selected state catalog target selected)
  | Select_active gesture ->
    Option.value_map (S.cursor m.selection) ~default:m ~f:(fun target ->
      update (fun state -> S.select state catalog target gesture))
  | Confirm (key, kind) ->
    if S.Catalog.is_enabled catalog key then confirm key kind else m
  | Confirm_active kind ->
    Option.value_map
      (S.confirm m.selection catalog kind)
      ~default:m
      ~f:(fun (target, kind) -> confirm (List_collection.Item_ref.key target) kind)
  | Context key ->
    with_target key (fun state target -> S.with_context state catalog (Some target))
  | Context_active -> update (fun state -> S.with_context state catalog (S.cursor state))
  | Cancel -> update (fun state -> S.cancel state catalog)
;;

let component graph =
  let model, inject =
    B.state_machine0
      ~default_model:Model.empty
      ~apply_action:(fun _ model -> function
         | Action.Reset -> { Model.empty with epoch = model.epoch + 1 }
         | Request (epoch, _) when epoch <> model.epoch -> model
         | Request (_, request) ->
           (match
              List_input.filter_map request ~f:(fun key -> Some (Key.to_string key))
            with
            | None -> model
            | Some request -> reduce model request))
      graph
  in
  let open B.Let_syntax in
  let%arr model = model
  and inject = inject in
  let rows =
    List.mapi [ "a"; "b"; "c" ] ~f:(fun index name ->
      let item =
        Accessibility.Option_item.create
          ~index
          ~count:3
          ~selected:(S.is_selected model.selection name)
          ()
        |> ok
      in
      ( key name
      , V.with_accessibility
          (V.column [ V.text name ])
          (Accessibility.create ~role:(Option_item item) ~label:name () |> ok)
        |> ok ))
  in
  let list =
    V.virtual_list
      ~key:(key "results")
      ~config:(Virtual_list.Config.create ~height:(Fixed 24.) () |> ok)
      rows
    |> ok
  in
  let list =
    V.with_accessibility
      list
      (Accessibility.create ~role:(List_box true) ~label:"Results" () |> ok)
    |> ok
  in
  let list =
    V.with_list_input
      list
      ~config:
        (List_input.Config.create
           ~epoch:(key (Int.to_string model.epoch))
           ?cursor:
             (Option.map (S.cursor model.selection) ~f:(fun target ->
                key (List_collection.Item_ref.key target)))
           ())
      ~on_input:(fun request -> inject (Request (model.epoch, request)))
    |> ok
  in
  V.column [ list; V.text model.confirmed; V.button ~on_click:(inject Reset) "Reset" ]
;;

let cycle d = Driver.cycle d ~now:Time_ns.epoch |> ok

let accept d =
  match Driver.next_message d with
  | Some (W.Message.Apply tx) ->
    Driver.submitted d;
    Driver.acknowledge d ~revision:tx.revision |> ok;
    tx.operations
  | Some _ | None -> assert false
;;

let input ops =
  List.find_map_exn ops ~f:(function
    | W.Op.Set_list_input (_, Some cfg) -> Some cfg
    | _ -> None)
;;

let%expect_test
    "native list requests reduce in order through Bonsai without a cursor snapshot"
  =
  let d = Driver.create window ~start:Time_ns.epoch ~theme:Theme.default component in
  cycle d;
  let initial = accept d in
  let node, handler =
    List.find_map_exn initial ~f:(function
      | W.Op.Create (n, Virtual_list, _, Some h) -> Some (n, h)
      | _ -> None)
  in
  let request ?(generation = 1L) r =
    Driver.dispatch
      d
      (List_input (window, node, handler, Driver.revision d, generation, r))
  in
  request (Navigate (Next, None));
  request (Navigate (Next, None));
  request (Select_active Toggle);
  request (Confirm_active Secondary);
  cycle d;
  let changed = accept d in
  assert (Option.equal Int64.equal (input changed).cursor (Some 2L));
  assert (Int64.equal (input changed).generation 1L);
  assert (
    List.exists changed ~f:(function
      | Set_text (_, "b:secondary") -> true
      | _ -> false));
  assert (
    not
      (List.exists changed ~f:(function
         | Create _ | Remove _ -> true
         | _ -> false)));
  request Cancel;
  cycle d;
  let canceled = accept d in
  assert (Option.is_none (input canceled).cursor);
  assert (
    not
      (List.exists canceled ~f:(function
         | Set_accessibility (_, Some { role = Some (Option_item _); _ }) -> true
         | _ -> false)));
  let reset_node, reset_handler =
    List.find_map_exn initial ~f:(function
      | Create (n, Button, "Reset", Some h) -> Some (n, h)
      | _ -> None)
  in
  Driver.dispatch d (Press (window, reset_node, reset_handler, Driver.revision d));
  cycle d;
  (* The reset candidate is not accepted yet: an old native event can still
     reach its accepted callback. The reducer also checks its captured epoch. *)
  request (Navigate (Next, None));
  cycle d;
  let reset = accept d in
  assert (Int64.equal (input reset).generation 2L);
  cycle d;
  assert (Option.is_none (Driver.next_message d));
  request (Navigate (Next, None));
  cycle d;
  assert (Option.is_none (Driver.next_message d));
  request ~generation:2L (Navigate (Next, None));
  cycle d;
  assert (Option.equal Int64.equal (input (accept d)).cursor (Some 1L));
  for _ = 1 to 20 do
    cycle d;
    assert (Option.is_none (Driver.next_message d))
  done;
  Driver.close d;
  request ~generation:2L Cancel;
  [%expect {| |}]
;;
