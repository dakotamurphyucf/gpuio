open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module G = Gpuio
module C = G.List_collection
module S = G.List_selection
module Rows = G.List_rows
module L = Gpuio_bonsai.Selectable_list
module R = G.Reconciler
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let key = G.Key.of_string_exn
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok
let config = G.Virtual_list.Config.create ~max_active:1 ~height:(Fixed 36.) () |> ok

let make_source () =
  C.of_alist (module Int) [ 0, "Section"; 1, "One"; 2, "Disabled"; 3, "Three" ] |> ok
;;

let layout source ?visible () =
  S.Catalog.create (C.identity source) ?visible ~disabled:[ 0; 2 ] ()
  |> ok
  |> fun catalog -> Rows.Layout.create catalog ~decorations:[ 0 ] () |> ok
;;

let interaction
      ?(epoch = "query-1")
      ?(busy = false)
      ?(disabled = false)
      ?(mode = S.Mode.Multiple)
      ?(boundary = S.Boundary.Wrap)
      ()
  =
  L.Interaction.create ~epoch:(key epoch) ~mode ~boundary ~busy ~disabled ()
;;

let create component =
  Bonsai_driver.create
    ~action_history:Release_after_flush
    ~clock:(Bonsai.Time_source.create ~start:Time_ns.epoch)
    component
;;

let result d =
  Bonsai_driver.flush d;
  Bonsai_driver.result d |> ok
;;

let display d =
  Bonsai_driver.trigger_lifecycles d;
  ignore (result d : (int, string, Int.comparator_witness) L.Output.t)
;;

let run d event =
  Bonsai_driver.schedule_event d event;
  ignore (result d : (int, string, Int.comparator_witness) L.Output.t)
;;

let cursor d = S.cursor (L.Output.state (result d)) |> Option.map ~f:C.Item_ref.key
let selected d = S.selected (L.Output.state (result d)) |> List.map ~f:C.Item_ref.key
let ctrl d = L.Output.controller (result d)
let target d i = L.Output.target (result d) i |> Option.value_exn

let rec find_owner view =
  let d = G.View.Expert.describe view in
  if Option.is_some d.virtual_list then Some d else List.find_map d.children ~f:find_owner
;;

let owner d = find_owner (L.Output.view (result d)) |> Option.value_exn
let payload d = (owner d).virtual_list |> Option.value_exn
let input d request = (Option.value_exn (payload d).list_input |> snd) request

let wire_at d index =
  G.Virtual_list.Order.keys (payload d).order |> fun keys -> List.nth_exn keys index
;;

let frame r d =
  let output = result d in
  let update = R.prepare r ~theme:G.Theme.default (Some (L.Output.view output)) |> ok in
  R.accept r update |> ok;
  display d;
  match R.message update with
  | Some (W.Message.Apply tx) -> tx.operations
  | None -> []
  | Some _ -> assert false
;;

let observe d index =
  let wire = wire_at d index in
  let viewport : G.Virtual_list.Viewport.t =
    { visible_first = index
    ; visible_last = index + 1
    ; requested = [ wire ]
    ; pinned = []
    ; anchor = Some (wire, 0.)
    ; following_tail = false
    ; at_start = index = 0
    ; at_end = false
    ; budget_exhausted = false
    }
  in
  run d ((payload d).on_viewport |> Option.value_exn |> fun callback -> callback viewport)
;;

let component
      source
      layout
      interaction
      ?initial_selected
      ?render_row
      ?query
      ?before
      ?after
      ?on_action
      ?config:cfg
      graph
  =
  L.component
    source
    ~layout
    ~interaction
    ~config:(Option.value cfg ~default:(B.return config))
    ~label:"Choices"
    ~item_label:(fun ~key:_ data -> data)
    ?initial_selected
    ?render_row
    ?query
    ?before
    ?after
    ?on_action
    graph
;;

