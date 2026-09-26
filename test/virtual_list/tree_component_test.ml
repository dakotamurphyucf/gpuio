open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module T = Gpuio.Tree
module S = Gpuio.Tree_state
module L = Gpuio.Tree_loading
module R = Gpuio.Tree_rows
module C = Gpuio.List_collection
module V = Gpuio_bonsai.Tree_rows
module View = Gpuio.View

let ok = Or_error.ok_exn
let id text = T.Id.of_string text |> ok
let config = Gpuio.Virtual_list.Config.create ~max_active:4 ~height:(Fixed 24.) () |> ok
let leaf data = T.Node.create ~label:data ~children:Leaf data |> ok

let forest count =
  let nodes =
    List.init count ~f:(fun i ->
      let name = Int.to_string i in
      id name, leaf name)
  in
  T.create ~roots:(List.map nodes ~f:fst) nodes |> ok
;;

let lazy_forest count =
  let nodes =
    List.init count ~f:(fun i ->
      let name = Int.to_string i in
      ( id name
      , T.Node.create ~label:name ~children:(Branch { ids = []; next = More None }) name
        |> ok ))
  in
  T.create ~roots:(List.map nodes ~f:fst) nodes |> ok
;;

let create component =
  Bonsai_driver.create
    ~action_history:Release_after_flush
    ~clock:(Bonsai.Time_source.create ~start:Time_ns.epoch)
    component
;;

let result driver =
  Bonsai_driver.flush driver;
  Bonsai_driver.result driver |> ok
;;

let display driver =
  Bonsai_driver.trigger_lifecycles driver;
  ignore (result driver : _ V.Output.t)
;;

let rec list_view view =
  let description = View.Expert.describe view in
  if Option.is_some description.virtual_list
  then description
  else list_view (List.hd_exn description.children)
;;

let payload output = (list_view (V.Output.view output)).virtual_list |> Option.value_exn
let projection driver = V.Output.projection (result driver)
let key rows name = R.item_key rows (id name) |> Option.value_exn

let observe driver ~first ~last ?(pins = []) () =
  let output = result driver in
  let rows = V.Output.projection output in
  let requested =
    C.range (R.collection rows) ~first ~last
    |> ok
    |> List.map ~f:(fun (key, _) -> R.Key.to_view_key key)
  in
  let viewport : Gpuio.Virtual_list.Viewport.t =
    { visible_first = first
    ; visible_last = last
    ; requested
    ; pinned = List.map pins ~f:R.Key.to_view_key
    ; anchor = Option.map (List.hd requested) ~f:(fun key -> key, 0.)
    ; following_tail = false
    ; at_start = first = 0
    ; at_end = last = C.length (R.collection rows)
    ; budget_exhausted = false
    }
  in
  Bonsai_driver.schedule_event
    driver
    (Option.value_exn (payload output).on_viewport viewport)
;;

let text ~key:_ ~data ~lifetime:_ _ =
  B.map data ~f:(function
    | R.Row.Item item -> View.text (T.Node.data item.node)
    | Boundary boundary -> View.text ("boundary " ^ T.Id.to_string boundary.parent))
;;

let%expect_test "tree accessibility reaches the native list and one envelope per item" =
  let module A = Gpuio.Accessibility in
  let module W = Gpuio_protocol.Accessibility_wire in
  let tree = lazy_forest 1 in
  let loader = L.create tree in
  let root_metadata = A.create ~role:(Tree true) ~label:"Projects" () |> ok in
  let driver =
    create (fun graph ->
      V.component
        (B.return (L.snapshot loader))
        ~state:
          (B.return (S.create tree ~expanded:[ id "0" ] ~selected:[ id "0" ] () |> ok))
        ~config
        ~accessibility:(B.return root_metadata)
        ~render_row:(fun ~key:_ ~data ~lifetime:_ _ ->
          B.map data ~f:(function
            | R.Row.Item item ->
              View.with_accessibility
                (View.column [ View.text (T.Node.label item.node) ])
                (R.Item.accessibility item)
              |> ok
            | Boundary _ -> View.text "Load more"))
        graph)
  in
  ignore (result driver : _ V.Output.t);
  display driver;
  observe driver ~first:0 ~last:2 ();
  let output = result driver in
  let list = list_view (V.Output.view output) in
  let metadata = Option.value_exn list.accessibility |> A.Expert.to_wire in
  assert (Option.equal W.Role.equal metadata.role (Some (Tree true)));
  let item = List.hd_exn list.children |> View.Expert.describe in
  let metadata = Option.value_exn item.accessibility |> A.Expert.to_wire in
  (match metadata.role with
   | Some (Tree_item item) ->
     assert (item.selected && Option.value_exn item.expanded && item.level = 1);
     assert (Option.equal Int.equal item.count (Some 1));
     assert (not item.busy)
   | None
   | Some
       ( Group
       | Label
       | Link
       | Separator
       | Description_list
       | Term
       | Definition
       | Status
       | Alert
       | Image
       | Heading _
       | Navigation
       | Tree _ ) -> assert false);
  assert (Option.is_none (View.Expert.describe (List.hd_exn item.children)).accessibility);
  let boundary = List.nth_exn list.children 1 |> View.Expert.describe in
  assert (Option.is_none boundary.accessibility);
  display driver;
  Bonsai_driver.Expert.invalidate_observers driver;
  print_endline
    "Tree root on virtual list; selected expanded TreeItem on envelope; boundary not an \
     item";
  [%expect
    {| Tree root on virtual list; selected expanded TreeItem on envelope; boundary not an item |}]
