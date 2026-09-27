open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View
module Loader = Gpuio_eio.Tree_loading
module Tree = Gpuio_bonsai.Tree
module T = Gpuio.Tree
module I = Gpuio.Tree_interaction
module D = Source_data

module Pending = struct
  type t =
    { token : int
    ; proposal : I.Move.t
    }
end

type t =
  { loader : string Loader.t
  ; fixture : string T.t Fixture_job.t
  ; build_large : unit -> string T.t
  ; notice : string B.Expert.Var.t
  ; pending : Pending.t option B.Expert.Var.t
  ; busy : bool B.Expert.Var.t
  ; mutable serial : int
  }

let ok = Or_error.ok_exn
let get = B.Expert.Var.get
let set = B.Expert.Var.set
let px = Gpuio.Length.px_exn
let style = Gpuio.Style.create_exn

let create ~scope ~sleep ~build_large =
  let attempts = Hashtbl.create (module String) in
  let%bind.Or_error loader =
    Loader.create ~scope (D.initial ()) ~load:(fun request ->
      let parent = Gpuio.Tree_loading.Request.parent request |> T.Id.to_string in
      let generation = Gpuio.Tree_loading.Request.generation request in
      let previous =
        match Hashtbl.find attempts parent with
        | Some (old_generation, count) when Int64.equal old_generation generation -> count
        | Some _ | None -> 0
      in
      let attempt = previous + 1 in
      Hashtbl.set attempts ~key:parent ~data:(generation, attempt);
      (* Research notes deliberately expose loading long enough to inspect or
         cancel from the native UI; ordinary project children stay quick. *)
      sleep (if String.equal parent "notes" then 2. else 0.3);
      D.load ~attempt request)
  in
  let%map.Or_error fixture =
    match Fixture_job.create ~scope with
    | Ok fixture -> Ok fixture
    | Error error ->
      Loader.close loader;
      Error error
  in
  { loader
  ; fixture
  ; build_large
  ; notice =
      B.Expert.Var.create "Explore sample sources. No real files are read or changed."
  ; pending = B.Expert.Var.create None
  ; busy = B.Expert.Var.create false
  ; serial = 0
  }
;;

let cancel_build t =
  Fixture_job.cancel t.fixture;
  set t.busy false
;;

let reset t tree notice =
  cancel_build t;
  set t.pending None;
  match Loader.reset t.loader tree with
  | Ok () -> set t.notice notice
  | Error error -> set t.notice (Error.to_string_hum error)
;;

let build_large t =
  let open E.Let_syntax in
  let%bind start =
    E.of_thunk (fun () ->
      if get t.busy
      then false
      else (
        set t.busy true;
        set t.pending None;
        set t.notice "Preparing 100,000 sources…";
        true))
  in
  if not start
  then E.Ignore
  else
    Fixture_job.submit t.fixture ~f:t.build_large ~on_result:(fun result ->
      E.of_thunk (fun () ->
        set t.busy false;
        match result with
        | Error error -> set t.notice (Error.to_string_hum error)
        | Ok tree -> reset t tree "Large sample ready. Explore the source collection."))
;;

let propose t proposal =
  t.serial <- t.serial + 1;
  set t.pending (Some { Pending.token = t.serial; proposal })
;;

let cancel t token =
  if Option.exists (get t.pending) ~f:(fun pending -> pending.token = token)
  then (
    set t.pending None;
    set t.notice "Move cancelled. Sample sources are unchanged.")
;;

let approve t token state =
  match get t.pending with
  | None -> ()
  | Some pending when pending.token <> token -> ()
  | Some pending ->
    set t.pending None;
    (match
       D.approve (Loader.snapshot t.loader) ~state pending.proposal
       |> Or_error.bind ~f:(Loader.update t.loader)
     with
     | Ok () -> set t.notice "Sample source moved. No real files were modified."
     | Error error -> set t.notice (Error.to_string_hum error))
;;

let command_id name = Gpuio.Command.Id.of_string name |> ok

