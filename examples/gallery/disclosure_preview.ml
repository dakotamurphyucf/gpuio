open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View
module Editor = Gpuio_eio.Text_input

let ok = Or_error.ok_exn
let id text = Choice.Id.of_string text |> ok
let px = Length.px_exn
let style = Style.create_exn

let definitions =
  [ "identity", "Identity"; "behavior", "Behavior"; "lifetime", "Lifetime" ]
;;

let items ~locked =
  List.map definitions ~f:(fun (name, label) ->
    Choice.create
      ~id:(id name)
      ~label
      ~disabled:(locked && String.equal name "behavior")
      ()
    |> ok)
  |> Choice.Collection.create
  |> ok
;;

type action =
  | Request of Disclosure.Request.t
  | Mode of Disclosure.Mode.t
  | Toggle_disabled
  | Toggle_behavior

let apply model = function
  | Request request -> Disclosure.apply_request model request
  | Mode mode -> Disclosure.with_mode model mode
  | Toggle_disabled -> Disclosure.with_disabled model (not (Disclosure.is_disabled model))
  | Toggle_behavior ->
    let locked =
      Choice.Collection.find (Disclosure.items model) (id "behavior")
      |> Option.value_exn
      |> Choice.is_disabled
    in
    Disclosure.with_items model (items ~locked:(not locked))
;;

let component window palette graph =
  let model, inject =
    B.state_machine0
      ~default_model:
        (Disclosure.create
           ~items:(items ~locked:false)
           ~mode:(Single { allow_empty = true })
           ~expanded:[ id "identity" ]
           ()
         |> ok)
      ~apply_action:(fun _ -> apply)
      graph
  in
  let retain, toggle_retain = B.toggle ~default_model:true graph in
  let animate, toggle_animate = B.toggle ~default_model:true graph in
  let editor =
    Editor.create
      window
      ~config:
        (B.return
           (Text_input.Config.create ~mode:Multiline ~label:"Disclosure notes" () |> ok))
      ~initial_text:"Write something here, then close and reopen this section."
      graph
  in
  let open B.Let_syntax in
  let%arr p = palette
  and model = model
  and inject = inject
  and retain = retain
  and toggle_retain = toggle_retain
  and animate = animate
  and toggle_animate = toggle_animate
  and editor = editor in
  let mode = Disclosure.mode model in
  let labels =
    List.mapi definitions ~f:(fun index (name, label) ->
      let expanded = Disclosure.is_expanded model (id name) in
      ( id name
      , V.row
          ~style:(style [ Grow 1.; Min_width (px 0.); Align_items Center; Gap (px 12.) ])
          [ Palette.text p ~muted:true (sprintf "%02d" (index + 1))
          ; V.column
              ~style:(style [ Grow 1.; Min_width (px 0.); Gap (px 3.) ])
              [ Palette.text p label
              ; Palette.text
                  p
                  ~muted:true
                  ~size:12.
                  (if expanded then "Open section" else "Explore this topic")
              ]
          ; Palette.text p ~muted:true (if expanded then "−" else "+")
          ] ))
  in
  Palette.card
    p
    ~title:"Reveal the right amount"
    [ V.row
        ~style:(style [ Gap (px 8.); Wrap Wrap ])
        [ Palette.button
            p
            ~selected:(Disclosure.Mode.equal mode (Single { allow_empty = true }))
            "Single"
            (inject (Mode (Single { allow_empty = true })))
        ; Palette.button
            p
            ~selected:(Disclosure.Mode.equal mode (Single { allow_empty = false }))
            "Keep one open"
            (inject (Mode (Single { allow_empty = false })))
        ; Palette.button
            p
            ~selected:(Disclosure.Mode.equal mode Multiple)
            "Multiple"
            (inject (Mode Multiple))
        ; Palette.button p ~selected:retain "Keep drafts" toggle_retain
        ; Palette.button p ~selected:animate "Animate" toggle_animate
        ; Palette.button
            p
            ~selected:(Disclosure.is_disabled model)
            "Disabled"
            (inject Toggle_disabled)
        ; Palette.button p "Toggle Behavior availability" (inject Toggle_behavior)
        ]
    ; V.accordion_with_labels
        ~model
        ~labels
        ~hidden:(if retain then Retain else Unmount)
        ~motion:
          (if animate then Disclosure.Motion.standard else Disclosure.Motion.immediate)
        ~style:
          (style
             [ Border_color (Palette.border p)
             ; Border_width 1.
             ; Radius 12.
             ; Overflow Hidden
             ])
        ~item_style:(fun _ ->
          style [ Border_bottom_width 1.; Border_color (Palette.border p) ])
        ~trigger_style:
          (style
             [ Padding (px 12.)
             ; Foreground (Palette.foreground p)
             ; Background (Background.solid (Palette.surface p))
             ])
        ~panel_style:(style [ Padding (px 12.) ])
        ~on_request:(fun request -> inject (Request request))
        ~content:(fun target ->
          if Choice.Id.equal target (id "identity")
          then
            [ Editor.view
                ~style:(style [ Height (px (Palette.size p 92.)); Padding (px 8.) ])
                editor
            ]
          else if Choice.Id.equal target (id "behavior")
          then
            [ Palette.text
                p
                "Use the arrow keys to move between section headings. Enter or Space \
                 opens a section."
            ]
          else
            [ Palette.text
                p
                "Keep drafts preserves your notes when a section is closed. Turn it off \
                 to reset a closed field."
            ])
        ()
      |> ok
    ]
;;