;;

let%expect_test "100000 selected tree nodes mount only the shared active budget" =
  let tree = forest 100_000 in
  let loader = L.create tree in
  let source = B.Expert.Var.create (L.snapshot loader) in
  let state =
    B.Expert.Var.create (S.create tree ~mode:Multiple ~selected:(T.roots tree) () |> ok)
  in
  let mounted = ref 0
  and unmounted = ref 0 in
  let driver =
    create (fun graph ->
      V.component
        (B.Expert.Var.value source)
        ~state:(B.Expert.Var.value state)
        ~config
        ~render_row:(fun ~key ~data ~lifetime graph ->
          B.Edge.lifecycle
            ~on_activate:(B.return (E.of_thunk (fun () -> Int.incr mounted)))
            ~on_deactivate:(B.return (E.of_thunk (fun () -> Int.incr unmounted)))
            graph;
          text ~key ~data ~lifetime graph)
        graph)
  in
  assert (V.Output.active_rows (result driver) = 0);
  display driver;
  observe driver ~first:0 ~last:10 ();
  let first = result driver in
  assert (V.Output.active_rows first = 4 && V.Output.budget_exhausted first);
  display driver;
  assert (!mounted = 4 && !unmounted = 0);
  let first_key = key (V.Output.projection first) "0" in
  observe driver ~first:99990 ~last:100000 ~pins:[ first_key ] ();
  assert (V.Output.active_rows (result driver) = 4);
  display driver;
  assert (!mounted = 7 && !unmounted = 3);
  let tree = T.set_data tree ~id:(id "0") "updated" |> ok in
  L.update loader tree |> ok;
  B.Expert.Var.set source (L.snapshot loader);
  let updated = result driver in
  assert (
    List.equal
      Gpuio.Key.equal
      (payload updated).invalidated
      [ R.Key.to_view_key first_key ]);
  display driver;
  observe driver ~first:0 ~last:0 ();
  assert (V.Output.active_rows (result driver) = 0);
  display driver;
  assert (!mounted = !unmounted);
  assert (List.length (S.selected (R.state (projection driver))) = 100_000);
  Bonsai_driver.Expert.invalidate_observers driver;
  print_endline
    "100000 selections; at most four mounted rows including pins; all rows released";
  [%expect
    {| 100000 selections; at most four mounted rows including pins; all rows released |}]
;;

let%expect_test
    "coalesced tree payloads invalidate from accepted projection and source order"
  =
  let tree = forest 3 in
  let loader = L.create tree in
  let source = B.Expert.Var.create (L.snapshot loader) in
  let driver =
    create (fun graph ->
      V.component
        (B.Expert.Var.value source)
        ~state:(B.return (S.create tree () |> ok))
        ~config
        ~render_row:text
        graph)
  in
  let first = result driver in
  display driver;
  let update name data =
    L.update
      loader
      (T.set_data (L.Snapshot.tree (L.snapshot loader)) ~id:(id name) data |> ok)
    |> ok;
    B.Expert.Var.set source (L.snapshot loader)
  in
  update "0" "one";
  let intermediate = result driver in
  update "1" "two";
  let latest = result driver in
  assert (
    Int64.equal
      (payload intermediate).invalidation_revision
      (payload latest).invalidation_revision);
  let expected =
    [ key (V.Output.projection first) "0"; key (V.Output.projection first) "1" ]
    |> List.map ~f:R.Key.to_view_key
    |> List.sort ~compare:Gpuio.Key.compare
  in
  assert (
    List.equal
      Gpuio.Key.equal
      (List.sort (payload latest).invalidated ~compare:Gpuio.Key.compare)
      expected);
  display driver;
  assert (List.is_empty (payload (result driver)).invalidated);
  Bonsai_driver.Expert.invalidate_observers driver;
  [%expect {| |}]
