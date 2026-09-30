open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module S = Gpuio.Settings
module Panel = Gpuio_bonsai.Settings
module V = Gpuio.View
module Q = Gpuio.Container_query

let ok = Or_error.ok_exn
let key = Gpuio.Key.of_string_exn
let page_id name = S.Page_id.of_string name |> ok
let group_id name = S.Group_id.of_string name |> ok
let item_id name = S.Item_id.of_string name |> ok

let item ?(disabled = false) name =
  S.Item.create ~id:(item_id name) ~title:name ~disabled ~reset:Dirty () |> ok
;;

let group name items = S.Group.create ~id:(group_id name) ~title:name items |> ok

let page name groups =
  S.Page.create ~id:(page_id name) ~title:name ~default_open:true groups |> ok
;;

let initial =
  S.create
    [ page
        "General"
        [ group "Basics" [ item "Name"; item ~disabled:true "Locked" ]
        ; group "Advanced" [ item "Expert" ]
        ]
    ; page "Network" [ group "Connection" [ item "Proxy" ] ]
    ]
  |> ok
;;

let rec descriptions view =
  let d = V.Expert.describe view in
  d :: List.concat_map d.children ~f:descriptions
;;

let nodes output = descriptions (Panel.Output.view output)
let node output ~f = List.find_exn (nodes output) ~f
let list output = List.find_map_exn (nodes output) ~f:(fun d -> d.virtual_list)

let click output label =
  (node output ~f:(fun d ->
     (String.equal d.text label
      || Option.exists d.link ~f:(fun config ->
        String.equal (Gpuio.Link.Config.label config) label)
      || Option.exists d.accessibility ~f:(fun accessibility ->
        Option.exists
          (Gpuio.Accessibility.Expert.to_wire accessibility).label
          ~f:(String.equal label)))
     && Option.is_some d.on_click))
    .on_click
  |> Option.value_exn
  |> fun f -> f ()
;;

let observed names : Gpuio.Virtual_list.Viewport.t =
  { visible_first = 0
  ; visible_last = List.length names
  ; requested = List.map names ~f:key
  ; pinned = []
  ; anchor = None
  ; following_tail = false
  ; at_start = true
  ; at_end = false
  ; budget_exhausted = false
  }
;;

let result driver =
  Bonsai_driver.flush driver;
  Bonsai_driver.result driver |> ok
;;

let cycle driver =
  ignore (result driver : Panel.Output.t);
  Bonsai_driver.trigger_lifecycles driver;
  result driver
;;

let send driver action =
  Bonsai_driver.schedule_event driver action;
  cycle driver
;;

let observe driver names =
  let callback = (list (result driver)).on_viewport |> Option.value_exn in
  send driver (callback (observed names))
;;

let resize driver width =
  let output = result driver in
  let query = List.find_map_exn (nodes output) ~f:(fun d -> d.container_query) in
  let wire = Q.Expert.to_wire query.config ~generation:1L |> ok in
  let branch = Gpuio_protocol.Container_query_wire.Config.select wire ~width ~height:1. in
  let selected =
    Q.Expert.selection_of_wire
      wire
      { generation = 1L; sequence = 1L; branch; width; height = 1. }
    |> ok
  in
  send driver ((Option.value_exn query.on_select) selected)
;;

let create ?labels model resets activations graph =
  Panel.component
    ?labels
    ~appearance:(B.return Gpuio.Presentation.Appearance.light)
    ~model:(B.Expert.Var.value model)
    ~search:(B.return (V.text "Search slot"))
    ~on_request:
      (B.return (fun request ->
         E.of_thunk (fun () ->
           B.Expert.Var.set model (S.apply_request (B.Expert.Var.get model) request))))
    ~on_reset:(B.return (fun scope -> E.of_thunk (fun () -> resets := scope :: !resets)))
    ~render_item:(fun ~item ~layout:_ ~lifetime graph ->
      let count, set_count = B.state 0 ~equal:Int.equal graph in
      B.Edge.lifecycle
        ~on_activate:(B.return (E.of_thunk (fun () -> Int.incr activations)))
        graph;
      let open B.Let_syntax in
      let%arr item = item
      and count = count
      and set_count = set_count
      and lifetime = lifetime in
      V.button
        ~key:(key "control")
        ~on_click:(fun () ->
          Gpuio_bonsai.Managed_rows.Lifetime.guard lifetime (set_count (count + 1)))
        (sprintf "%s:%d" (S.Item.title item |> Option.value ~default:"custom") count))
    graph
;;

