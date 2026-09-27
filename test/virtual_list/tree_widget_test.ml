open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module T = Gpuio.Tree
module S = Gpuio.Tree_state
module L = Gpuio.Tree_loading
module I = Gpuio.Tree_interaction
module R = Gpuio.Tree_rows
module C = Gpuio.List_collection
module W = Gpuio_bonsai.Tree
module View = Gpuio.View

let ok = Or_error.ok_exn
let id name = T.Id.of_string name |> ok
let ids names = List.map names ~f:id
let config = Gpuio.Virtual_list.Config.create ~max_active:4 ~height:(Fixed 28.) () |> ok

let leaf ?(disabled = false) name =
  T.Node.create ~label:name ~disabled ~children:Leaf name |> ok
;;

let branch name children =
  T.Node.create ~label:name ~children:(Branch { ids = ids children; next = End }) name
  |> ok
;;

let forest () =
  T.create
    ~roots:(ids [ "folder"; "other"; "disabled" ])
    [ id "folder", branch "Folder" [ "a"; "nested" ]
    ; id "a", leaf "Alpha"
    ; id "nested", branch "Nested" [ "b" ]
    ; id "b", leaf "Beta"
    ; id "other", leaf "Other"
    ; id "disabled", leaf ~disabled:true "Disabled"
    ]
  |> ok
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
  ignore (result driver : _ W.Output.t);
  Bonsai_driver.trigger_lifecycles driver;
  ignore (result driver : _ W.Output.t)
;;

let rec native_list view =
  let d = View.Expert.describe view in
  if Option.is_some d.virtual_list then d else native_list (List.hd_exn d.children)
;;

let payload output = (native_list (W.Output.view output)).virtual_list |> Option.value_exn
let target output name = W.Output.target output (id name) |> ok

let submit driver f =
  let output = result driver in
  Bonsai_driver.schedule_event driver (f (W.Output.controller output) output)
;;

let dispatch driver input =
  let callback = (payload (result driver)).on_tree_input |> Option.value_exn in
  Bonsai_driver.schedule_event driver (callback input)
;;

let names = List.map ~f:T.Id.to_string

let show driver =
  let s = W.Output.state (result driver) in
  print_s
    [%sexp
      (S.active s : T.Id.t option)
    , (names (S.selected s) : string list)
    , (names (S.expanded s) : string list)]
;;

let observe driver ~first ~last =
  let output = result driver in
  let collection = W.Output.projection output |> R.collection in
  let requested =
    C.range collection ~first ~last
    |> ok
    |> List.map ~f:(fun (key, _) -> R.Key.to_view_key key)
  in
  let viewport : Gpuio.Virtual_list.Viewport.t =
    { visible_first = first
    ; visible_last = last
    ; requested
    ; pinned = []
    ; anchor = None
    ; following_tail = false
    ; at_start = first = 0
    ; at_end = last = C.length collection
    ; budget_exhausted = false
    }
  in
  Bonsai_driver.schedule_event
    driver
    (Option.value_exn (payload output).on_viewport viewport)
;;

let scroll driver =
  let output = result driver in
  Option.map (payload output).scroll ~f:(fun scroll ->
    Gpuio.Virtual_list.Expert.scroll_to_wire scroll ~find_id:(fun key ->
      C.keys (R.collection (W.Output.projection output))
      |> List.findi ~f:(fun _ current -> Gpuio.Key.equal key (R.Key.to_view_key current))
      |> Option.map ~f:(fun (index, _) -> Int64.of_int (index + 1)))
    |> ok)
;;

let%expect_test "native requests reduce ordered state and reveal only after display" =
  let source = L.snapshot (L.create (forest ())) in
  let activated = ref [] in
  let driver =
    create (fun graph ->
      W.component
        (B.return source)
        ~config
        ~label:"Projects"
        ~on_action:
          (B.return (function
             | I.Action.Activate id ->
               E.of_thunk (fun () -> activated := id :: !activated)
             | None | Move _ -> assert false))
        graph)
  in
  display driver;
  let callback = Option.value_exn (payload (result driver)).on_tree_input in
  List.iter
    [ Gpuio.Tree_input.Navigate (Next, Some Replace); Navigate (Next, Some Replace) ]
    ~f:(fun input -> Bonsai_driver.schedule_event driver (callback input));
  show driver;
  assert (Option.is_none (scroll driver));
  display driver;
  let first = scroll driver |> Option.value_exn in
  assert (Gpuio_protocol.List_wire.Scroll_target.equal first.target (Focus_tree_row 2L));
  display driver;
  assert (Int64.equal (Option.value_exn (scroll driver)).serial first.serial);
  dispatch driver Activate_active;
  ignore (result driver : _ W.Output.t);
  assert (List.equal T.Id.equal !activated [ id "other" ]);
  Bonsai_driver.Expert.invalidate_observers driver;
  [%expect {| ((other) (other) ()) |}]