let render_item source ~dark ~target ~item ~controller ~lifetime _graph =
  let open B.Let_syntax in
  let%arr source = source
  and target = target
  and item = item
  and controller = controller
  and lifetime = lifetime
  and dark = dark in
  let palette = Palette.of_dark dark in
  let guard = Gpuio_bonsai.Managed_rows.Lifetime.guard lifetime in
  let archive () =
    match I.Target.capture source (D.id "archive") with
    | Error _ -> E.Ignore
    | Ok destination ->
      guard
        (E.Many
           [ Tree.Controller.reveal controller destination
           ; Tree.Controller.propose_move controller ~source:target ~destination Inside
           ])
  in
  let leaf =
    match T.Node.children item.Gpuio.Tree_rows.Item.node with
    | Leaf -> true
    | Branch _ -> false
  in
  let commands =
    Gpuio.Command.Registry.create
      [ Gpuio.Command.create
          ~id:(command_id "open-source")
          ~label:"Read source note"
          ~on_invoke:(fun () -> guard (Tree.Controller.activate controller target))
          ()
        |> ok
      ; Gpuio.Command.create
          ~id:(command_id "archive-source")
          ~label:"Archive source"
          ~enabled:leaf
          ~on_invoke:archive
          ()
        |> ok
      ]
    |> ok
  in
  let menu =
    Gpuio.Menu.create
      ~label:"Actions"
      [ Command (command_id "open-source"); Command (command_id "archive-source") ]
    |> ok
  in
  V.command_scope
    ~commands
    ~style:(style [ Grow 1.; Min_width (px 0.) ])
    [ V.context_menu
        ~menu
        (V.row
           ~style:(style [ Grow 1.; Min_width (px 0.); Align_items Center; Gap (px 6.) ])
           [ V.text
               ~style:
                 (style
                    [ Grow 1.
                    ; Min_width (px 0.)
                    ; White_space No_wrap
                    ; Text_overflow Ellipsis
                    ])
               (T.Node.label item.node)
           ; V.menu_button
               ~menu
               ~style:
                 (style
                    [ Shrink 0.
                    ; Font_size 11.
                    ; Padding (px 5.)
                    ; Radius 6.
                    ; Background (Gpuio.Background.solid palette.raised)
                    ; Foreground palette.muted
                    ])
               ()
           ])
    ]
;;

