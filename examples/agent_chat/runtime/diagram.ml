open Core
module App = Gpuio_eio.App
module Scene = Gpuio_eio.Canvas
module Canvas = Gpuio.Canvas
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View
module Stage = Run_diagram.Stage

module Registration = struct
  type t =
    | Absent
    | Pending
    | Ready of Scene.t
    | Failed
end

type t =
  { model : Run_diagram.t B.Expert.Var.t
  ; registration : Registration.t B.Expert.Var.t
  ; selected : Stage.t option B.Expert.Var.t
  ; viewport : Canvas.Viewport.t B.Expert.Var.t
  ; command : Canvas.Command.t option B.Expert.Var.t
  ; status : string B.Expert.Var.t
  ; mutable sequence : int64
  ; mutable generation : int64
  ; mutable annotation : Gpuio.Color_value.Value.t
  ; mutable dark : bool
  }

let create () =
  { model = B.Expert.Var.create (Run_diagram.create ())
  ; registration = B.Expert.Var.create Registration.Absent
  ; selected = B.Expert.Var.create None
  ; viewport = B.Expert.Var.create Canvas.Viewport.default
  ; command = B.Expert.Var.create None
  ; status = B.Expert.Var.create "Preparing diagram…"
  ; sequence = 0L
  ; generation = 0L
  ; annotation = Gpuio.Color_value.Value.Empty
  ; dark = true
  }
;;

let ok = Or_error.ok_exn
let get = B.Expert.Var.get
let set = B.Expert.Var.set
let px = Gpuio.Length.px_exn
let style = Gpuio.Style.create_exn

let snapshot t =
  t.generation <- Int64.succ t.generation;
  Run_diagram.scene
    ~annotation:t.annotation
    (get t.model)
    ~palette:(Palette.of_dark t.dark)
    ~generation:t.generation
;;

let publish t =
  match get t.registration with
  | Absent | Pending | Failed -> ()
  | Ready scene ->
    (match Scene.set scene (snapshot t) with
     | Ok () -> ()
     | Error _ -> set t.status "Diagram update unavailable")
;;

let command t action =
  t.sequence <- Int64.succ t.sequence;
  set t.command (Some (Canvas.Command.create ~sequence:t.sequence action |> ok))
;;

let initialize t ~app ~window =
  let open E.Let_syntax in
  let%bind start =
    E.of_thunk (fun () ->
      match get t.registration with
      | Pending | Ready _ -> None
      | Absent | Failed ->
        set t.registration Pending;
        set t.status "Preparing diagram…";
        Some (snapshot t, t.dark, t.annotation))
  in
  match start with
  | None -> E.Ignore
  | Some (snapshot, dark, annotation) ->
    let%bind result = Scene.create app ~scope:(App.Window.scope window) snapshot in
    E.of_thunk (fun () ->
      match result with
      | Error _ ->
        set t.registration Failed;
        set t.status "Diagram unavailable. Try again."
      | Ok scene ->
        set t.registration (Ready scene);
        if
          not
            (Bool.equal dark t.dark
             && Gpuio.Color_value.Value.equal annotation t.annotation)
        then publish t;
        set t.status "Select a stage to inspect the run")
;;

let on_event t ~on_open (event : Canvas.Event.t) =
  match event.observation with
  | Activated id -> Option.value_map (Stage.of_id id) ~default:E.Ignore ~f:on_open
  | observation ->
    E.of_thunk (fun () ->
      match observation with
      | Activated _ -> ()
      | Selection_changed selected -> set t.selected (Option.bind selected ~f:Stage.of_id)
      | Moved (id, transform) ->
        Option.iter (Stage.of_id id) ~f:(fun stage ->
          match Run_diagram.move (get t.model) stage transform with
          | Ok model ->
            set t.model model;
            publish t;
            set t.status ("Moved: " ^ Stage.name stage)
          | Error _ ->
            command t Reset_positions;
            set t.status "Keep stages inside the diagram's coordinate limits")
      | Viewport_changed viewport -> set t.viewport viewport
      | Command_completed sequence ->
        if Int64.equal sequence t.sequence then set t.command None
      | Failed _ -> set t.status "Diagram interaction unavailable")
;;

