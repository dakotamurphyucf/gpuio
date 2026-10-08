open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module G = Gpuio
module View = G.View

let ok = Or_error.ok_exn

let rec native_owner view =
  let description = View.Expert.describe view in
  if Option.is_some description.virtual_list
  then Some description
  else (
    assert (Option.is_none description.scrollbar);
    List.find_map description.children ~f:native_owner)
;;

let mounted (activated, deactivated) graph =
  B.Edge.lifecycle
    ~on_activate:(B.return (E.of_thunk (fun () -> Int.incr activated)))
    ~on_deactivate:(B.return (E.of_thunk (fun () -> Int.incr deactivated)))
    graph
;;

let check name make =
  let appearance label = G.Scrollbar.create ~label ~mode:Always () |> ok in
  let initial = Some (appearance "Initial") in
  let scrollbar = B.Expert.Var.create initial in
  let activated = ref 0 in
  let deactivated = ref 0 in
  let driver =
    Component_test.create
      (make (B.Expert.Var.value scrollbar) (mounted (activated, deactivated)))
  in
  let owner () = native_owner (Component_test.result driver) |> Option.value_exn in
  let first = owner () in
  assert (Option.equal G.Scrollbar.equal first.scrollbar initial);
  let payload = Option.value_exn first.virtual_list in
  let first_index = if String.equal name "selectable list" then 1 else 0 in
  let requested =
    List.take (List.drop (G.Virtual_list.Order.keys payload.order) first_index) 1
  in
  let viewport : G.Virtual_list.Viewport.t =
    { visible_first = first_index
    ; visible_last = first_index + 1
    ; requested
    ; pinned = []
    ; anchor = Some (List.hd_exn requested, 0.)
    ; following_tail = false
    ; at_start = true
    ; at_end = false
    ; budget_exhausted = false
    }
  in
  Bonsai_driver.trigger_lifecycles driver;
  Bonsai_driver.schedule_event driver (Option.value_exn payload.on_viewport viewport);
  ignore (owner ());
  Bonsai_driver.trigger_lifecycles driver;
  let active = !activated in
  assert (active > 0);
  let row_keys =
    List.map (owner ()).children ~f:(fun v -> (View.Expert.describe v).key)
  in
  List.iter
    [ Some (appearance "Changed"); None; initial ]
    ~f:(fun expected ->
      B.Expert.Var.set scrollbar expected;
      let current = owner () in
      assert (Option.equal G.Scrollbar.equal current.scrollbar expected);
      assert (
        List.equal
          (Option.equal G.Key.equal)
          row_keys
          (List.map current.children ~f:(fun v -> (View.Expert.describe v).key)));
      Bonsai_driver.trigger_lifecycles driver;
      assert (!activated = active);
      assert (!deactivated = 0));
  Bonsai_driver.Expert.invalidate_observers driver;
  print_endline (name ^ ": native presentation updated/cleared; row lifetimes retained")
;;

let%expect_test "managed collection presentation reaches native owners without remount" =
  check "list" (fun scrollbar mounted graph ->
    let module V = Gpuio_bonsai.Virtual_list in
    V.component
      (module Int)
      (B.return (Component_test.source [ 0, "row" ]))
      ~row_key:G.Key.of_int
      ~config:Component_test.config
      ~scrollbar
      ~render_row:(fun ~key:_ ~data ~lifetime:_ graph ->
        mounted graph;
        B.map data ~f:View.text)
      graph
    |> B.map ~f:(Or_error.map ~f:V.Output.view));
  check "tree rows" (fun scrollbar mounted graph ->
    let module F = Tree_component_test in
    let module V = Gpuio_bonsai.Tree_rows in
    let forest = F.forest 2 in
    V.component
      (B.return (G.Tree_loading.snapshot (G.Tree_loading.create forest)))
      ~state:(B.return (G.Tree_state.create forest () |> ok))
      ~config:F.config
      ~scrollbar
      ~render_row:(fun ~key ~data ~lifetime graph ->
        mounted graph;
        F.text ~key ~data ~lifetime graph)
      graph
    |> B.map ~f:(Or_error.map ~f:V.Output.view));
  check "tree" (fun scrollbar mounted graph ->
    let module F = Tree_widget_test in
    let module V = Gpuio_bonsai.Tree in
    V.component
      (B.return (G.Tree_loading.snapshot (G.Tree_loading.create (F.forest ()))))
      ~config:F.config
      ~label:"Tree"
      ~scrollbar
      ~render_item:(fun ~target:_ ~item ~controller:_ ~lifetime:_ graph ->
        mounted graph;
        B.map item ~f:(fun item ->
          View.text (G.Tree.Node.label item.G.Tree_rows.Item.node)))
      graph
    |> B.map ~f:(Or_error.map ~f:V.Output.view));
  check "selectable list" (fun scrollbar mounted graph ->
    let module F = Selectable_list_component_test in
    let module V = Gpuio_bonsai.Selectable_list in
    let source = F.make_source () in
    V.component
      (B.return source)
      ~layout:(B.return (F.layout source ()))
      ~interaction:(B.return (F.interaction ()))
      ~config:(B.return F.config)
      ~label:"Choices"
      ~item_label:(fun ~key:_ data -> data)
      ~scrollbar
      ~render_row:(fun ~row ~controller ~lifetime graph ->
        mounted graph;
        F.render_counter ~row ~controller ~lifetime graph)
      graph
    |> B.map ~f:(Or_error.map ~f:V.Output.view));
  check "table" (fun scrollbar mounted graph ->
    let module F = Table_component_test in
    let module V = Gpuio_bonsai.Table in
    V.component
      (B.return (F.source 2))
      ~config:(B.return (F.config ()))
      ~scrollbar
      ~render_cell:(fun ~row ~data ~column ~lifetime graph ->
        mounted graph;
        F.text_cell ~row ~data ~column ~lifetime graph)
      graph
    |> B.map ~f:(Or_error.map ~f:V.Output.view));
  [%expect
    {|
    list: native presentation updated/cleared; row lifetimes retained
    tree rows: native presentation updated/cleared; row lifetimes retained
    tree: native presentation updated/cleared; row lifetimes retained
    selectable list: native presentation updated/cleared; row lifetimes retained
    table: native presentation updated/cleared; row lifetimes retained
    |}]
;;