;;

let%expect_test
    "independent generation-zero sources retire controllers and reset row models"
  =
  let tree = forest 1 in
  let first = L.create tree
  and second = L.create tree in
  let source = B.Expert.Var.create (L.snapshot first) in
  let driver =
    create (fun graph ->
      V.component
        (B.Expert.Var.value source)
        ~state:(B.return (S.create tree () |> ok))
        ~config
        ~render_row:(fun ~key:_ ~data:_ ~lifetime:_ graph ->
          let count, bump =
            B.state_machine0
              ~default_model:0
              ~apply_action:(fun _ count () -> count + 1)
              graph
          in
          let open B.Let_syntax in
          let%arr count = count
          and bump = bump in
          View.button ~on_click:bump (Int.to_string count))
        graph)
  in
  ignore (result driver : _ V.Output.t);
  display driver;
  observe driver ~first:0 ~last:1 ();
  let before = result driver in
  display driver;
  let button output =
    let row =
      List.hd_exn (list_view (V.Output.view output)).children |> View.Expert.describe
    in
    List.hd_exn row.children |> View.Expert.describe
  in
  Bonsai_driver.schedule_event driver (Option.value_exn (button before).on_click ());
  assert (String.equal (button (result driver)).text "1");
  display driver;
  let old_controller = V.Output.controller before in
  let old_key = key (V.Output.projection before) "0" in
  let old_native_source_key =
    let d =
      List.hd_exn (View.Expert.describe (V.Output.view before)).children
      |> View.Expert.describe
    in
    Option.value_exn d.key
  in
  B.Expert.Var.set source (L.snapshot second);
  let after = result driver in
  let new_native_source_key =
    let d =
      List.hd_exn (View.Expert.describe (V.Output.view after)).children
      |> View.Expert.describe
    in
    Option.value_exn d.key
  in
  assert (not (Gpuio.Key.equal old_native_source_key new_native_source_key));
  assert (V.Output.active_rows after = 0);
  display driver;
  Bonsai_driver.schedule_event driver (V.Controller.reveal old_controller old_key);
  assert (Option.is_none (payload (result driver)).scroll);
  observe driver ~first:0 ~last:1 ();
  assert (String.equal (button (result driver)).text "0");
  display driver;
  let current = result driver in
  Bonsai_driver.schedule_event
    driver
    (V.Controller.reveal
       (V.Output.controller current)
       (key (V.Output.projection current) "0"));
  assert (Option.is_some (payload (result driver)).scroll);
  Bonsai_driver.Expert.invalidate_observers driver;
  print_endline
    "same numeric generation; separate mounted identity; old controller ignored";
  [%expect
    {| same numeric generation; separate mounted identity; old controller ignored |}]
;;