;;

let%expect_test "reveal opens ancestors, supersedes requests and survives sibling reorder"
  =
  let loader = L.create (forest ()) in
  let source = B.Expert.Var.create (L.snapshot loader) in
  let driver =
    create (fun graph ->
      W.component (B.Expert.Var.value source) ~config ~label:"Projects" graph)
  in
  display driver;
  let output = result driver in
  let controller = W.Output.controller output in
  let beta = target output "b" in
  Bonsai_driver.schedule_event driver (W.Controller.reveal controller ~focus:true beta);
  Bonsai_driver.schedule_event
    driver
    (W.Controller.reveal controller ~focus:false (target output "other"));
  show driver;
  display driver;
  let request = scroll driver |> Option.value_exn in
  assert (Gpuio_protocol.List_wire.Scroll_target.equal request.target (Reveal 5L));
  let tree = L.Snapshot.tree (L.snapshot loader) in
  let reordered =
    T.replace tree ~roots:(ids [ "other"; "folder"; "disabled" ]) (T.to_alist tree) |> ok
  in
  L.update loader reordered |> ok;
  B.Expert.Var.set source (L.snapshot loader);
  Bonsai_driver.schedule_event driver (W.Controller.reveal controller ~focus:true beta);
  display driver;
  let request = scroll driver |> Option.value_exn in
  assert (Gpuio_protocol.List_wire.Scroll_target.equal request.target (Focus_tree_row 5L));
  Bonsai_driver.Expert.invalidate_observers driver;
  [%expect {| ((b) () (folder nested)) |}]
;;

let%expect_test "source reset and deleted/recreated IDs retire captured commands" =
  let loader = L.create (forest ()) in
  let source = B.Expert.Var.create (L.snapshot loader) in
  let driver =
    create (fun graph ->
      W.component (B.Expert.Var.value source) ~config ~label:"Projects" graph)
  in
  display driver;
  observe driver ~first:0 ~last:3;
  display driver;
  let output = result driver in
  let controller = W.Output.controller output in
  let command = W.Controller.select controller (target output "other") in
  let tree = L.Snapshot.tree (L.snapshot loader) in
  let reduced =
    T.replace
      tree
      ~roots:(ids [ "folder"; "disabled" ])
      (List.filter (T.to_alist tree) ~f:(fun (key, _) ->
         not (T.Id.equal key (id "other"))))
    |> ok
  in
  L.update loader reduced |> ok;
  B.Expert.Var.set source (L.snapshot loader);
  ignore (result driver : _ W.Output.t);
  let replaced = T.replace reduced ~roots:(T.roots tree) (T.to_alist tree) |> ok in
  L.update loader replaced |> ok;
  B.Expert.Var.set source (L.snapshot loader);
  Bonsai_driver.schedule_event driver command;
  show driver;
  display driver;
  let current = result driver in
  let old =
    W.Controller.reveal (W.Output.controller current) ~focus:true (target current "b")
  in
  L.reset loader (forest ()) |> ok;
  B.Expert.Var.set source (L.snapshot loader);
  ignore (result driver : _ W.Output.t);
  Bonsai_driver.schedule_event driver old;
  display driver;
  show driver;
  assert (Option.is_none (scroll driver));
  Bonsai_driver.Expert.invalidate_observers driver;
  [%expect
    {|
    (() () ())
    (() () ())
    |}]
;;