let driver ?labels ~optimize model resets activations =
  Bonsai_driver.create
    ~optimize
    ~clock:(Bonsai.Time_source.create ~start:Time_ns.epoch)
    (create ?labels model resets activations)
;;

let run ~optimize =
  let model = B.Expert.Var.create initial in
  let resets = ref [] in
  let activations = ref 0 in
  let driver = driver ~optimize model resets activations in
  assert (Panel.Output.active_groups (cycle driver) = 0);
  let output = observe driver [ "Basics" ] in
  assert (!activations = 2);
  let old_edit = click output "Name:0" in
  let edited = send driver old_edit in
  assert (List.exists (nodes edited) ~f:(fun d -> String.equal d.text "Name:1"));
  (* A responsive change produces style refinements, not replacement control IDs. *)
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let reconciler = Gpuio.Reconciler.create window in
  let accept output =
    let update =
      Gpuio.Reconciler.prepare
        reconciler
        ~theme:Gpuio.Theme.default
        (Some (Panel.Output.view output))
      |> ok
    in
    let operations =
      match Gpuio.Reconciler.message update with
      | Some (Apply tx) -> tx.operations
      | _ -> []
    in
    Gpuio.Reconciler.accept reconciler update |> ok;
    operations
  in
  ignore (accept edited : Gpuio_protocol.Wire.Op.t list);
  List.iter [ 479.; 480.; 700.; 320. ] ~f:(fun width ->
    let output = resize driver width in
    let row =
      node output ~f:(fun d -> Option.exists d.key ~f:(Gpuio.Key.equal (key "Name")))
    in
    let fields = Gpuio.Style.Expert.to_wire row.style ~theme:Gpuio.Theme.default |> ok in
    assert (
      List.exists fields ~f:(function
        | Fields fields ->
          List.exists fields ~f:(function
            | Direction value ->
              Int64.equal value (if Float.(width < 480.) then 1L else 0L)
            | _ -> false)
        | _ -> false));
    assert (List.exists (nodes output) ~f:(fun d -> String.equal d.text "Name:1"));
    assert (!activations = 2);
    let operations = accept output in
    assert (
      List.for_all operations ~f:(function
        | Create _ | Remove _ -> false
        | _ -> true)));
  let before = (list (cycle driver)).invalidation_revision in
  let output = send driver (click (result driver) "Collapse General") in
  assert (Int64.equal before (list output).invalidation_revision);
  assert (List.is_empty (list output).invalidated);
  let replacement =
    [ page
        "General"
        [ group "Basics" [ item "Name"; item "Locked" ]
        ; group "Advanced" [ item "Expert" ]
        ]
    ; page "Network" [ group "Connection" [ item "Proxy" ] ]
    ]
  in
  B.Expert.Var.set model (S.with_pages (B.Expert.Var.get model) replacement |> ok);
  assert (List.equal Gpuio.Key.equal (list (result driver)).invalidated [ key "Basics" ]);
  let output = cycle driver in
  assert (List.is_empty (list output).invalidated);
  let reset_effect = click output "Reset matching settings" in
  let old_viewport = (list output).on_viewport |> Option.value_exn in
  ignore (send driver (click output "Network") : Panel.Output.t);
  assert (Panel.Output.active_groups (result driver) = 0);
  ignore (send driver (old_viewport (observed [ "Basics" ])) : Panel.Output.t);
  assert (Panel.Output.active_groups (result driver) = 0);
  ignore (observe driver [ "Connection" ] : Panel.Output.t);
  ignore (send driver reset_effect : Panel.Output.t);
  let scope = List.hd_exn !resets in
  assert (List.is_empty (S.reset_targets (B.Expert.Var.get model) ~scope));
  ignore (send driver (click (result driver) "General") : Panel.Output.t);
  assert (Panel.Output.active_groups (result driver) = 0);
  ignore (send driver (old_viewport (observed [ "Basics" ])) : Panel.Output.t);
  assert (Panel.Output.active_groups (result driver) = 0);
  let output = observe driver [ "Basics" ] in
  assert (List.exists (nodes output) ~f:(fun d -> String.equal d.text "Name:0"));
  ignore (send driver old_edit : Panel.Output.t);
  assert (List.exists (nodes (result driver)) ~f:(fun d -> String.equal d.text "Name:0"));
  let output = send driver (click (result driver) "Advanced") in
  assert (Option.is_some (list output).scroll);
  let serial = (list output).scroll |> Option.value_exn in
  let output = send driver (click output "Advanced") in
  assert (
    not
      (Gpuio.Virtual_list.Scroll_request.equal
         serial
         (Option.value_exn (list output).scroll)));
  let search query =
    B.Expert.Var.set
      model
      (S.apply_request (B.Expert.Var.get model) (Search (S.Query.of_string query |> ok)))
  in
  search "missing";
  assert (Panel.Output.active_groups (cycle driver) = 0);
  assert (
    not (List.exists (nodes (result driver)) ~f:(fun d -> Option.is_some d.virtual_list)));
  search "";
  assert (Panel.Output.active_groups (cycle driver) = 0);
  Gpuio.Reconciler.close reconciler;
  Bonsai_driver.Expert.invalidate_observers driver;
  print_s
    [%sexp
      (optimize : bool)
    , ("responsive identity; navigation silence; stale page/row callbacks fenced; reset \
        scope retained"
       : string)]
