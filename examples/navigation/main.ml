open Core
open Gpuio
module App = Gpuio_eio.App
module Scope = Gpuio_eio.Scope
module Input = Gpuio_eio.Text_input
module B = Bonsai.Cont
module E = Bonsai.Effect
module UI = Gpuio_bonsai.View

let ok = Or_error.ok_exn
let px = Length.px_exn
let style = Style.create_exn
let full = Length.percent_exn 100.

module Modal = struct
  type t =
    | Closed
    | Drawer of Sheet.Edge.t
    | Confirmation of Sheet.Edge.t
end

module Action = struct
  type t =
    | Page of Pagination.Request.t
    | Total of int
    | Toggle_details
    | Toggle_lazy
    | Home
    | Sidebar of Sidebar.Request.t
    | Collapse_mode of Sidebar.Collapse.t
    | Inspect_sidebar
    | Route_back
    | Route_forward
    | Route_replace
    | Preview_open of bool
    | Open_drawer of Sheet.Edge.t
    | Close_drawer
    | Confirm_drawer_close
    | Cancel_drawer_close
end

module Model = struct
  type t =
    { pages : Pagination.t
    ; details : bool
    ; lazy_content : bool
    ; sidebar : Sidebar.t
    ; inspected : bool
    ; preview_open : bool
    ; modal : Modal.t
    ; routes : string Navigation_stack.t
    }

  let sidebar_id value = Sidebar.Id.of_string value |> ok

  let sidebar =
    let item ?(children = []) ?(disabled = false) compact_label label =
      Sidebar.Item.create
        ~id:(sidebar_id label)
        ~label
        ~compact_label
        ~children
        ~disabled
        ()
      |> ok
    in
    Sidebar.create
      ~selected:(Some (sidebar_id "Inbox"))
      ~expanded:[ sidebar_id "Archive" ]
      ~groups:
        [ Sidebar.Group.create
            ~id:(sidebar_id "destinations")
            ~label:"WORKSPACE"
            [ item "A" "Archive" ~children:[ item "I" "Inbox"; item "S" "Starred" ]
            ; item "G" "Settings"
            ; item "L" "Locked" ~disabled:true
            ]
          |> ok
        ]
      ()
    |> ok
  ;;

  let initial =
    { pages = Pagination.create ~total_pages:Pagination.max_pages () |> ok
    ; details = true
    ; lazy_content = true
    ; sidebar
    ; preview_open = false
    ; modal = Closed
    ; inspected = false
    ; routes =
        Navigation_stack.create
          (List.map [ "Draft"; "Preview" ] ~f:(fun label ->
             Navigation_stack.Entry.create
               ~id:(Navigation_stack.Id.of_string label |> ok)
               ~label
               label
             |> ok))
        |> ok
        |> Navigation_stack.pop
    }
  ;;

  let apply t = function
    | Action.Page request -> { t with pages = Pagination.apply_request t.pages request }
    | Total total -> { t with pages = Pagination.with_total_pages t.pages total |> ok }
    | Toggle_details -> { t with details = not t.details }
    | Toggle_lazy -> { t with lazy_content = not t.lazy_content }
    | Home -> { t with pages = Pagination.apply_request t.pages Pagination.Request.first }
    | Sidebar request -> { t with sidebar = Sidebar.apply_request t.sidebar request }
    | Collapse_mode collapse ->
      { t with sidebar = Sidebar.with_collapse t.sidebar collapse }
    | Inspect_sidebar -> { t with inspected = true }
    | Route_back -> { t with routes = Navigation_stack.pop t.routes }
    | Route_forward -> { t with routes = Navigation_stack.forward t.routes }
    | Preview_open preview_open -> { t with preview_open }
    | Open_drawer edge -> { t with modal = Drawer edge }
    | Close_drawer -> { t with modal = Closed }
    | Confirm_drawer_close ->
      (match t.modal with
       | Closed | Confirmation _ -> t
       | Drawer edge -> { t with modal = Confirmation edge })
    | Cancel_drawer_close ->
      (match t.modal with
       | Closed | Drawer _ -> t
       | Confirmation edge -> { t with modal = Drawer edge })
    | Route_replace ->
      let entry =
        Navigation_stack.Entry.create
          ~id:(Navigation_stack.Id.of_string "Replacement" |> ok)
          ~label:"Replacement"
          "A replacement route has a fresh native page."
        |> ok
      in
      { t with routes = Navigation_stack.replace t.routes entry |> ok }
  ;;