let%expect_test "retaining a controller and target does not retain replaced payload" =
  let weak = Stdlib.Weak.create 1 in
  let initial =
    let payload = Bytes.create 1_000_000 in
    Stdlib.Weak.set weak 0 (Some payload);
    T.create
      ~roots:[ id "row" ]
      [ id "row", T.Node.create ~label:"Row" ~children:Leaf payload |> ok ]
    |> ok
  in
  let loader = L.create initial in
  let source = B.Expert.Var.create (L.snapshot loader) in
  let driver =
    create (fun graph ->
      W.component (B.Expert.Var.value source) ~config ~label:"Payload" graph)
  in
  display driver;
  let controller, target =
    let output = result driver in
    W.Output.controller output, target output "row"
  in
  let next =
    T.set_data (L.Snapshot.tree (L.snapshot loader)) ~id:(id "row") (Bytes.create 0) |> ok
  in
  L.update loader next |> ok;
  B.Expert.Var.set source (L.snapshot loader);
  display driver;
  display driver;
  Gc.full_major ();
  Gc.full_major ();
  assert (not (Stdlib.Weak.check weak 0));
  Bonsai_driver.schedule_event driver (W.Controller.select controller target);
  assert (List.equal T.Id.equal (S.selected (W.Output.state (result driver))) [ id "row" ]);
  Bonsai_driver.Expert.invalidate_observers driver;
  print_endline "payload collected while controller and target remain usable";
  [%expect {| payload collected while controller and target remain usable |}]
;;

let%expect_test "new expansion loads below the viewport within admission limits" =
  List.iter [ false; true ] ~f:(fun auto_load ->
    let branch_id i = id ("d" ^ Int.to_string i) in
    let nodes =
      List.init 8 ~f:(fun i ->
        let child = if i = 7 then id "leaf" else branch_id (i + 1) in
        ( branch_id i
        , T.Node.create
            ~label:(Int.to_string i)
            ~children:(Branch { ids = [ child ]; next = More None })
            ()
          |> ok ))
    in
    let tree =
      T.create
        ~roots:[ branch_id 0 ]
        ((id "leaf", T.Node.create ~label:"Leaf" ~children:Leaf () |> ok) :: nodes)
      |> ok
    in
    let loader = L.create tree in
    let source = B.Expert.Var.create (L.snapshot loader) in
    let loading =
      Gpuio_bonsai.Tree_rows.Loading.create
        ~request:(fun target ->
          E.of_thunk (fun () ->
            if L.Snapshot.is_current (L.snapshot loader) target
            then (
              ignore (L.request loader (L.Target.parent target) |> ok : bool);
              B.Expert.Var.set source (L.snapshot loader))))
        ~retry:(fun _ -> E.Ignore)
        ~cancel:(fun _ -> E.Ignore)
        ~cancel_hidden:(fun _ _ -> E.Ignore)
    in
    let driver =
      create (fun graph ->
        W.component
          (B.Expert.Var.value source)
          ~config
          ~label:"Nested"
          ~loading:(B.return loading)
          ~auto_load:(B.return auto_load)
          graph)
    in
    display driver;
    submit driver (fun controller output ->
      W.Controller.reveal controller (target output "leaf"));
    display driver;
    assert (List.length (S.expanded (W.Output.state (result driver))) = 8);
    assert (W.Output.active_rows (result driver) = 0);
    assert (L.Snapshot.queued_count (L.snapshot loader) = if auto_load then 4 else 0);
    Bonsai_driver.Expert.invalidate_observers driver);
  print_endline
    "opening eight ancestors without viewport: zero or four queued pages by policy";
  [%expect
    {| opening eight ancestors without viewport: zero or four queued pages by policy |}]
;;

let rec descriptions view =
  let d = View.Expert.describe view in
  d :: List.concat_map d.children ~f:descriptions
;;