let%expect_test "visible demand obeys active and queue bounds, with explicit retry" =
  let tree = lazy_forest 40 in
  let loader = L.create tree in
  let source = B.Expert.Var.create (L.snapshot loader) in
  let state = B.Expert.Var.create (S.create tree ~expanded:(T.roots tree) () |> ok) in
  let auto = B.Expert.Var.create true in
  let requested = ref []
  and retried = ref []
  and cancellations = ref 0 in
  let loading =
    V.Loading.create
      ~request:(fun target ->
        E.of_thunk (fun () ->
          requested := T.Id.to_string (L.Target.parent target) :: !requested;
          ignore (L.request loader (L.Target.parent target) |> ok : bool);
          B.Expert.Var.set source (L.snapshot loader)))
      ~retry:(fun target ->
        E.of_thunk (fun () -> retried := L.Target.parent target :: !retried))
      ~cancel:(fun _ -> E.Ignore)
      ~cancel_hidden:(fun _ state ->
        E.of_thunk (fun () ->
          Int.incr cancellations;
          L.cancel_hidden loader state;
          B.Expert.Var.set source (L.snapshot loader)))
  in
  let driver =
    create (fun graph ->
      V.component
        (B.Expert.Var.value source)
        ~state:(B.Expert.Var.value state)
        ~config
        ~loading:(B.return loading)
        ~auto_load:(B.Expert.Var.value auto)
        ~render_row:text
        graph)
  in
  ignore (result driver : _ V.Output.t);
  display driver;
  assert (List.is_empty !requested);
  observe driver ~first:0 ~last:80 ();
  ignore (result driver : _ V.Output.t);
  display driver;
  assert (List.equal String.equal (List.rev !requested) [ "0"; "1" ]);
  let request = L.take loader |> Option.value_exn in
  assert (L.Completion.equal (L.fail loader request (Error.of_string "offline")) Applied);
  B.Expert.Var.set source (L.snapshot loader);
  ignore (result driver : _ V.Output.t);
  display driver;
  observe driver ~first:0 ~last:4 ();
  ignore (result driver : _ V.Output.t);
  display driver;
  assert (List.length !requested = 2 && List.is_empty !retried);
  let output = result driver in
  Bonsai_driver.schedule_event
    driver
    (V.Controller.retry
       (V.Output.controller output)
       (L.Snapshot.target (L.snapshot loader) (id "0") |> ok));
  ignore (result driver : _ V.Output.t);
  assert (List.length !retried = 1);
  B.Expert.Var.set auto false;
  observe driver ~first:4 ~last:8 ();
  ignore (result driver : _ V.Output.t);
  display driver;
  assert (List.length !requested = 2);
  B.Expert.Var.set state (S.with_expanded (B.Expert.Var.get state) tree [] |> ok);
  ignore (result driver : _ V.Output.t);
  display driver;
  assert (L.Snapshot.queued_count (L.snapshot loader) = 0);
  assert (!cancellations > 0);
  Bonsai_driver.Expert.invalidate_observers driver;
  print_endline
    "two visible boundaries in four-row budget; no failure retry; collapse cancels";
  [%expect
    {| two visible boundaries in four-row budget; no failure retry; collapse cancels |}]
;;

let%expect_test "automatic demand never exceeds remaining queue capacity" =
  let tree = lazy_forest 100 in
  let loader = L.create tree in
  List.iter (List.range 20 83) ~f:(fun i ->
    assert (L.request loader (id (Int.to_string i)) |> ok));
  let source = B.Expert.Var.create (L.snapshot loader) in
  let requested = ref [] in
  let loading =
    V.Loading.create
      ~request:(fun target ->
        E.of_thunk (fun () ->
          requested := L.Target.parent target :: !requested;
          assert (L.request loader (L.Target.parent target) |> ok);
          B.Expert.Var.set source (L.snapshot loader)))
      ~retry:(fun _ -> E.Ignore)
      ~cancel:(fun _ -> E.Ignore)
      ~cancel_hidden:(fun _ _ -> E.Ignore)
  in
  let driver =
    create (fun graph ->
      V.component
        (B.Expert.Var.value source)
        ~state:(B.return (S.create tree ~expanded:(T.roots tree) () |> ok))
        ~config
        ~loading:(B.return loading)
        ~render_row:text
        graph)
  in
  ignore (result driver : _ V.Output.t);
  display driver;
  observe driver ~first:0 ~last:4 ();
  ignore (result driver : _ V.Output.t);
  display driver;
  assert (List.equal T.Id.equal !requested [ id "0" ]);
  assert (L.Snapshot.queued_count (L.snapshot loader) = 64);
  display driver;
  display driver;
  assert (List.length !requested = 1);
  ignore (L.take loader |> Option.value_exn : L.Request.t);
  B.Expert.Var.set source (L.snapshot loader);
  ignore (result driver : _ V.Output.t);
  display driver;
  observe driver ~first:0 ~last:4 ();
  ignore (result driver : _ V.Output.t);
  display driver;
  assert (List.equal T.Id.equal !requested [ id "1"; id "0" ]);
  assert (L.Snapshot.queued_count (L.snapshot loader) = 64);
  Bonsai_driver.Expert.invalidate_observers driver;
  print_endline
    "one available queue slot admits one boundary; released capacity admits next";
  [%expect
    {| one available queue slot admits one boundary; released capacity admits next |}]
;;