let%expect_test
    "owned selection keeps hidden preferences and reduces ordered relative native input"
  =
  let source = make_source () in
  let source_var = B.Expert.Var.create source in
  let layout_var = B.Expert.Var.create (layout source ()) in
  let policy_var = B.Expert.Var.create (interaction ()) in
  let seeds = B.Expert.Var.create [ 1 ] in
  let actions = ref [] in
  let d =
    create
      (component
         (B.Expert.Var.value source_var)
         (B.Expert.Var.value layout_var)
         (B.Expert.Var.value policy_var)
         ~initial_selected:(B.Expert.Var.value seeds)
         ~on_action:
           (B.return (fun action -> E.of_thunk (fun () -> actions := action :: !actions))))
  in
  let r = R.create window in
  ignore (frame r d : W.Op.t list);
  assert (List.equal Int.equal (selected d) [ 1 ]);
  assert (Option.is_none (cursor d));
  B.Expert.Var.set seeds [ 3 ];
  let callback = input d in
  run
    d
    (E.Many
       [ callback (G.List_input.Navigate (Next, None))
       ; callback (Navigate (Next, None))
       ; callback (Select_active Toggle)
       ; callback (Confirm_active Secondary)
       ]);
  assert (Option.equal Int.equal (cursor d) (Some 3));
  assert (List.equal Int.equal (selected d) [ 1; 3 ]);
  (match !actions with
   | [ L.Action.Confirm (t, Secondary) ] -> assert (C.Item_ref.key t = 3)
   | _ -> assert false);
  let controller = ctrl d in
  run d (L.Controller.context controller ~target:(target d 1) ());
  assert (Option.equal Int.equal (cursor d) (Some 3));
  assert (Option.value_exn (S.context (L.Output.state (result d))) |> C.Item_ref.key = 1);
  B.Expert.Var.set layout_var (layout source ~visible:[ 0; 3 ] ());
  ignore (frame r d : W.Op.t list);
  assert (List.equal Int.equal (selected d) [ 1; 3 ]);
  run d (L.Controller.navigate controller Next);
  assert (Option.equal Int.equal (cursor d) (Some 3));
  run d (L.Controller.cancel controller);
  assert (Option.is_none (cursor d));
  assert (List.equal Int.equal (selected d) [ 1; 3 ]);
  B.Expert.Var.set layout_var (layout source ());
  ignore (frame r d : W.Op.t list);
  let old_target = target d 1 in
  let removed = C.splice source ~at:1 ~remove:1 [] |> ok in
  let replaced = C.splice removed ~at:1 ~remove:0 [ 1, "New one" ] |> ok in
  B.Expert.Var.set source_var replaced;
  B.Expert.Var.set layout_var (layout replaced ());
  ignore (frame r d : W.Op.t list);
  assert (List.equal Int.equal (selected d) [ 3 ]);
  actions := [];
  run
    d
    (E.Many
       [ L.Controller.select controller old_target Replace
       ; L.Controller.confirm controller ~target:old_target Primary
       ]);
  assert (List.is_empty !actions);
  assert (List.equal Int.equal (selected d) [ 3 ]);
  B.Expert.Var.set policy_var (interaction ~disabled:true ());
  run d (L.Controller.clear_selection controller);
  assert (List.equal Int.equal (selected d) [ 3 ]);
  B.Expert.Var.set policy_var (interaction ());
  run d (L.Controller.clear_selection controller);
  assert (List.is_empty (selected d));
  R.close r;
  Bonsai_driver.Expert.invalidate_observers d;
  [%expect {| |}]
;;

let query controller =
  G.View.text_input
    ~controller:(key controller)
    ~config:(G.Text_input.Config.create ~mode:Single_line ~label:"Search" () |> ok)
    ~on_event:(fun _ -> E.Ignore)
    ()
  |> ok
  |> fun view -> G.View.with_key view (key "search-view")
;;