let%expect_test
    "default disclosure pointer has no button focus stop and toggles queued clicks"
  =
  let driver =
    create (fun graph ->
      W.component
        (B.return (L.snapshot (L.create (forest ()))))
        ~config
        ~label:"Projects"
        graph)
  in
  display driver;
  observe driver ~first:0 ~last:3;
  let output = result driver in
  display driver;
  assert (W.Output.active_rows output = 3);
  let nodes = descriptions (W.Output.view output) in
  assert (not (List.exists nodes ~f:(fun d -> Option.is_some d.on_click)));
  let pointer = List.find_map nodes ~f:(fun d -> d.pointer) |> Option.value_exn in
  let sample phase x : Gpuio_protocol.Wire.Pointer.Sample.t =
    { gesture = 1L
    ; phase
    ; button = Left
    ; window_x = 0.
    ; window_y = 0.
    ; local_x = x
    ; local_y = 5.
    ; modifiers =
        { shift = false
        ; control = false
        ; alt = false
        ; command = false
        ; function_ = false
        }
    }
  in
  List.iter
    [ sample Released (-1.); sample (Cancelled Escape) 5. ]
    ~f:(fun sample ->
      Bonsai_driver.schedule_event
        driver
        (pointer.on_event (Gpuio.Pointer.Expert.event_of_wire sample |> ok)));
  assert (List.is_empty (S.expanded (W.Output.state (result driver))));
  let click =
    pointer.on_event (Gpuio.Pointer.Expert.event_of_wire (sample Released 5.) |> ok)
  in
  Bonsai_driver.schedule_event driver click;
  Bonsai_driver.schedule_event driver click;
  show driver;
  Bonsai_driver.Expert.invalidate_observers driver;
  [%expect {| ((folder) () ()) |}]
;;

let%expect_test "reactive seeds initialize once per mount and mode remains controlled" =
  let source = B.Expert.Var.create (L.snapshot (L.create (forest ()))) in
  let seed = B.Expert.Var.create (ids [ "folder"; "other" ]) in
  let mode = B.Expert.Var.create S.Mode.Multiple in
  let driver =
    create (fun graph ->
      W.component
        (B.Expert.Var.value source)
        ~config
        ~label:"Projects"
        ~mode:(B.Expert.Var.value mode)
        ~initial_selected:(B.Expert.Var.value seed)
        graph)
  in
  display driver;
  show driver;
  B.Expert.Var.set seed [ id "other" ];
  display driver;
  show driver;
  B.Expert.Var.set mode Single;
  display driver;
  show driver;
  B.Expert.Var.set source (L.snapshot (L.create (forest ())));
  display driver;
  show driver;
  Bonsai_driver.Expert.invalidate_observers driver;
  List.iter
    [ "", []; "Projects", [ id "absent" ] ]
    ~f:(fun (label, selected) ->
      let driver =
        create (fun graph ->
          W.component
            (B.return (L.snapshot (L.create (forest ()))))
            ~config
            ~label
            ~initial_selected:(B.return selected)
            graph)
      in
      Bonsai_driver.flush driver;
      assert (Or_error.is_error (Bonsai_driver.result driver));
      Bonsai_driver.Expert.invalidate_observers driver);
  [%expect
    {|
    (() (folder other) ())
    (() (folder other) ())
    (() (folder) ())
    (() (other) ())
    |}]
;;

let%expect_test "one hundred thousand selected items mount only the requested budget" =
  let nodes =
    List.init 100_000 ~f:(fun index ->
      let name = Int.to_string index in
      id name, leaf name)
  in
  let ids = List.map nodes ~f:fst in
  let tree = T.create ~roots:ids nodes |> ok in
  let driver =
    create (fun graph ->
      W.component
        (B.return (L.snapshot (L.create tree)))
        ~config
        ~label:"Large tree"
        ~mode:(B.return S.Mode.Multiple)
        ~initial_selected:(B.return ids)
        graph)
  in
  display driver;
  assert (W.Output.active_rows (result driver) = 0);
  observe driver ~first:0 ~last:100;
  display driver;
  let output = result driver in
  assert (W.Output.active_rows output = 4);
  assert (W.Output.budget_exhausted output);
  assert (List.length (S.selected (W.Output.state output)) = 100_000);
  assert (List.length (native_list (W.Output.view output)).children = 4);
  Bonsai_driver.Expert.invalidate_observers driver;
  print_endline "100000 persistent selections; 4 active rows; exhaustion reported";
  [%expect {| 100000 persistent selections; 4 active rows; exhaustion reported |}]
;;

