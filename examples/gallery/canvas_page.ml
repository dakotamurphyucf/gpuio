open Core
open Gpuio
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View
module Study = Gpuio_gallery_model.Canvas_study
module Registered = Gpuio_eio.Canvas

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn
let error e = Error.create_s [%sexp (e : Registered.Error.t)]

module Source = struct
  type t =
    { canvas : Registered.t
    ; mutable model : Study.t
    ; mutable sequence : int64
    }

  let create app scope =
    E.map
      (Registered.create app ~scope (Study.scene Study.initial))
      ~f:(fun result ->
        Result.map result ~f:(fun canvas ->
          { canvas; model = Study.initial; sequence = 0L })
        |> Result.map_error ~f:error)
  ;;

  let publish t ~reset model =
    Result.map
      ((if reset then Registered.reset else Registered.set) t.canvas (Study.scene model))
      ~f:(fun () -> t.model <- model)
    |> Result.map_error ~f:error
  ;;

  let command t action =
    let next = Int64.succ t.sequence in
    Result.map (Canvas.Command.create ~sequence:next action) ~f:(fun command ->
      t.sequence <- next;
      command)
  ;;
end

let component app window palette graph =
  let trace = Array.exists (Sys.get_argv ()) ~f:(String.equal "--trace-canvas") in
  let resources =
    Preview_scope.acquire window ~name:"gallery-canvas" ~create:(Source.create app) graph
  in
  let selected, set_selected = B.state None graph in
  let command, set_command = B.state None graph in
  let zoom, set_zoom = B.state 1. graph in
  let notice, set_notice = B.state "Ready to explore" graph in
  let activated, set_activated = B.state None graph in
  let hidden, toggle_hidden = B.toggle ~default_model:false graph in
  let disabled, toggle_disabled = B.toggle ~default_model:false graph in
  let open B.Let_syntax in
  B.Edge.lifecycle
    ~on_deactivate:
      (let%arr set_selected = set_selected
       and set_command = set_command
       and set_notice = set_notice
       and set_zoom = set_zoom
       and set_activated = set_activated in
       E.Many
         [ set_selected None
         ; set_command None
         ; set_notice "Ready to explore"
         ; set_zoom 1.
         ; set_activated None
         ])
    graph;
  let%arr p = palette
  and resources = resources
  and selected = selected
  and set_selected = set_selected
  and command = command
  and set_command = set_command
  and zoom = zoom
  and set_zoom = set_zoom
  and activated = activated
  and set_activated = set_activated
  and notice = notice
  and set_notice = set_notice
  and hidden = hidden
  and toggle_hidden = toggle_hidden
  and disabled = disabled
  and toggle_disabled = toggle_disabled in
  match resources with
  | Preview_scope.Loading -> Palette.text p "Preparing canvas…"
  | Failed e -> Palette.text p ("Canvas unavailable: " ^ Error.to_string_hum e)
  | Ready source ->
    let report = function
      | Ok message -> set_notice message
      | Error e -> set_notice (Error.to_string_hum e)
    in
    let name id =
      Study.find source.model id
      |> Option.value_map ~default:"Unknown shape" ~f:Study.Item.name
    in
    let issue action =
      E.bind
        (E.of_thunk (fun () -> Source.command source action))
        ~f:(function
          | Ok command -> set_command (Some command)
          | Error e -> set_notice (Error.to_string_hum e))
    in
    let on_event (event : Canvas.Event.t) =
      let observation_effect =
        match event.observation with
        | Selection_changed id -> set_selected id
        | Activated id -> set_activated (Some (name id))
        | Moved (id, transform) ->
          E.bind
            (E.of_thunk (fun () ->
               let open Or_error.Let_syntax in
               let%bind model = Study.move source.model id transform in
               let%map () = Source.publish source ~reset:false model in
               let position =
                 Study.Item.position (Study.find model id |> Option.value_exn)
               in
               sprintf
                 "Moved: %s · x %.2f · y %.2f"
                 (name id)
                 (Canvas_geometry.Point.x position)
                 (Canvas_geometry.Point.y position)))
            ~f:report
        | Viewport_changed viewport -> set_zoom (Canvas.Viewport.zoom viewport)
        | Command_completed _ -> set_notice "Canvas request completed"
        | Failed e -> set_notice (Sexp.to_string_hum [%sexp (e : Canvas.Error.t)])
      in
      if trace
      then
        E.Many
          [ E.of_thunk (fun () ->
              Eio.traceln
                "GALLERY_CANVAS_EVENT: %s published=%b active=%b"
                (Sexp.to_string_hum [%sexp (event : Canvas.Event.t)])
                (Registered.is_published source.canvas)
                (Option.exists (Gpuio_eio.App.Window.snapshot window) ~f:(fun snapshot ->
                   snapshot.active)))
          ; observation_effect
          ]
      else observation_effect
    in
    let reset =
      E.bind
        (E.of_thunk (fun () -> Source.publish source ~reset:true Study.initial))
        ~f:(function
          | Ok () -> E.Many [ set_selected None; set_notice "Canvas reset" ]
          | Error e -> set_notice (Error.to_string_hum e))
    in
    let description =
      Option.bind selected ~f:(Study.find source.model)
      |> Option.value_map ~default:"Select a canvas shape to inspect it." ~f:(fun item ->
        let position = Study.Item.position item in
        sprintf
          "Selected: %s · x %.0f · y %.0f"
          (Study.Item.name item)
          (Canvas_geometry.Point.x position)
          (Canvas_geometry.Point.y position))
    in
    Palette.card
      p
      ~title:"A little room to make something"
      [ V.row
          ~style:(style [ Gap (px 8.); Wrap Wrap ])
          (List.map (Study.items source.model) ~f:(fun item ->
             Palette.button
               p
               ("Select " ^ Study.Item.name item)
               (issue (Select (Some (Study.Item.id item))))))
      ; V.row
          ~style:(style [ Gap (px 8.); Wrap Wrap ])
          [ Palette.button
              p
              "Zoom to 125%"
              (issue
                 (Set_viewport
                    (Canvas.Viewport.create
                       ~origin:(Canvas_geometry.Point.create ~x:0. ~y:0. |> ok)
                       ~zoom:1.25
                     |> ok)))
          ; Palette.button p "Reset canvas view" (issue Reset_viewport)
          ; Palette.button p "Reset canvas scene" reset
          ]
      ; V.row
          ~style:(style [ Gap (px 12.) ])
          [ V.switch ~checked:disabled ~on_toggle:toggle_disabled "Disable canvas input"
          ; Palette.button
              p
              (if hidden then "Show canvas" else "Hide canvas")
              toggle_hidden
          ]
      ; V.canvas
          ~key:(Key.of_string_exn "gallery-canvas")
          ~on_event
          ~style:
            (style
               [ Width (Length.percent_exn 100.)
               ; Height (px (Palette.size p 280.))
               ; Shrink 0.
               ; Radius 12.
               ; Background (Background.solid (Color.rgb_exn 0xf0f5fb))
               ; Display (if hidden then Hidden else Flex)
               ])
          (Canvas.Config.create
             ~scene:(Registered.handle source.canvas)
             ~label:"Canvas study"
             ~disabled
             ~selection_color:(Color.rgb_exn 0x245bd4)
             ?command
             ()
           |> ok)
      ; Palette.text p description
      ; Palette.text p (sprintf "Canvas zoom: %.0f%%" (zoom *. 100.))
      ; Palette.text
          p
          (Option.value_map activated ~default:"No shape activated yet" ~f:(fun name ->
             "Last activation: " ^ name))
      ; Palette.text p notice
      ; Palette.text
          p
          ~muted:true
          "Drag a shape, or focus it and use Shift + arrows to move. Pan and zoom stay \
           native. Hiding the canvas keeps your work; leaving this page starts a fresh \
           scene."
      ]
;;
