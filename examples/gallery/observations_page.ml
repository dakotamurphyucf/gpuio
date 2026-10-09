open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View
module Editor = Gpuio_eio.Text_input
module I = Input_region

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn
let full = Length.percent_exn 100.

let kinds =
  I.Kind.
    [ Click
    ; Auxiliary_click
    ; Mouse_down
    ; Mouse_up
    ; Mouse_move
    ; Mouse_enter
    ; Mouse_leave
    ; Mouse_down_outside
    ; Key_down
    ; Key_up
    ; Focus
    ; Blur
    ; Scroll
    ]
;;

let kind_name = function
  | I.Kind.Click -> "Clicks"
  | Auxiliary_click -> "Auxiliary clicks"
  | Mouse_down -> "Down"
  | Mouse_up -> "Up"
  | Mouse_move -> "Moves"
  | Mouse_enter -> "Enters"
  | Mouse_leave -> "Leaves"
  | Mouse_down_outside -> "Outside presses"
  | Key_down -> "Key down"
  | Key_up -> "Key up"
  | Focus -> "Focus"
  | Blur -> "Blur"
  | Scroll -> "Wheel"
;;

module Action = struct
  type t =
    | Observe of I.Event.t
    | Reset
end

module Observation = struct
  type t =
    { counts : (I.Kind.t * int) list
    ; position : string
    ; wheel : string
    ; key : string
    ; hovered : bool
    ; focused : bool
    }

  let initial =
    { counts = List.map kinds ~f:(fun kind -> kind, 0)
    ; position = "Move into the surface to explore"
    ; wheel = "No wheel input yet"
    ; key = "No raw key input yet"
    ; hovered = false
    ; focused = false
    }
  ;;

  let apply t event =
    let kind = I.Event.kind event in
    let t =
      { t with
        counts =
          List.map t.counts ~f:(fun (key, count) ->
            ( key
            , if I.Kind.equal key kind && count < Int.max_value then count + 1 else count
            ))
      }
    in
    let position t (location : I.Location.t) =
      { t with
        position =
          sprintf "Pointer: %.0f, %.0f logical pixels" location.local.x location.local.y
      }
    in
    match event with
    | I.Event.Click mouse | Auxiliary_click mouse | Mouse_down mouse | Mouse_up mouse ->
      position t mouse.location
    | Mouse_move motion -> position t motion.location
    | Mouse_down_outside _ -> t
    | Mouse_enter -> { t with hovered = true }
    | Mouse_leave -> { t with hovered = false }
    | Focus -> { t with focused = true }
    | Blur -> { t with focused = false }
    | Key_down (key, repeated) ->
      { t with
        key = sprintf "Key: %s%s" key.key (if repeated then " · repeating" else "")
      }
    | Key_up _ -> t
    | Scroll scroll ->
      let unit_name, delta =
        match scroll.delta with
        | Pixels p -> "pixels", p
        | Lines p -> "lines", p
      in
      { t with wheel = sprintf "Wheel: %.1f, %.1f %s" delta.x delta.y unit_name }
  ;;
end

