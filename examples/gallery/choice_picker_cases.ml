open Core
open Gpuio
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View
module P = Choice_picker
module Controller = Gpuio_eio.Choice_picker
module State = Gpuio_gallery_model.Picker_state

let ok = Or_error.ok_exn
let px = Length.px_exn
let style = Style.create_exn

let appearance p ~empty_label =
  P.Appearance.create
    ~popup_width:(Palette.size p 360.)
    ~max_height:(Palette.size p 300.)
    ~empty_label
    ~popup_style:
      (style
         [ Background (Background.solid (Palette.surface p))
         ; Foreground (Palette.foreground p)
         ; Border_color (Palette.border p)
         ])
    ~option_style:
      (Style.with_state_exn Style.empty Selected [ Foreground (Palette.accent p) ])
    ()
  |> ok
;;

let trigger_style p =
  style
    [ Width (px (Palette.size p 320.))
    ; Height (px (Palette.size p 44.))
    ; Radius 10.
    ; Border_color (Palette.border p)
    ; Background (Background.solid (Palette.surface p))
    ; Foreground (Palette.foreground p)
    ]
;;

let controlled window palette graph =
  let state, act =
    B.state_machine0
      ~default_model:State.initial
      ~equal:State.equal
      ~apply_action:(fun _ state action -> State.apply state action)
      graph
  in
  let open B.Let_syntax in
  let picker =
    Controller.create
      window
      ~config:(B.map state ~f:State.config)
      ~on_event:
        (let%arr act = act in
         function
         | P.Event.Selection_requested request ->
           act (Request (P.Selection_request.request request))
         | Open_requested (open_, _) -> act (Request_open open_)
         | Visibility value -> act (Observe value)
         | Query_changed _ -> E.Ignore)
      graph
  in
  let%arr p = palette
  and picker = picker
  and state = state
  and act = act in
  Palette.card
    p
    ~title:"A destination you control"
    [ Palette.text
        p
        ~muted:true
        "Choose where an export belongs. The app decides when this chooser can open."
    ; V.switch
        ~checked:(State.can_open state)
        ~on_toggle:(act Toggle_permission)
        "Allow destination changes"
    ; Controller.view
        picker
        ~style:(trigger_style p)
        ~appearance:(appearance p ~empty_label:"No destinations available")
      |> ok
    ; Palette.text p ("Destination: " ^ State.selected_label state)
    ; Palette.text
        p
        ~size:12.
        ~muted:true
        (if not (State.can_open state)
         then "Changes paused · turn them on to choose a destination"
         else if State.visible state
         then "Chooser open · Escape closes it"
         else "Ready when you are")
    ; Palette.button p "Reset destination" (act Reset)
    ]
;;

let workspace_config options selected =
  P.Config.create
    ~label:"Workspace directory"
    ~options
    ~selected
    ~search:Substring
    ~clearable:true
    ~placeholder:"Search 4,096 workspaces"
    ~search_placeholder:"Try 2048 or 4096…"
    ()
  |> ok
;;

let directory window palette graph =
  let selected, request =
    B.state_machine0
      ~default_model:(P.Selection.single None)
      ~equal:P.Selection.equal
      ~apply_action:(fun _ selected request ->
        P.Config.apply_request (workspace_config State.workspaces selected) request)
      graph
  in
  let open B.Let_syntax in
  let picker =
    Controller.create
      window
      ~config:(B.map selected ~f:(workspace_config State.workspaces))
      ~on_event:
        (let%arr request = request in
         function
         | P.Event.Selection_requested value ->
           request (P.Selection_request.request value)
         | Query_changed _ | Open_requested _ | Visibility _ -> E.Ignore)
      graph
  in
  let%arr p = palette
  and picker = picker
  and selected = selected in
  let selected =
    match P.Selection.ids selected with
    | [] -> "No workspace selected"
    | id :: _ ->
      "Selected: "
      ^ (P.Collection.find State.workspaces id |> Option.value_exn |> Choice.label)
  in
  Palette.card
    p
    ~title:"Your next workspace"
    [ Palette.text
        p
        ~muted:true
        "Search 4,096 workspaces by name, or browse with the arrow keys."
    ; Controller.view
        picker
        ~style:(trigger_style p)
        ~appearance:(appearance p ~empty_label:"No workspaces match your search")
      |> ok
    ; Palette.text p selected
    ]
;;

let empty_config populated selected =
  P.Config.create
    ~label:"New workspace"
    ~options:(if populated then State.first_workspace else State.empty)
    ~selected
    ~search:Substring
    ~clearable:true
    ~placeholder:"Choose your first workspace"
    ~search_placeholder:"Find a workspace…"
    ()
  |> ok
;;

module Fresh_action = struct
  type t =
    | Create
    | Reset
    | Request of P.Request.t
end

let fresh_start window palette graph =
  let state, act =
    B.state_machine0
      ~default_model:(false, P.Selection.single None)
      ~equal:[%equal: bool * P.Selection.t]
      ~apply_action:(fun _ (populated, selected) -> function
         | Fresh_action.Create -> true, selected
         | Reset -> false, P.Selection.single None
         | Request request ->
           populated, P.Config.apply_request (empty_config populated selected) request)
      graph
  in
  let open B.Let_syntax in
  let picker =
    Controller.create
      window
      ~config:
        (B.map state ~f:(fun (populated, selected) -> empty_config populated selected))
      ~on_event:
        (let%arr act = act in
         function
         | P.Event.Selection_requested request ->
           act (Fresh_action.Request (P.Selection_request.request request))
         | Query_changed _ | Open_requested _ | Visibility _ -> E.Ignore)
      graph
  in
  let%arr p = palette
  and picker = picker
  and state = state
  and act = act in
  let populated, selected = state in
  Palette.card
    p
    ~title:"A fresh start"
    [ Palette.text
        p
        ~muted:true
        "An empty workspace still has a clear next step. Open the chooser to get started."
    ; Controller.view
        picker
        ~style:(trigger_style p)
        ~appearance:(appearance p ~empty_label:"No workspaces yet")
        ~empty:
          (V.column
             ~style:(style [ Gap (px 6.); Padding (px 12.) ])
             [ Palette.text
                 p
                 (if populated
                  then "No matching workspace"
                  else "Make room for your first idea")
             ; Palette.text
                 p
                 ~size:12.
                 ~muted:true
                 (if populated
                  then "Try another search."
                  else "Create a workspace below, then choose it here.")
             ])
        ~footer:
          (V.row
             ~style:(style [ Padding (px 8.); Gap (px 8.) ])
             [ (if populated
                then Palette.text p ~size:12. ~muted:true "Your workspace is ready"
                else Palette.button p "Create workspace" (act Fresh_action.Create))
             ])
      |> ok
    ; Palette.text
        p
        (if List.is_empty (P.Selection.ids selected)
         then "No workspace selected yet"
         else "Selected: My first workspace")
    ; Palette.button
        p
        ~disabled:(not populated)
        "Reset workspace example"
        (act Fresh_action.Reset)
    ]
;;

let component window palette graph =
  let destination = controlled window palette graph in
  let directory = directory window palette graph in
  let fresh_start = fresh_start window palette graph in
  let open B.Let_syntax in
  let%arr destination = destination
  and directory = directory
  and fresh_start = fresh_start in
  V.column ~style:(style [ Gap (px 20.) ]) [ destination; directory; fresh_start ]
;;