let%expect_test "custom row models reset on eviction and a remount retires controllers" =
  let shown = B.Expert.Var.create true in
  let source = L.snapshot (L.create (forest ())) in
  let driver =
    create (fun graph ->
      let open B.Let_syntax in
      match%sub B.Expert.Var.value shown with
      | false -> B.return (Or_error.error_string "hidden")
      | true ->
        W.component
          (B.return source)
          ~config
          ~label:"Projects"
          ~render_item:(fun ~target:_ ~item:_ ~controller:_ ~lifetime graph ->
            let count, set_count = B.state 0 graph in
            let%arr count = count
            and set_count = set_count
            and lifetime = lifetime in
            View.button
              ~on_click:(fun () ->
                Gpuio_bonsai.Managed_rows.Lifetime.guard lifetime (set_count (count + 1)))
              (Int.to_string count))
          graph)
  in
  display driver;
  observe driver ~first:0 ~last:1;
  display driver;
  let button () =
    descriptions (W.Output.view (result driver))
    |> List.find_exn ~f:(fun d -> Option.is_some d.on_click)
  in
  let old_click = Option.value_exn (button ()).on_click () in
  Bonsai_driver.schedule_event driver old_click;
  assert (String.equal (button ()).text "1");
  observe driver ~first:1 ~last:2;
  display driver;
  observe driver ~first:0 ~last:1;
  display driver;
  assert (String.equal (button ()).text "0");
  Bonsai_driver.schedule_event driver old_click;
  assert (String.equal (button ()).text "0");
  let output = result driver in
  let stale =
    W.Controller.reveal (W.Output.controller output) ~focus:true (target output "b")
  in
  B.Expert.Var.set shown false;
  Bonsai_driver.flush driver;
  Bonsai_driver.trigger_lifecycles driver;
  Bonsai_driver.flush driver;
  B.Expert.Var.set shown true;
  display driver;
  Bonsai_driver.schedule_event driver stale;
  display driver;
  assert (List.is_empty (S.expanded (W.Output.state (result driver))));
  assert (Option.is_none (scroll driver));
  Bonsai_driver.Expert.invalidate_observers driver;
  print_endline "evicted content model reset; old row effect and old controller ignored";
  [%expect {| evicted content model reset; old row effect and old controller ignored |}]
;;

let%expect_test "default lazy rows request retry and collapse cancels hidden data work" =
  let parent = id "lazy" in
  let node =
    T.Node.create ~label:"Lazy" ~children:(Branch { ids = []; next = More None }) () |> ok
  in
  let loader = L.create (T.create ~roots:[ parent ] [ parent, node ] |> ok) in
  let source = B.Expert.Var.create (L.snapshot loader) in
  let publish () = B.Expert.Var.set source (L.snapshot loader) in
  let run f target =
    E.of_thunk (fun () ->
      if L.Snapshot.is_current (L.snapshot loader) target
      then (
        ignore (f loader (L.Target.parent target) |> ok : bool);
        publish ()))
  in
  let loading =
    Gpuio_bonsai.Tree_rows.Loading.create
      ~request:(run L.request)
      ~retry:(run L.retry)
      ~cancel:(fun target ->
        E.of_thunk (fun () ->
          if L.Snapshot.is_current (L.snapshot loader) target
          then (
            L.cancel loader (L.Target.parent target);
            publish ())))
      ~cancel_hidden:(fun lease state ->
        E.of_thunk (fun () ->
          if L.Lease.equal lease (L.Snapshot.lease (L.snapshot loader))
          then (
            L.cancel_hidden loader state;
            publish ())))
  in
  let driver =
    create (fun graph ->
      W.component
        (B.Expert.Var.value source)
        ~config
        ~label:"Files"
        ~initial_expanded:(B.return [ parent ])
        ~loading:(B.return loading)
        ~auto_load:(B.return false)
        graph)
  in
  display driver;
  observe driver ~first:0 ~last:2;
  display driver;
  let button text =
    descriptions (W.Output.view (result driver))
    |> List.find_exn ~f:(fun d -> String.equal d.text text && Option.is_some d.on_click)
  in
  let old_request = Option.value_exn (button "Load children").on_click () in
  Bonsai_driver.schedule_event driver old_request;
  ignore (result driver : _ W.Output.t);
  assert (L.Snapshot.queued_count (L.snapshot loader) = 1);
  let request = L.take loader |> Option.value_exn in
  publish ();
  display driver;
  assert (
    List.exists
      (descriptions (W.Output.view (result driver)))
      ~f:(fun d -> Option.is_some d.loading));
  ignore (L.fail loader request (Error.of_string "Unavailable") : L.Completion.t);
  publish ();
  display driver;
  Bonsai_driver.schedule_event driver (Option.value_exn (button "Retry").on_click ());
  ignore (result driver : _ W.Output.t);
  assert (L.Snapshot.queued_count (L.snapshot loader) = 1);
  submit driver (fun controller output ->
    W.Controller.set_expanded controller (W.Output.target output parent |> ok) false);
  display driver;
  display driver;
  assert (L.Snapshot.queued_count (L.snapshot loader) = 0);
  Bonsai_driver.schedule_event driver old_request;
  ignore (result driver : _ W.Output.t);
  assert (L.Snapshot.queued_count (L.snapshot loader) = 0);
  Bonsai_driver.Expert.invalidate_observers driver;
  print_endline
    "load, busy, failure/retry and collapse cancellation; hidden old boundary ignored";
  [%expect
    {| load, busy, failure/retry and collapse cancellation; hidden old boundary ignored |}]