let component t ~active ~dark graph =
  let source = Loader.value t.loader in
  let open B.Let_syntax in
  let tree_style =
    let%arr dark = dark in
    let palette = Palette.of_dark dark in
    style
      [ Width (Gpuio.Length.percent_exn 100.)
      ; Height (px 300.)
      ; Shrink 0.
      ; Background (Gpuio.Background.solid palette.surface)
      ; Foreground palette.accent
      ; Border_width 1.
      ; Border_color palette.line
      ; Radius 10.
      ]
  in
  let initial_expanded =
    let%arr source = source in
    let tree = Gpuio.Tree_loading.Snapshot.tree source in
    if Option.is_some (T.find tree (D.id "project")) then [ D.id "project" ] else []
  in
  let output =
    Tree.component
      source
      ~config:
        (Gpuio.Virtual_list.Config.create
           ~height:(Fixed 34.)
           ~overscan:68.
           ~max_active:24
           ()
         |> ok)
      ~key:(Gpuio.Key.of_string_exn "sample-source-tree")
      ~label:"Sample workspace sources"
      ~style:tree_style
      ~mode:(B.return Gpuio.Tree_state.Mode.Multiple)
      ~initial_expanded
      ~loading:(B.return (Loader.controls t.loader))
      ~auto_load:active
      ~allow_moves:(B.return true)
      ~render_item:(render_item source ~dark)
      ~on_action:
        (B.return (function
           | I.Action.None -> E.Ignore
           | Move proposal -> E.of_thunk (fun () -> propose t proposal)
           | Activate id ->
             E.of_thunk (fun () ->
               match
                 T.find (Gpuio.Tree_loading.Snapshot.tree (Loader.snapshot t.loader)) id
               with
               | None -> ()
               | Some node -> set t.notice ("Source note: " ^ T.Node.data node))))
      graph
  in
  let peek = B.peek output graph in
  let%arr output = output
  and peek = peek
  and source = source
  and dark = dark
  and pending = B.Expert.Var.value t.pending
  and busy = B.Expert.Var.value t.busy
  and notice = B.Expert.Var.value t.notice in
  let palette = Palette.of_dark dark in
  let button ?(disabled = false) label on_click =
    V.button
      ~disabled
      ~on_click
      label
      ~style:
        (style
           [ Foreground palette.text
           ; Background (Gpuio.Background.solid palette.raised)
           ; Border_width 1.
           ; Border_color palette.line
           ; Padding (px 8.)
           ; Radius 8.
           ])
  in
  let tree = Gpuio.Tree_loading.Snapshot.tree source in
  let find id = T.find tree id in
  let reveal id =
    match output with
    | Error _ -> E.Ignore
    | Ok output ->
      (match Tree.Output.target output id with
       | Error _ -> E.Ignore
       | Ok target ->
         Tree.Controller.reveal (Tree.Output.controller output) ~focus:true target)
  in
  let selection =
    match output with
    | Error _ -> []
    | Ok output -> Gpuio.Tree_state.selected (Tree.Output.state output)
  in
  let selected =
    match selection with
    | [ id ] -> Option.map (find id) ~f:(fun node -> id, node)
    | [] | _ :: _ :: _ -> None
  in
  let dialog =
    Option.map pending ~f:(fun pending ->
      let cancel = E.of_thunk (fun () -> cancel t pending.token) in
      let confirm =
        E.bind peek ~f:(function
          | B.Computation_status.Inactive | Active (Error _) -> E.Ignore
          | Active (Ok output) ->
            E.of_thunk (fun () -> approve t pending.token (Tree.Output.state output)))
      in
      let name target =
        Option.value_map
          (find (I.Target.id target))
          ~default:"Unavailable source"
          ~f:T.Node.label
      in
      V.column
        ~style:(style [ Gap (px 16.) ])
        [ V.text
            ~style:(style [ Font_size 22.; Font_weight 600 ])
            "Move this sample source?"
        ; V.text
            (sprintf
               "%s → %s"
               (name (I.Move.source pending.proposal))
               (name (I.Move.destination pending.proposal)))
        ; V.text "This changes only the in-memory sample. Real files are never modified."
        ; V.row
            ~style:(style [ Gap (px 10.) ])
            [ button "Cancel move" cancel; button "Confirm move" confirm ]
        ])
  in
  let location id =
    match T.position tree id with
    | Some { parent = Some parent; _ } ->
      Option.value_map (find parent) ~default:"Workspace" ~f:T.Node.label
    | Some { parent = None; _ } | None -> "Workspace"
  in
  let appearance =
    if dark
    then Gpuio.Presentation.Appearance.dark
    else Gpuio.Presentation.Appearance.light
  in
  V.column
    ~style:(style [ Gap (px 14.); Min_width (px 0.) ])
    [ V.row
        [ Gpuio.Presentation.badge appearance ~size:Small ~tone:Accent "SAMPLE SOURCES" ]
    ; V.text ~style:(style [ Font_size 23.; Font_weight 600 ]) "Context, close at hand."
    ; V.text
        ~style:(style [ Foreground palette.muted; Font_size 13.; Line_height (px 20.) ])
        "Expand collections, inspect a source, or organize the sample. Research notes \
         demonstrate a failed load and retry."
    ; (if T.length tree = 0
       then
         Gpuio.Presentation.empty_state
           appearance
           ~title:"No sources yet"
           ~description:"Restore the sample workspace to explore its source collections."
           ~style:
             (style
                [ Height (px 220.)
                ; Background (Gpuio.Background.solid palette.surface)
                ; Border_color palette.line
                ; Radius 10.
                ])
           ~actions:
             (button
                "Restore sample sources"
                (E.of_thunk (fun () -> reset t (D.initial ()) "Sample sources restored.")))
           ()
       else (
         match output with
         | Error error -> V.text (Error.to_string_hum error)
         | Ok output -> Tree.Output.view output))
    ; V.text
        ~style:(style [ Font_size 12.; Foreground palette.muted ])
        (sprintf
           "%s loaded · %d selected"
           (Int.to_string_hum ~delimiter:',' (T.length tree))
           (List.length selection))
    ; (match selected with
       | None ->
         V.text
           (if List.is_empty selection
            then "Select a source to inspect it."
            else "Multiple sources selected")
       | Some (id, node) ->
         V.column
           ~style:(style [ Gap (px 6.) ])
           [ V.text ~style:(style [ Font_weight 600 ]) ("Selected: " ^ T.Node.label node)
           ; V.text
               ~style:(style [ Foreground palette.muted; Font_size 12. ])
               ("Location: " ^ location id)
           ; V.text
               ~style:
                 (style [ Foreground palette.muted; Font_size 13.; Line_height (px 20.) ])
               (T.Node.data node)
           ])
    ; V.text ~style:(style [ Foreground palette.accent; Font_size 12. ]) notice
    ; V.row
        ~style:(style [ Gap (px 8.); Wrap Wrap ])
        [ button
            ~disabled:(Option.is_none (find (D.id "app")))
            "Reveal main source"
            (reveal (D.id "app"))
        ; button
            ~disabled:(Option.is_none (find (D.id "source-099997")))
            "Reveal last source"
            (reveal (D.id "source-099997"))
        ]
    ; V.row
        ~style:(style [ Gap (px 8.); Wrap Wrap ])
        [ button
            "Sample sources"
            (E.of_thunk (fun () -> reset t (D.initial ()) "Sample sources restored."))
        ; button ~disabled:busy "Load 100,000 sources" (build_large t)
        ; button
            "Empty workspace"
            (E.of_thunk (fun () ->
               reset t (D.empty ()) "No sources in this sample workspace."))
        ]
    ; V.alert_dialog
        ?key:
          (Option.map pending ~f:(fun pending ->
             Gpuio.Key.of_string_exn (sprintf "source-move-%d" pending.token)))
        ~style:
          (style
             [ Background (Gpuio.Background.solid palette.surface)
             ; Foreground palette.text
             ; Border_color palette.line
             ])
        ~config:
          (Gpuio.Alert_dialog.Config.create ~label:"Review source move" ~width:460. ()
           |> ok)
        ~on_dismiss:(fun _ ->
          match pending with
          | None -> E.Ignore
          | Some pending -> E.of_thunk (fun () -> cancel t pending.token))
        dialog
    ]
;;