let component t ~app ~window ~active ~dark ~annotation ~on_open graph =
  let visibility =
    B.map3 active dark annotation ~f:(fun active dark annotation ->
      active, dark, annotation)
  in
  B.Edge.on_change
    visibility
    ~equal:[%equal: bool * bool * Gpuio.Color_value.Value.t]
    ~callback:
      (B.return (fun (active, dark, annotation) ->
         let open E.Let_syntax in
         let%bind () =
           E.of_thunk (fun () ->
             if
               not
                 (Bool.equal dark t.dark
                  && Gpuio.Color_value.Value.equal annotation t.annotation)
             then (
               t.dark <- dark;
               t.annotation <- annotation;
               publish t);
             if active
             then
               Option.iter (get t.selected) ~f:(fun stage ->
                 command t (Select (Some (Stage.id stage)))))
         in
         if active then initialize t ~app ~window else E.Ignore))
    graph;
  let open B.Let_syntax in
  let%arr model = B.Expert.Var.value t.model
  and registration = B.Expert.Var.value t.registration
  and selected = B.Expert.Var.value t.selected
  and viewport = B.Expert.Var.value t.viewport
  and pending = B.Expert.Var.value t.command
  and status = B.Expert.Var.value t.status
  and annotation = annotation
  and dark = dark in
  let palette = Palette.of_dark dark in
  let appearance =
    if dark
    then Gpuio.Presentation.Appearance.dark
    else Gpuio.Presentation.Appearance.light
  in
  let button label action =
    V.button
      ~on_click:action
      ~style:
        (style
           [ Background (Gpuio.Background.solid palette.raised)
           ; Foreground palette.text
           ; Border_width 1.
           ; Border_color palette.line
           ; Padding (px 8.)
           ; Radius 8.
           ])
      label
  in
  let canvas =
    match registration with
    | Absent | Pending -> V.text "Preparing diagram…"
    | Failed -> button "Retry diagram" (initialize t ~app ~window)
    | Ready scene ->
      let config =
        Canvas.Config.create
          ~scene:(Scene.handle scene)
          ~label:"Run diagram"
          ~initial_viewport:viewport
          ~selection_color:palette.accent
          ?command:pending
          ()
        |> ok
      in
      V.canvas
        ~key:(Gpuio.Key.of_string_exn "run-diagram")
        config
        ~on_event:(on_event t ~on_open)
        ~style:
          (style
             [ Width (Gpuio.Length.percent_exn 100.)
             ; Height (px 300.)
             ; Shrink 0.
             ; Background (Gpuio.Background.solid palette.surface)
             ; Border_width 1.
             ; Border_color palette.line
             ; Radius 12.
             ])
  in
  V.column
    ~style:(style [ Gap (px 14.); Min_width (px 0.) ])
    [ V.row
        [ Gpuio.Presentation.badge appearance ~size:Small ~tone:Accent "SIMULATED RUN" ]
    ; V.text ~style:(style [ Font_size 23.; Font_weight 600 ]) "From context to clarity."
    ; V.text
        ~style:(style [ Foreground palette.muted; Font_size 13.; Line_height (px 20.) ])
        "Explore the stages of a sample run. Drag a stage to arrange the diagram; open \
         it to read the details."
    ; canvas
    ; V.text
        ~style:(style [ Foreground palette.muted; Font_size 12. ])
        ("Diagram annotation: "
         ^
         match annotation with
         | Empty -> "Theme accent"
         | Color color -> Gpuio.Color_value.Rgba.to_hex color)
    ; V.row
        ~style:(style [ Justify_content Space_between; Align_items Center ])
        [ V.text
            ~style:(style [ Foreground palette.muted; Font_size 12. ])
            (sprintf "Zoom %.0f%%" (Canvas.Viewport.zoom viewport *. 100.))
        ; button
            "Reset view"
            (E.of_thunk (fun () -> command t (Set_viewport Canvas.Viewport.default)))
        ]
    ; V.text
        ~style:(style [ Foreground palette.muted; Font_size 12.; Line_height (px 18.) ])
        "Scroll to pan · Ctrl + scroll to zoom · Shift + arrows to move · Enter to open"
    ; V.text ~style:(style [ Font_size 12.; Foreground palette.accent ]) status
    ; V.text ~style:(style [ Font_weight 600 ]) "Stages & selected-object details"
    ; V.text
        ~style:(style [ Font_size 12.; Foreground palette.accent ])
        (Option.value_map selected ~default:"No stage selected" ~f:(fun stage ->
           "Selected: " ^ Stage.name stage))
    ; V.column
        ~style:(style [ Gap (px 8.) ])
        (List.map Stage.all ~f:(fun stage ->
           let point = Run_diagram.position model stage in
           V.column
             ~key:(Gpuio.Key.of_string_exn (Stage.name stage))
             ~style:
               (style
                  [ Gap (px 6.)
                  ; Padding (px 12.)
                  ; Radius 8.
                  ; Background (Gpuio.Background.solid palette.surface)
                  ; Border_width 1.
                  ; Border_color
                      (if Option.equal Stage.equal selected (Some stage)
                       then palette.accent
                       else palette.line)
                  ])
             [ button
                 ("Select " ^ Stage.name stage)
                 (E.of_thunk (fun () -> command t (Select (Some (Stage.id stage)))))
             ; V.text
                 ~style:(style [ Font_size 12.; Foreground palette.muted ])
                 (sprintf
                    "%s · x %.0f · y %.0f"
                    (Stage.name stage)
                    (Gpuio.Canvas_geometry.Point.x point)
                    (Gpuio.Canvas_geometry.Point.y point))
             ; V.text
                 ~style:(style [ Font_size 12.; Foreground palette.muted ])
                 (Stage.description stage)
             ; button ("Open " ^ Stage.name stage) (on_open stage)
             ]))
    ]
;;