let%expect_test
    "queued native actions check new policy before native acknowledgement; busy does not \
     retire"
  =
  let source = make_source () in
  let policy = B.Expert.Var.create (interaction ()) in
  let before = B.Expert.Var.create [ query "editor-1" ] in
  let d =
    create
      (component
         (B.return source)
         (B.return (layout source ()))
         (B.Expert.Var.value policy)
         ~query:(B.return (key "search-view"))
         ~before:(B.Expert.Var.value before))
  in
  let r = R.create window in
  ignore (frame r d : W.Op.t list);
  let old = input d (G.List_input.Navigate (Next, None)) in
  B.Expert.Var.set policy (interaction ~busy:true ());
  run d old;
  assert (Option.equal Int.equal (cursor d) (Some 1));
  let old = input d (G.List_input.Navigate (Next, None)) in
  B.Expert.Var.set policy (interaction ~epoch:"query-2" ());
  let pending =
    R.prepare r ~theme:G.Theme.default (Some (L.Output.view (result d))) |> ok
  in
  (* The old native frame is still accepted while Bonsai already sees query-2. *)
  run d old;
  assert (Option.equal Int.equal (cursor d) (Some 1));
  R.accept r pending |> ok;
  display d;
  let old = input d (G.List_input.Navigate (Next, None)) in
  B.Expert.Var.set before [ query "editor-2" ];
  run d old;
  assert (Option.equal Int.equal (cursor d) (Some 1));
  ignore (frame r d : W.Op.t list);
  let old = input d (G.List_input.Navigate (Next, None)) in
  B.Expert.Var.set policy (interaction ~epoch:"query-2" ~mode:Single ());
  run d old;
  assert (Option.equal Int.equal (cursor d) (Some 1));
  run d (input d (Navigate (Next, None)));
  assert (Option.equal Int.equal (cursor d) (Some 3));
  ignore (frame r d : W.Op.t list);
  let old = input d (G.List_input.Navigate (Next, None)) in
  B.Expert.Var.set policy (interaction ~epoch:"query-2" ~mode:Single ~boundary:Stop ());
  run d old;
  assert (Option.equal Int.equal (cursor d) (Some 3));
  let changed = frame r d in
  assert (
    List.exists changed ~f:(function
      | W.Op.Set_list_input (_, Some _) -> true
      | _ -> false));
  run d (input d (Navigate (Next, None)));
  assert (Option.equal Int.equal (cursor d) (Some 3));
  R.close r;
  Bonsai_driver.Expert.invalidate_observers d;
  [%expect {| |}]
;;

let render_counter ~row ~controller:_ ~lifetime:_ graph =
  let count, bump =
    B.state_machine0 ~default_model:0 ~apply_action:(fun _ count () -> count + 1) graph
  in
  let open B.Let_syntax in
  let%arr count = count
  and bump = bump
  and row = row in
  G.View.button
    ~on_click:(fun () -> bump ())
    (Int.to_string (Rows.Item.key (L.Row.item row)) ^ ":" ^ Int.to_string count)
;;

let rec button view =
  let d = G.View.Expert.describe view in
  match d.on_click with
  | Some callback -> Some (d.text, callback)
  | None -> List.find_map d.children ~f:button
;;

let count d = button (L.Output.view (result d)) |> Option.value_exn

