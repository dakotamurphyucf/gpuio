open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View
module C = Carousel_track
module Editor = Gpuio_eio.Text_input

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn
let full = Length.percent_exn 100.

module Card = struct
  type t =
    | Capture
    | Explore
    | Refine
    | Publish

  let all = [ Capture; Explore; Refine; Publish ]

  let name = function
    | Capture -> "Capture"
    | Explore -> "Explore"
    | Refine -> "Refine"
    | Publish -> "Publish"
  ;;

  let detail = function
    | Capture -> "Leave a thought here. It stays with this card."
    | Explore -> "Follow a promising direction. Neighbors stay within reach."
    | Refine -> "Small adjustments make a big difference."
    | Publish -> "Bring the finished idea into the world."
  ;;

  let extent = function
    | Capture -> 280.
    | Explore -> 240.
    | Refine -> 220.
    | Publish -> 260.
  ;;

  let item t = C.Item.create ~id:(C.Id.of_string (name t) |> ok) ~label:(name t) t |> ok
end

module Action = struct
  type t =
    | Request of C.Request.t
    | Toggle_axis
    | Toggle_loop
    | Toggle_auto
    | Toggle_disabled
    | Reverse
end

let apply model = function
  | Action.Request request -> C.apply_request model request |> ok
  | Toggle_axis ->
    C.with_axis
      model
      (match C.axis model with
       | Horizontal -> Vertical
       | Vertical -> Horizontal)
    |> ok
  | Toggle_loop -> C.with_looping model (not (C.is_looping model)) |> ok
  | Toggle_auto ->
    C.with_auto_advance
      model
      (if Option.is_some (C.auto_advance model)
       then None
       else Some (C.Auto_advance.create ~interval:(Time_ns.Span.of_sec 4.) () |> ok))
    |> ok
  | Toggle_disabled -> C.with_disabled model (not (C.is_disabled model)) |> ok
  | Reverse -> C.with_items model (List.rev (C.items model)) |> ok
;;

let component window palette graph =
  let model, dispatch =
    B.state_machine0
      ~default_model:(C.create (List.map Card.all ~f:Card.item) |> ok)
      ~apply_action:(fun _ model action -> apply model action)
      graph
  in
  let compact, toggle_compact = B.toggle ~default_model:false graph in
  let animated, toggle_animated = B.toggle ~default_model:true graph in
  let draft =
    Editor.create
      window
      ~initial_text:"Make something worth sharing."
      ~config:
        (B.return
           (Text_input.Config.create ~mode:Single_line ~label:"Card draft" () |> ok))
      graph
  in
  let open B.Let_syntax in
  let%arr p = palette
  and model = model
  and dispatch = dispatch
  and compact = compact
  and toggle_compact = toggle_compact
  and animated = animated
  and toggle_animated = toggle_animated
  and draft = draft in
  let horizontal =
    match C.axis model with
    | Horizontal -> true
    | Vertical -> false
  in
  let controls =
    style
      [ Padding (px 8.)
      ; Radius 8.
      ; Foreground (Palette.foreground p)
      ; Background (Background.solid (Palette.border p))
      ]
  in
  Palette.card
    p
    ~title:"Ideas in motion"
    [ Palette.text
        p
        ~muted:true
        "Drag the space around a card, scroll along the track, or use the arrow controls."
    ; V.row
        ~style:(style [ Gap (px 10.); Wrap Wrap; Align_items Center ])
        [ Palette.button
            p
            (if horizontal then "Use vertical track" else "Use horizontal track")
            (dispatch Toggle_axis)
        ; Palette.button p "Reverse cards" (dispatch Reverse)
        ; V.switch ~checked:compact ~on_toggle:toggle_compact "Compact viewport"
        ; V.switch ~checked:animated ~on_toggle:toggle_animated "Animate card movement"
        ]
    ; V.row
        ~style:(style [ Gap (px 16.); Wrap Wrap ])
        [ V.switch
            ~checked:(C.is_looping model)
            ~on_toggle:(dispatch Toggle_loop)
            "Loop cards"
        ; V.switch
            ~checked:(Option.is_some (C.auto_advance model))
            ~on_toggle:(dispatch Toggle_auto)
            "Auto-advance cards"
        ; V.switch
            ~checked:(C.is_disabled model)
            ~on_toggle:(dispatch Toggle_disabled)
            "Disable track navigation"
        ]
    ; V.carousel_track
        model
        ~key:(Key.of_string_exn "measured-idea-track")
        ~label:"Measured idea cards"
        ~on_request:(fun request -> dispatch (Request request))
        ~motion:(if animated then C.Motion.default else C.Motion.immediate)
        ~style:(style [ Width full; Gap (px 12.); Shrink 0. ])
        ~viewport_style:
          (style
             [ Width (if compact then px 360. else full)
             ; Max_width full
             ; Height (px (if horizontal then 210. else if compact then 250. else 350.))
             ; Shrink 0.
             ; Radius 12.
             ; Background (Background.solid (Palette.background p))
             ])
        ~track_style:(style [ Gap (px 16.) ])
        ~item_style:(fun item ->
          let extent = Card.extent (C.Item.data item) in
          style
            [ Width (if horizontal then px extent else full)
            ; Height (px (if horizontal then 210. else extent))
            ; Padding (px 18.)
            ; Gap (px 14.)
            ; Shrink 0.
            ; Radius 12.
            ; Background (Background.solid (Palette.border p))
            ])
        ~control_style:controls
        ~controls_style:(style [ Gap (px 8.); Justify_content Center ])
        ~content:(fun item ->
          let card = C.Item.data item in
          [ Palette.text p ~size:22. (Card.name card)
          ; Palette.text p ~muted:true (Card.detail card)
          ; (match card with
             | Capture -> Editor.view draft
             | Explore | Refine | Publish ->
               Palette.button
                 p
                 ("Select " ^ Card.name card)
                 (dispatch (Request (C.Request.select (C.Item.id item)))))
          ])
        ()
    ; Palette.text
        p
        ("Selected card: "
         ^ (C.selected model |> Option.value_map ~default:"None" ~f:C.Item.label))
    ; Palette.text
        p
        ~muted:true
        "Cards keep their state when reordered. Automatic movement pauses during \
         interaction. Loops use an immediate boundary jump when the viewport cannot fit \
         a seamless wrap."
    ]
;;