;;

let%expect_test "native moves are opt-in proposals checked against latest data and policy"
  =
  let source = B.Expert.Var.create (L.snapshot (L.create (forest ()))) in
  let allowed = B.Expert.Var.create true in
  let moves = ref [] in
  let driver =
    create (fun graph ->
      W.component
        (B.Expert.Var.value source)
        ~config
        ~label:"Movable"
        ~allow_moves:(B.Expert.Var.value allowed)
        ~initial_expanded:(B.return [ id "folder" ])
        ~on_action:
          (B.return (function
             | I.Action.Move move -> E.of_thunk (fun () -> moves := move :: !moves)
             | None | Activate _ -> E.Ignore))
        graph)
  in
  display driver;
  observe driver ~first:0 ~last:4;
  display driver;
  let output = result driver in
  let key name =
    R.item_key (W.Output.projection output) (id name)
    |> Option.value_exn
    |> R.Key.to_view_key
  in
  let callback = (payload output).on_tree_input |> Option.value_exn in
  let move source destination placement =
    callback
      (Gpuio.Tree_input.Move
         { source = key source; destination = key destination; placement })
  in
  let valid = move "other" "folder" Inside in
  Bonsai_driver.schedule_event driver valid;
  display driver;
  assert (List.length !moves = 1);
  let proposal = List.hd_exn !moves in
  assert (
    I.Move.is_current
      proposal
      (B.Expert.Var.get source)
      ~state:(W.Output.state (result driver)));
  assert (T.Id.equal (I.Target.id (I.Move.source proposal)) (id "other"));
  assert (T.Id.equal (I.Target.id (I.Move.destination proposal)) (id "folder"));
  (* Proposal delivery never rewrites the application forest. *)
  assert (
    List.equal
      T.Id.equal
      (T.roots (L.Snapshot.tree (B.Expert.Var.get source)))
      (ids [ "folder"; "other"; "disabled" ]));
  List.iter
    [ move "folder" "nested" Inside; move "other" "a" Inside; move "a" "a" Before ]
    ~f:(Bonsai_driver.schedule_event driver);
  display driver;
  assert (List.length !moves = 1);
  B.Expert.Var.set allowed false;
  display driver;
  assert (not (payload (result driver)).tree_moves);
  Bonsai_driver.schedule_event driver valid;
  display driver;
  assert (List.length !moves = 1);
  B.Expert.Var.set allowed true;
  display driver;
  submit driver (fun controller output ->
    W.Controller.set_expanded controller (target output "folder") false);
  display driver;
  assert (
    I.Move.is_current
      proposal
      (B.Expert.Var.get source)
      ~state:(W.Output.state (result driver)));
  (* Captured child row keys disappear on collapse. *)
  Bonsai_driver.schedule_event driver (move "a" "other" After);
  display driver;
  assert (List.length !moves = 1);
  B.Expert.Var.set source (L.snapshot (L.create (forest ())));
  display driver;
  Bonsai_driver.schedule_event driver valid;
  display driver;
  assert (List.length !moves = 1);
  Bonsai_driver.Expert.invalidate_observers driver;
  print_endline
    "valid proposal only; no mutation; invalid endpoints, disabled policy, collapse and \
     reset ignored";
  [%expect
    {| valid proposal only; no mutation; invalid endpoints, disabled policy, collapse and reset ignored |}]
;;