end

module Observation = struct
  type t =
    { model : Model.t
    ; inject : Action.t -> unit E.t
    ; editor : Input.t
    ; route_editor : Input.t
    ; preview_editor : Input.t
    }
end

let component
      ~sidebar_side
      ~motion_test
      ~set_motion
      ~icon
      ~loaded
      ~observed
      ~lazy_activations
      ~lazy_deactivations
      window
      graph
  =
  let model, inject =
    B.state_machine0
      ~default_model:Model.initial
      ~apply_action:(fun _ model action -> Model.apply model action)
      graph
  in
  let editor =
    Input.create
      window
      ~initial_text:"A draft that survives pagination."
      ~config:
        (B.return
           (Text_input.Config.create ~mode:Single_line ~label:"Retained draft" () |> ok))
      graph
  in
  let open B.Let_syntax in
  let route_editor =
    Input.create
      window
      ~initial_text:"A draft that survives navigation."
      ~config:
        (B.return
           (Text_input.Config.create ~mode:Single_line ~label:"Route draft" () |> ok))
      graph
  in
  let preview_editor =
    Input.create
      window
      ~initial_text:"A private note that survives closing the preview."
      ~config:
        (B.return
           (Text_input.Config.create ~mode:Single_line ~label:"Contributor note" () |> ok))
      graph
  in
  let lazy_enabled =
    let%arr model = model in
    model.Model.lazy_content
  in
  let lazy_view =
    match%sub lazy_enabled with
    | false -> B.return (UI.text "Lazy computation is inactive.")
    | true ->
      B.Edge.lifecycle
        ~on_activate:(B.return (E.of_thunk (fun () -> incr lazy_activations)))
        ~on_deactivate:(B.return (E.of_thunk (fun () -> incr lazy_deactivations)))
        graph;
      let count, set_count = B.state 0 graph in
      let%arr count = count
      and set_count = set_count in
      UI.button
        ~on_click:(set_count (count + 1))
        (sprintf "Lazy component counter · %d" count)
  in
  B.Edge.after_display
    (let%arr model = model
     and inject = inject
     and editor = editor
     and route_editor = route_editor
     and preview_editor = preview_editor in
     E.of_thunk (fun () ->
       observed
       := Some { Observation.model; inject; editor; route_editor; preview_editor }))
    graph;
  let loaded = B.Expert.Var.value loaded in
  let icon = B.Expert.Var.value icon in
  let%arr model = model
  and inject = inject
  and editor = editor
  and route_editor = route_editor
  and preview_editor = preview_editor
  and lazy_view = lazy_view
  and loaded = loaded
  and icon = icon in
  let page = Option.value (Pagination.current model.pages) ~default:0 in
  let crumbs =
    [ "workspace", "Workspace"; "archive", "Archive"; "page", sprintf "Page %d" page ]
    |> List.map ~f:(fun (id, label) ->
      Choice.create ~id:(Choice.Id.of_string id |> ok) ~label () |> ok)
    |> Choice.Collection.create
    |> ok
  in
  let card children =
    UI.column
      ~style:
        (style
           [ Padding (px 20.)
           ; Gap (px 14.)
           ; Radius 14.
           ; Background (Background.solid (Color.rgb_exn 0x202c43))
           ])
      children
  in
  let inspect_id = Command.Id.of_string "sidebar.inspect" |> ok in
  let menu = Menu.create ~label:"Destination actions" [ Command inspect_id ] |> ok in
  let commands =
    Command.Registry.create
      [ Command.create
          ~id:inspect_id
          ~label:"Inspect destination"
          ~on_invoke:(fun () -> inject Inspect_sidebar)
          ()
        |> ok
      ]
    |> ok
  in
  let sidebar =
    Sidebar.view
      model.sidebar
      ~side:sidebar_side
      ~appearance:
        (Sidebar.Appearance.create
           ~motion:
             (if motion_test
              then
                Sidebar.Motion.create
                  ~duration:(Time_ns.Span.of_sec 2.)
                  ~easing:Animation.Easing.linear
                  ()
                |> ok
              else Sidebar.Motion.default)
           ()
         |> ok)
      ~hidden:Retain
      ~header:(fun ~compact ->
        UI.text
          ~style:(style [ Font_weight 700; Padding (px 8.) ])
          (if compact then "A" else "ASTER"))
      ~footer:(fun ~compact -> UI.text (if compact then "●" else "●  Local workspace"))
      ~decorate:(fun item ->
        let is_archive = String.equal (Sidebar.Item.label item) "Archive" in
        Sidebar.Decoration.create
          ?icon:(if is_archive then icon else None)
          ?suffix:
            (if String.equal (Sidebar.Item.label item) "Inbox"
             then Some (UI.text "12")
             else None)
          ~context_menu:menu
          ())
      ~on_request:(fun request -> inject (Sidebar request))
      ()
    |> ok
  in
  let selected =
    Sidebar.selected model.sidebar
    |> Option.bind ~f:(Sidebar.find model.sidebar)
    |> Option.value_map ~default:"None" ~f:Sidebar.Item.label
  in
  let drawer =
    let edge, content =
      match model.modal with
      | Modal.Closed -> Sheet.Edge.Right, None
      | Drawer edge | Confirmation edge ->
        let confirming =
          match model.modal with
          | Confirmation _ -> true
          | Closed | Drawer _ -> false
        in
        ( edge
        , Some
            (UI.column
               ~style:(style [ Gap (px 16.) ])
               [ UI.text
                   ~style:(style [ Font_size 22.; Font_weight 700 ])
                   "Workspace details"
               ; UI.text
                   "This drawer stays modal until an accepted application update closes \
                    it."
               ; UI.button
                   ~on_click:(inject Confirm_drawer_close)
                   "Close with confirmation"
               ; UI.button ~on_click:(inject Close_drawer) "Done"
               ; UI.alert_dialog
                   ~key:(Key.of_string_exn "drawer-confirmation")
                   ~config:
                     (Alert_dialog.Config.create ~label:"Close workspace details?" ()
                      |> ok)
                   ~on_dismiss:(fun _ -> inject Cancel_drawer_close)
                   (if confirming
                    then
                      Some
                        (UI.column
                           ~style:(style [ Gap (px 14.) ])
                           [ UI.text
                               ~style:(style [ Font_size 20.; Font_weight 600 ])
                               "Close workspace details?"
                           ; UI.text
                               "Backdrop clicks leave this confirmation open. Escape \
                                returns to the drawer."
                           ; UI.row
                               ~style:(style [ Gap (px 10.) ])
                               [ UI.button
                                   ~on_click:(inject Cancel_drawer_close)
                                   "Keep open"
                               ; UI.button ~on_click:(inject Close_drawer) "Close details"
                               ]
                           ])
                    else None)
               ]) )
    in
    UI.sheet
      ~key:(Key.of_string_exn "workspace-drawer")
      ~config:(Sheet.Config.create ~label:"Workspace details" ~edge ~extent:360. () |> ok)
      ~on_dismiss:(fun _ -> inject Close_drawer)
      content
  in
  let content =
    UI.column
      ~style:
        (style
           [ Grow 1.
           ; Min_width (px 0.)
           ; Overflow_y Scroll
           ; Height full
           ; Padding (px 28.)
           ; Gap (px 20.)
           ; Background (Background.solid (Color.token_exn "background"))
           ; Foreground (Color.token_exn "foreground")
           ])
      [ UI.row
          ~style:(style [ Gap (px 8.); Wrap Wrap ])
          ([ Sidebar.toggle
               model.sidebar
               ~on_request:(fun request -> inject (Sidebar request))
               ()
           ; UI.button ~on_click:(inject (Collapse_mode Icon)) "Icon mode"
           ; UI.button ~on_click:(inject (Collapse_mode Offcanvas)) "Offcanvas mode"
           ]
           @
           if motion_test
           then
             [ UI.button
                 ~on_click:(set_motion Animation.Preference.Reduce)
                 "Reduce motion"
             ]
           else [])
      ; UI.text ("Selected destination: " ^ selected)
      ; UI.text
          (if model.inspected
           then "Destination inspected"
           else "Use Shift-F10 on a destination for its context menu.")
      ; UI.text
          ~style:
            (style
               [ Font_size 12.; Foreground (Color.rgb_exn 0xa0b7ed); Font_weight 600 ])
          "GPUIO / NAVIGATION LAB"
      ; UI.text
          ~style:(style [ Font_size 30.; Font_weight 700 ])
          "A place for every detail."
      ; UI.row
          ~style:(style [ Gap (px 8.); Wrap Wrap ])
          (List.map
             [ Sheet.Edge.Left, "Left"; Right, "Right"; Top, "Top"; Bottom, "Bottom" ]
             ~f:(fun (edge, label) ->
               UI.button ~on_click:(inject (Open_drawer edge)) (label ^ " drawer")))
      ; UI.hover_card
          ~key:(Key.of_string_exn "contributor-preview")
          ~config:
            (Hover_card.Config.create
               ~label:"Contributor preview"
               ~open_state:(Controlled model.preview_open)
               ()
             |> ok)
          ~on_open_change:(fun open_ -> inject (Preview_open open_))
          ~anchor:(UI.button ~on_click:(E.return ()) "Contributor preview")
          ~content:
            (UI.column
               ~style:(style [ Gap (px 12.) ])
               [ UI.text
                   ~style:(style [ Font_size 18.; Font_weight 600 ])
                   "Avery · Design engineer"
               ; UI.text
                   "Hover or focus the trigger to preview. Tab enters this card without \
                    trapping focus."
               ; Input.view ~style:(style [ Width full; Height (px 40.) ]) preview_editor
               ; UI.button ~on_click:(inject (Preview_open false)) "Close preview"
               ])
          ()
      ; Navigation.breadcrumbs
          crumbs
          ~label:"Archive path"
          ~current_description:"Current location"
          ~on_navigate:(fun _ -> inject Home)
          ()
        |> ok
      ; card
          [ UI.text
              ~style:(style [ Font_size 19.; Font_weight 600 ])
              (sprintf "Archive · page %d" page)
          ; UI.text
              (if loaded
               then
                 "Archive metadata loaded. Data tasks are independent of visible content."
               else "Loading archive metadata…")
          ; Navigation.pagination
              model.pages
              ~on_request:(fun request -> inject (Page request))
              ()
            |> ok
          ; UI.row
              ~style:(style [ Gap (px 8.); Wrap Wrap ])
              [ UI.button ~on_click:(inject (Total 3)) "Shrink to 3 pages"
              ; UI.button
                  ~on_click:(inject (Total Pagination.max_pages))
                  "Restore billion-page archive"
              ]
          ]
      ; card
          [ UI.text
              ~style:(style [ Font_size 19.; Font_weight 600 ])
              "Native route transitions"
          ; UI.row
              ~style:(style [ Gap (px 8.) ])
              [ UI.button
                  ~disabled:(not (Navigation_stack.can_pop model.routes))
                  ~on_click:(inject Route_back)
                  "Back to draft"
              ; UI.button
                  ~disabled:(not (Navigation_stack.can_forward model.routes))
                  ~on_click:(inject Route_forward)
                  "Open preview"
              ; UI.button ~on_click:(inject Route_replace) "Replace route"
              ]
          ; UI.navigation_stack
              model.routes
              ~label:"Workspace routes"
              ~hidden:Retain
              ~style:(style [ Width full; Height (px 130.); Shrink 0. ])
              ~page_style:
                (style [ Width full; Height full; Padding (px 12.); Gap (px 10.) ])
              ~content:(fun entry ->
                let label = Navigation_stack.Entry.label entry in
                [ UI.text ~style:(style [ Font_weight 600 ]) label
                ; (if String.equal label "Draft"
                   then
                     Input.view
                       ~style:(style [ Width full; Height (px 40.) ])
                       route_editor
                   else UI.text (Navigation_stack.Entry.data entry))
                ; UI.text
                    "Route selection changes immediately; presentation and focus stay \
                     native."
                ])
              ()
          ]
      ; UI.disclosure
          ~key:(Key.of_string_exn "draft")
          ~label:"Retained draft"
          ~expanded:model.details
          ~hidden:Retain
          ~on_toggle:(inject Toggle_details)
          [ card
              [ Input.view ~style:(style [ Width full; Height (px 40.) ]) editor
              ; UI.text
                  "Collapsing this panel keeps the editor and its Bonsai computation \
                   alive."
              ]
          ]
      ; card
          [ UI.row
              ~style:(style [ Gap (px 12.); Align_items Center ])
              [ UI.button
                  ~on_click:(inject Toggle_lazy)
                  (if model.lazy_content
                   then "Deactivate lazy content"
                   else "Activate lazy content")
              ; lazy_view
              ]
          ; UI.text
              "This switch changes a Bonsai branch. It does not cancel the archive data \
               scope."
          ]
      ]
  in
  let content =
    UI.with_accessibility
      content
      (Accessibility.create ~role:Group ~label:"Navigation content" () |> ok)
    |> ok
  in
  UI.command_scope
    ~commands
    ~style:(style [ Width full; Height full ])
    [ UI.row
        ~style:(style [ Width full; Height full; Min_width (px 0.); Min_height (px 0.) ])
        (match sidebar_side with
         | Sidebar.Side.Left -> [ sidebar; content ]
         | Right -> [ content; sidebar ])
    ; drawer
    ]