;;

let%expect_test "Settings page visits and responsive field lifetime" =
  run ~optimize:false;
  run ~optimize:true;
  [%expect
    {|
    (false
     "responsive identity; navigation silence; stale page/row callbacks fenced; reset scope retained")
    (true
     "responsive identity; navigation silence; stale page/row callbacks fenced; reset scope retained")
    |}]
;;

let%expect_test
    "Settings IDs use separate namespaces and disabled custom content is inert"
  =
  List.iter
    [ String.make 256 'x'; "header"; "content"; "width-observer"; "page"; "field" ]
    ~f:(fun id ->
      let custom = S.Item.custom ~id:(item_id id) ~disabled:true () |> ok in
      let catalog = S.create [ page id [ group id [ custom ] ] ] |> ok in
      let model = B.Expert.Var.create catalog in
      let driver = driver ~optimize:true model (ref []) (ref 0) in
      ignore (cycle driver : Panel.Output.t);
      let output = observe driver [ id ] in
      let custom_row =
        node output ~f:(fun d ->
          Option.exists d.key ~f:(Gpuio.Key.equal (key id))
          && List.exists d.children ~f:(fun v ->
            let field = V.Expert.describe v in
            Option.exists field.key ~f:(Gpuio.Key.equal (key "field"))
            && List.exists field.children ~f:(fun child ->
              Option.exists
                (V.Expert.describe child).key
                ~f:(Gpuio.Key.equal (key "control")))))
      in
      let fields =
        Gpuio.Style.Expert.to_wire custom_row.style ~theme:Gpuio.Theme.default |> ok
      in
      assert (
        List.exists fields ~f:(function
          | Fields fields ->
            List.exists fields ~f:(function
              | Inert true -> true
              | _ -> false)
          | _ -> false));
      let current =
        List.filter_map (nodes output) ~f:(fun d ->
          Option.bind d.accessibility ~f:(fun a ->
            let wire = Gpuio.Accessibility.Expert.to_wire a in
            Option.map wire.current ~f:(fun _ -> wire.description)))
      in
      assert (List.length current = 1);
      let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
      let reconciler = Gpuio.Reconciler.create window in
      let pending =
        Gpuio.Reconciler.prepare
          reconciler
          ~theme:Gpuio.Theme.default
          (Some (Panel.Output.view output))
        |> ok
      in
      Gpuio.Reconciler.accept reconciler pending |> ok;
      Gpuio.Reconciler.close reconciler;
      Bonsai_driver.Expert.invalidate_observers driver);
  print_endline
    "256-byte page/group/item IDs reconcile together; custom row is inert; current page \
     is annotated";
  [%expect
    {| 256-byte page/group/item IDs reconcile together; custom row is inert; current page is annotated |}]
;;

let%expect_test "Settings rejects malformed static and dynamic localized labels" =
  let labels navigation collapse =
    Panel.Labels.create
      ~navigation
      ~empty:"Empty"
      ~resize:"Resize"
      ~reset_matches:"Matches"
      ~reset_page:"Page"
      ~current_page:"Current page"
      ~current_group:"Current group"
      ~expand:Fn.id
      ~collapse
  in
  assert (Or_error.is_error (labels "\000" Fn.id));
  let labels = labels "Navigation" (fun _ -> "") |> ok in
  let driver =
    driver ~labels ~optimize:true (B.Expert.Var.create initial) (ref []) (ref 0)
  in
  Bonsai_driver.flush driver;
  assert (Or_error.is_error (Bonsai_driver.result driver));
  Bonsai_driver.Expert.invalidate_observers driver;
  print_endline "invalid labels return errors";
  [%expect {| invalid labels return errors |}]
;;