let component window palette graph =
  let disabled, toggle_disabled = B.toggle ~default_model:false graph in
  let capture, toggle_capture = B.toggle ~default_model:true graph in
  let observed, observe =
    B.state_machine0
      ~default_model:Observation.initial
      ~apply_action:(fun _ state action ->
        match action with
        | Action.Observe event -> Observation.apply state event
        | Reset -> Observation.initial)
      graph
  in
  let saved, save =
    B.state_machine0
      ~default_model:0
      ~apply_action:(fun _ count () -> if count = Int.max_value then count else count + 1)
      graph
  in
  let editor =
    Editor.create
      window
      ~initial_text:"Small ideas grow here."
      ~config:
        (B.return
           (Text_input.Config.create ~mode:Single_line ~label:"Observation draft" () |> ok))
      graph
  in
  let open B.Let_syntax in
  B.Edge.lifecycle
    ~on_deactivate:
      (let%arr observe = observe in
       observe Action.Reset)
    graph;
  let%arr p = palette
  and disabled = disabled
  and toggle_disabled = toggle_disabled
  and capture = capture
  and toggle_capture = toggle_capture
  and observed = observed
  and observe = observe
  and editor = editor
  and saved = saved
  and save = save in
  let config =
    I.Config.create
      ~label:"Input observation surface"
      ~disabled
      ~focus:Tab
      (List.map kinds ~f:(fun kind ->
         let phase =
           match kind with
           | Click
           | Auxiliary_click
           | Mouse_enter
           | Mouse_leave
           | Mouse_down_outside
           | Focus
           | Blur -> I.Phase.Bubble
           | Mouse_down | Mouse_up | Mouse_move | Key_down | Key_up | Scroll ->
             if capture then Capture else Bubble
         in
         I.Subscription.create kind ~phase () |> ok))
    |> ok
  in
  V.column
    ~style:(style [ Gap (px 20.) ])
    [ Palette.card
        p
        ~title:"Every interaction has a story"
        [ Palette.text
            p
            ~muted:true
            "Move, click or scroll across the surface. Click its open space to focus it, \
             or keep writing in the draft."
        ; V.row
            ~style:(style [ Gap (px 8.); Wrap Wrap ])
            [ Palette.button
                p
                (if disabled then "Enable observations" else "Disable observations")
                toggle_disabled
            ; Palette.button
                p
                (if capture then "Use bubble listeners" else "Use capture listeners")
                toggle_capture
            ]
        ; V.column
            ~style:(style [ Position Relative; Width full; Height (px 190.); Shrink 0. ])
            [ V.input_region
                ~key:(Key.of_string_exn "gallery-input-observations")
                ~style:
                  (style
                     [ Width full
                     ; Height (px 190.)
                     ; Shrink 0.
                     ; Padding (px 18.)
                     ; Gap (px 12.)
                     ; Radius 14.
                     ; Background (Background.solid (Palette.background p))
                     ; Border_width 1.
                     ; Border_color
                         (if (not disabled) && observed.hovered
                          then Palette.accent p
                          else Palette.border p)
                     ])
                ~config
                ~on_event:(fun event -> observe (Action.Observe event))
                [ Palette.text p ~size:19. "A place to try things"
                ; Editor.view
                    ~style:(style [ Width full; Height (px 38.); Shrink 0. ])
                    editor
                ; Palette.text
                    p
                    ~muted:true
                    "Your draft stays in place when observation settings change."
                ]
            ; V.button
                ~style:
                  (style
                     [ Position Absolute
                     ; Right (px 18.)
                     ; Bottom (px 14.)
                     ; Padding (px 10.)
                     ; Radius 8.
                     ; Font_size 14.
                     ; Background (Background.solid (Palette.surface p))
                     ; Foreground (Palette.foreground p)
                     ; Border_width 1.
                     ; Border_color (Palette.accent p)
                     ; Pointer_occlusion Pointer
                     ])
                ~on_click:(save ())
                "Save idea"
            ]
        ; Palette.text p (sprintf "Floating action: %d" saved)
        ; Palette.text
            p
            (if disabled
             then "Observations paused"
             else if observed.focused
             then "Focus: surface"
             else "Focus: elsewhere")
        ]
    ; Palette.card
        p
        ~title:"A live readout"
        [ Palette.text p observed.position
        ; Palette.text p observed.key
        ; Palette.text p observed.wheel
        ; V.row
            ~style:(style [ Gap (px 12.); Wrap Wrap ])
            (List.map observed.counts ~f:(fun (kind, count) ->
               Palette.text p ~muted:true (sprintf "%s: %d" (kind_name kind) count)))
        ; Palette.text
            p
            ~muted:true
            "Native editing keeps working. Editor shortcuts may handle a key before the \
             raw key readout sees it."
        ]
    ]
;;