let%expect_test "unmount and remount of the same source retires controller effects" =
  let tree = lazy_forest 1 in
  let loader = L.create tree in
  let shown = B.Expert.Var.create true in
  let requests = ref 0 in
  let loading =
    V.Loading.create
      ~request:(fun _ -> E.of_thunk (fun () -> Int.incr requests))
      ~retry:(fun _ -> E.Ignore)
      ~cancel:(fun _ -> E.Ignore)
      ~cancel_hidden:(fun _ _ -> E.Ignore)
  in
  let driver =
    create (fun graph ->
      let members =
        B.map (B.Expert.Var.value shown) ~f:(fun shown ->
          if shown then Int.Map.singleton 0 () else Int.Map.empty)
      in
      B.assoc
        (module Int)
        members
        ~f:(fun _ _ graph ->
          V.component
            (B.return (L.snapshot loader))
            ~state:(B.return (S.create tree ~expanded:(T.roots tree) () |> ok))
            ~config
            ~loading:(B.return loading)
            ~render_row:text
            graph)
        graph)
  in
  let cycle () =
    Bonsai_driver.flush driver;
    Bonsai_driver.trigger_lifecycles driver;
    Bonsai_driver.flush driver;
    Bonsai_driver.result driver
  in
  let first = Map.find_exn (cycle ()) 0 |> ok in
  let target = L.Snapshot.target (L.snapshot loader) (id "0") |> ok in
  let old_effect = V.Controller.request (V.Output.controller first) target in
  Bonsai_driver.schedule_event driver old_effect;
  ignore (cycle () : _ Int.Map.t);
  assert (!requests = 1);
  B.Expert.Var.set shown false;
  assert (Map.is_empty (cycle ()));
  Bonsai_driver.schedule_event driver old_effect;
  ignore (cycle () : _ Int.Map.t);
  assert (!requests = 1);
  B.Expert.Var.set shown true;
  let second = Map.find_exn (cycle ()) 0 |> ok in
  Bonsai_driver.schedule_event driver old_effect;
  ignore (cycle () : _ Int.Map.t);
  assert (!requests = 1);
  Bonsai_driver.schedule_event
    driver
    (V.Controller.request (V.Output.controller second) target);
  ignore (cycle () : _ Int.Map.t);
  assert (!requests = 2);
  assert (L.Snapshot.queued_count (L.snapshot loader) = 0);
  Bonsai_driver.Expert.invalidate_observers driver;
  print_endline
    "old controller stays inactive after remount; application loader is still alive";
  [%expect
    {| old controller stays inactive after remount; application loader is still alive |}]
;;

let%expect_test
    "native input resolves current projection and rejects an old mounted source"
  =
  let module I = Gpuio.Tree_interaction in
  let tree = forest 3 in
  let loader = L.create tree in
  let source = B.Expert.Var.create (L.snapshot loader) in
  let state = B.Expert.Var.create (S.create tree () |> ok) in
  let count = ref 0 in
  let on_request request =
    E.of_thunk (fun () ->
      match I.apply (B.Expert.Var.get state) (B.Expert.Var.get source) request with
      | None -> ()
      | Some outcome ->
        Int.incr count;
        B.Expert.Var.set state (I.Outcome.state outcome))
  in
  let metadata =
    Gpuio.Accessibility.create ~role:(Tree false) ~label:"Projects" () |> ok
  in
  let driver =
    create (fun graph ->
      V.component
        (B.Expert.Var.value source)
        ~state:(B.Expert.Var.value state)
        ~config
        ~accessibility:(B.return metadata)
        ~on_request:(B.return on_request)
        ~render_row:text
        graph)
  in
  display driver;
  observe driver ~first:0 ~last:3 ();
  let initial = result driver in
  display driver;
  let callback = Option.value_exn (payload initial).on_tree_input in
  let event = callback (Navigate (Next, Some Replace)) in
  Bonsai_driver.schedule_event driver event;
  Bonsai_driver.schedule_event driver event;
  ignore (result driver : _ V.Output.t);
  assert (Option.equal T.Id.equal (S.active (B.Expert.Var.get state)) (Some (id "1")));
  assert (!count = 2);
  let target_key = key (V.Output.projection initial) "2" |> R.Key.to_view_key in
  Bonsai_driver.schedule_event driver (callback (Select (target_key, Replace)));
  ignore (result driver : _ V.Output.t);
  assert (Option.equal T.Id.equal (S.active (B.Expert.Var.get state)) (Some (id "2")));
  let replacement = L.create tree in
  B.Expert.Var.set source (L.snapshot replacement);
  B.Expert.Var.set state (S.create tree () |> ok);
  ignore (result driver : _ V.Output.t);
  display driver;
  Bonsai_driver.schedule_event driver event;
  ignore (result driver : _ V.Output.t);
  assert (!count = 3);
  assert (Option.is_none (S.active (B.Expert.Var.get state)));
  Bonsai_driver.Expert.invalidate_observers driver;
  print_endline
    "ordered requests reduce latest state; native keys resolve item identity; retired \
     source callback ignored";
  [%expect
    {| ordered requests reduce latest state; native keys resolve item identity; retired source callback ignored |}]
;;