let%expect_test
    "one-row budget, axis changes, point streaming and source replacement retain only \
     current models"
  =
  let source = make_source () in
  let source_var = B.Expert.Var.create source in
  let layout_var = B.Expert.Var.create (layout source ()) in
  let cfg = B.Expert.Var.create config in
  let d =
    create
      (component
         (B.Expert.Var.value source_var)
         (B.Expert.Var.value layout_var)
         (B.return (interaction ()))
         ~config:(B.Expert.Var.value cfg)
         ~render_row:render_counter)
  in
  let r = R.create window in
  ignore (frame r d : W.Op.t list);
  observe d 1;
  ignore (frame r d : W.Op.t list);
  run d ((snd (count d)) ());
  assert (String.equal (fst (count d)) "1:1");
  let old = ctrl d in
  let old_target = target d 1 in
  run d (L.Controller.select old old_target Replace);
  ignore (frame r d : W.Op.t list);
  let streamed = C.set source ~key:1 ~data:"One streamed" |> ok in
  B.Expert.Var.set source_var streamed;
  ignore (frame r d : W.Op.t list);
  assert (String.equal (fst (count d)) "1:1");
  B.Expert.Var.set
    cfg
    (G.Virtual_list.Config.horizontal ~max_active:1 ~width:(Fixed 100.) () |> ok);
  let changes = frame r d in
  assert (
    not
      (List.exists changes ~f:(function
         | W.Op.Create _ | Remove _ -> true
         | _ -> false)));
  assert (String.equal (fst (count d)) "1:1");
  run d (L.Controller.focus old (target d 3));
  observe d 3;
  ignore (frame r d : W.Op.t list);
  assert (L.Output.active_rows (result d) = 1);
  assert (String.equal (fst (count d)) "3:0");
  assert (List.equal Int.equal (selected d) [ 1 ]);
  observe d 1;
  ignore (frame r d : W.Op.t list);
  assert (String.equal (fst (count d)) "1:0");
  let new_source = make_source () in
  B.Expert.Var.set source_var new_source;
  B.Expert.Var.set layout_var (layout new_source ());
  ignore (frame r d : W.Op.t list);
  run d (L.Controller.select old old_target Replace);
  assert (List.is_empty (selected d));
  (* Revisiting the original source does not resurrect its controller/models. *)
  B.Expert.Var.set source_var source;
  B.Expert.Var.set layout_var (layout source ());
  ignore (frame r d : W.Op.t list);
  run d (L.Controller.select old old_target Replace);
  assert (List.is_empty (selected d));
  observe d 1;
  ignore (frame r d : W.Op.t list);
  assert (String.equal (fst (count d)) "1:0");
  R.close r;
  Bonsai_driver.Expert.invalidate_observers d;
  [%expect {| |}]
;;

let%expect_test "invalid seeds and stale layout are recoverable component errors" =
  let source = make_source () in
  let check source layout initial =
    let d =
      create
        (component
           (B.return source)
           (B.return layout)
           (B.return (interaction ~mode:Single ()))
           ~initial_selected:(B.return initial))
    in
    Bonsai_driver.flush d;
    assert (Result.is_error (Bonsai_driver.result d));
    Bonsai_driver.Expert.invalidate_observers d
  in
  check source (layout source ()) [ 1; 3 ];
  check source (layout source ()) [ 99 ];
  check (make_source ()) (layout source ()) [];
  [%expect {| |}]
;;

let%expect_test "unmount retires external controllers even when the same source returns" =
  let source = make_source () in
  let shown = B.Expert.Var.create true in
  let d =
    create (fun graph ->
      let open B.Let_syntax in
      match%sub B.Expert.Var.value shown with
      | false -> B.return (Or_error.error_string "hidden")
      | true ->
        component
          (B.return source)
          (B.return (layout source ()))
          (B.return (interaction ()))
          graph)
  in
  let r = R.create window in
  ignore (frame r d : W.Op.t list);
  let old = ctrl d in
  let old_target = target d 1 in
  run d (L.Controller.select old old_target Replace);
  B.Expert.Var.set shown false;
  Bonsai_driver.flush d;
  let removal = R.prepare r ~theme:G.Theme.default None |> ok in
  R.accept r removal |> ok;
  Bonsai_driver.trigger_lifecycles d;
  Bonsai_driver.flush d;
  B.Expert.Var.set shown true;
  ignore (frame r d : W.Op.t list);
  run d (L.Controller.select old old_target Replace);
  assert (List.is_empty (selected d));
  run d (L.Controller.select (ctrl d) (target d 1) Replace);
  assert (List.equal Int.equal (selected d) [ 1 ]);
  R.close r;
  Bonsai_driver.Expert.invalidate_observers d;
  [%expect {| |}]
;;