;;

let () =
  let self_test = Array.exists (Sys.get_argv ()) ~f:(String.equal "--self-test") in
  let motion_test = Array.exists (Sys.get_argv ()) ~f:(String.equal "--motion-test") in
  let sidebar_side =
    if Array.exists (Sys.get_argv ()) ~f:(String.equal "--right-sidebar")
    then Sidebar.Side.Right
    else Left
  in
  let observed = ref None
  and lazy_activations = ref 0
  and lazy_deactivations = ref 0 in
  let completed = ref false
  and data_completed = ref false
  and data_cancelled = ref false in
  App.run
    ~motion:(if motion_test then Full else System)
    (fun env app ->
       let clock = Eio.Stdenv.clock env in
       let loaded = B.Expert.Var.create false in
       let icon = B.Expert.Var.create None in
       Sidebar_icons.load env app icon;
       let released, release = Eio.Promise.create () in
       let window =
         App.open_window
           app
           ~focus:true
           ~title:"GPUIO Navigation Lab"
           ~width:1120.
           ~height:760.
           (component
              ~sidebar_side
              ~motion_test
              ~set_motion:(fun policy -> E.of_thunk (fun () -> App.set_motion app policy))
              ~icon
              ~loaded
              ~observed
              ~lazy_activations
              ~lazy_deactivations)
         |> ok
       in
       let data_scope =
         Scope.child (App.Window.scope window) ~name:"archive-data" |> ok
       in
       ignore
         (Scope.start
            data_scope
            ~f:(fun () ->
              if self_test then Eio.Promise.await released else Eio.Time.sleep clock 0.4;
              data_completed := true)
            ~on_result:(fun result ->
              E.of_thunk (fun () ->
                ok result;
                B.Expert.Var.set loaded true))
          |> ok
          : Scope.Task.t);
       (* A long-lived subscription belongs to the data owner, not either panel. *)
       ignore
         (Scope.start
            data_scope
            ~f:(fun () ->
              Exn.protect ~f:Eio.Fiber.await_cancel ~finally:(fun () ->
                data_cancelled := true))
            ~on_result:(fun result -> E.of_thunk (fun () -> ok result))
          |> ok
          : Scope.Task.t);
       if self_test
       then
         ignore
           (Scope.start
              (App.Window.scope window)
              ~f:(fun () ->
                Eio.Time.with_timeout_exn clock 20. (fun () ->
                  let await ready =
                    while not (ready ()) do
                      Eio.Time.sleep clock 0.005
                    done
                  in
                  let on_ui ui_effect =
                    let result, resolve = Eio.Promise.create () in
                    Scope.Expert.enqueue (App.Window.scope window) (fun () ->
                      E.Expert.handle (E.map ui_effect ~f:(Eio.Promise.resolve resolve)));
                    Eio.Promise.await result
                  in
                  let latest () = Option.value_exn !observed in
                  let model () = (latest ()).model in
                  let send action = on_ui ((latest ()).inject action) in
                  let read () =
                    match on_ui (Input.read_snapshot (latest ()).editor) with
                    | Ok snapshot -> snapshot
                    | Error error -> raise_s [%sexp (error : Text_input.Command_error.t)]
                  in
                  let route_read () =
                    match on_ui (Input.read_snapshot (latest ()).route_editor) with
                    | Ok snapshot -> snapshot
                    | Error error -> raise_s [%sexp (error : Text_input.Command_error.t)]
                  in
                  await (fun () ->
                    Option.exists !observed ~f:(fun o ->
                      Option.is_some (Input.snapshot o.editor))
                    && !lazy_activations = 1);
                  (match
                     on_ui
                       (Input.replace
                          (latest ()).editor
                          ~selection:End
                          ~undo:Record
                          "Preserved draft 👩🏽‍💻")
                   with
                   | Ok _ -> ()
                   | Error error -> raise_s [%sexp (error : Text_input.Command_error.t)]);
                  let before = read () in
                  let route_before = route_read () in
                  let preview_snapshot () =
                    match on_ui (Input.read_snapshot (latest ()).preview_editor) with
                    | Ok snapshot -> snapshot
                    | Error error -> raise_s [%sexp (error : Text_input.Command_error.t)]
                  in
                  let preview_before = preview_snapshot () in
                  assert (Result.is_error (on_ui (Input.focus (latest ()).preview_editor)));
                  send (Preview_open true);
                  await (fun () -> (model ()).preview_open);
                  assert (Result.is_ok (on_ui (Input.focus (latest ()).preview_editor)));
                  send (Preview_open false);
                  await (fun () -> not (model ()).preview_open);
                  assert (Result.is_error (on_ui (Input.focus (latest ()).preview_editor)));
                  assert (Text_input.Snapshot.equal preview_before (preview_snapshot ()));
                  send (Open_drawer Right);
                  await (fun () ->
                    match (model ()).modal with
                    | Drawer _ -> true
                    | Closed | Confirmation _ -> false);
                  assert (Result.is_error (on_ui (Input.focus (latest ()).editor)));
                  send Confirm_drawer_close;
                  await (fun () ->
                    match (model ()).modal with
                    | Confirmation _ -> true
                    | Closed | Drawer _ -> false);
                  send Cancel_drawer_close;
                  await (fun () ->
                    match (model ()).modal with
                    | Drawer _ -> true
                    | Closed | Confirmation _ -> false);
                  send Close_drawer;
                  await (fun () ->
                    match (model ()).modal with
                    | Closed -> true
                    | Drawer _ | Confirmation _ -> false);
                  assert (Text_input.Snapshot.equal before (read ()));
                  send Route_forward;
                  await (fun () -> Navigation_stack.can_pop (model ()).routes);
                  assert (Text_input.Snapshot.equal route_before (route_read ()));
                  assert (Result.is_error (on_ui (Input.focus (latest ()).route_editor)));
                  send Route_replace;
                  await (fun () ->
                    Option.exists
                      (Navigation_stack.current (model ()).routes)
                      ~f:(fun entry ->
                        String.equal (Navigation_stack.Entry.label entry) "Replacement"));
                  send Route_back;
                  await (fun () -> not (Navigation_stack.can_pop (model ()).routes));
                  assert (Text_input.Snapshot.equal route_before (route_read ()));
                  send Toggle_details;
                  await (fun () -> not (model ()).details);
                  assert (Text_input.Snapshot.equal before (read ()));
                  assert (
                    Result.equal
                      Text_input.Snapshot.equal
                      Text_input.Command_error.equal
                      (on_ui (Input.focus (latest ()).editor))
                      (Error Focus_blocked));
                  assert (!lazy_deactivations = 0);
                  send Toggle_lazy;
                  await (fun () -> !lazy_deactivations = 1);
                  assert (Scope.is_active data_scope && not !data_cancelled);
                  Eio.Promise.resolve release ();
                  await (fun () -> B.Expert.Var.get loaded);
                  assert !data_completed;
                  on_ui
                    (E.Many
                       (List.init 3 ~f:(fun _ ->
                          (latest ()).inject (Page Pagination.Request.next))));
                  await (fun () ->
                    Option.equal Int.equal (Pagination.current (model ()).pages) (Some 4));
                  send (Total 2);
                  await (fun () ->
                    Option.equal Int.equal (Pagination.current (model ()).pages) (Some 2));
                  send (Page (Pagination.Request.page 100 |> ok));
                  send Toggle_details;
                  await (fun () -> (model ()).details);
                  assert (
                    Option.equal Int.equal (Pagination.current (model ()).pages) (Some 2));
                  assert (
                    String.equal (Text_input.Snapshot.text (read ())) "Preserved draft 👩🏽‍💻");
                  send Toggle_lazy;
                  await (fun () -> !lazy_activations = 2);
                  assert (Scope.is_active data_scope && not !data_cancelled);
                  send (Sidebar Toggle_collapsed);
                  await (fun () -> Sidebar.is_collapsed (model ()).sidebar);
                  send (Sidebar (Select (Model.sidebar_id "Inbox")));
                  send (Sidebar (Select (Model.sidebar_id "Settings")));
                  await (fun () ->
                    Option.equal
                      Sidebar.Id.equal
                      (Sidebar.selected (model ()).sidebar)
                      (Some (Model.sidebar_id "Settings")));
                  send (Collapse_mode Offcanvas);
                  await (fun () ->
                    Sidebar.Collapse.equal (Sidebar.collapse (model ()).sidebar) Offcanvas);
                  send (Sidebar Toggle_collapsed);
                  await (fun () -> not (Sidebar.is_collapsed (model ()).sidebar));
                  assert (
                    Sidebar.is_expanded (model ()).sidebar (Model.sidebar_id "Archive"));
                  assert (
                    String.equal (Text_input.Snapshot.text (read ())) "Preserved draft 👩🏽‍💻");
                  await (fun () -> Option.is_some (B.Expert.Var.get icon));
                  completed := true))
              ~on_result:(fun result ->
                E.of_thunk (fun () ->
                  App.Window.close window;
                  ok result))
            |> ok
            : Scope.Task.t));
  if self_test
  then (
    assert (!completed && !data_completed && !data_cancelled);
    assert (!lazy_activations = 2 && !lazy_deactivations = 2);
    Eio.traceln
      "GPUIO_NAVIGATION_PUBLIC_OK: bounded pages, queued intents, shrink, retained \
       draft, lazy lifecycle, independent Eio data and scoped teardown")
;;
