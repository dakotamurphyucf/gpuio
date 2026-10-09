open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View
module Editor = Gpuio_eio.Text_input

let ok = Or_error.ok_exn
let id = Choice.Id.of_string

let definitions =
  [ "plan", "Plan"; "draft", "Draft"; "review", "Review"; "launch", "Launch" ]
;;

let steps ~review_disabled =
  List.map definitions ~f:(fun (name, label) ->
    Choice.create
      ~id:(id name |> ok)
      ~label
      ~disabled:(review_disabled && String.equal name "review")
      ()
    |> ok)
  |> Choice.Collection.create
  |> ok
;;

let review_disabled model =
  Choice.Collection.find (Stepper.steps model) (id "review" |> ok)
  |> Option.value_exn
  |> Choice.is_disabled
;;

module Action = struct
  type t =
    | Navigate of Stepper.Request.t
    | Toggle_disabled
    | Toggle_review
end

let apply model = function
  | Action.Navigate request -> Stepper.apply_request model request
  | Toggle_disabled -> Stepper.with_disabled model (not (Stepper.is_disabled model))
  | Toggle_review ->
    Stepper.with_steps model (steps ~review_disabled:(not (review_disabled model))) |> ok
;;

let component window palette graph =
  let model, inject =
    B.state_machine0
      ~default_model:
        (Stepper.create
           ~steps:(steps ~review_disabled:false)
           ~current:(Some (id "plan" |> ok))
           ()
         |> ok)
      ~apply_action:(fun _ model action -> apply model action)
      graph
  in
  let vertical, toggle_vertical = B.toggle ~default_model:false graph in
  let centered, toggle_centered = B.toggle ~default_model:true graph in
  let symbols, toggle_symbols = B.toggle ~default_model:false graph in
  let notes =
    Editor.create
      window
      ~config:
        (B.return
           (Text_input.Config.create
              ~mode:Multiline
              ~label:"Workflow notes"
              ~min_rows:2
              ~max_rows:4
              ()
            |> ok))
      ~initial_text:"A small idea, ready to take shape."
      graph
  in
  let open B.Let_syntax in
  let%arr p = palette
  and model = model
  and inject = inject
  and vertical = vertical
  and toggle_vertical = toggle_vertical
  and centered = centered
  and toggle_centered = toggle_centered
  and symbols = symbols
  and toggle_symbols = toggle_symbols
  and notes = notes in
  let style = Style.create_exn in
  let px = Length.px_exn in
  let status_text = function
    | Stepper.Status.Completed -> "Behind your current stage"
    | Current -> "Where you are now"
    | Upcoming -> "Still ahead"
  in
  let appearance =
    Stepper.Appearance.create
      ~indicator_size:(Palette.size p 32.)
      ~item_style:
        (style [ Foreground (Palette.foreground p); Font_size (Palette.size p 14.) ])
      ~indicator_style:
        (style
           [ Background (Background.solid (Palette.surface p))
           ; Foreground (Palette.foreground p)
           ])
      ~current_style:
        (style [ Border_color (Palette.accent p); Foreground (Palette.accent p) ])
      ~completed_style:(style [ Border_color (Palette.accent p) ])
      ~upcoming_style:(style [ Border_color (Palette.border p) ])
      ()
    |> ok
  in
  let indicator =
    if symbols
    then
      Some
        (fun _ status ->
          V.text
            (match status with
             | Stepper.Status.Completed -> "✓"
             | Current -> "●"
             | Upcoming -> "·"))
    else None
  in
  let current =
    Option.value_map
      (Stepper.current model)
      ~default:"No stage selected"
      ~f:(fun selected ->
        Choice.Collection.find (Stepper.steps model) selected
        |> Option.value_exn
        |> Choice.label)
  in
  let navigate request = inject (Action.Navigate request) in
  Palette.card
    p
    ~title:"From an idea to a launch"
    [ Stepper.view
        model
        ~label:"Publishing workflow"
        ~axis:(if vertical then Vertical else Horizontal)
        ~centered
        ~appearance
        ?indicator
        ~content:(fun item status ->
          V.column
            ~style:(style [ Gap (px 3.) ])
            [ Palette.text p (Choice.label item)
            ; Palette.text p ~muted:true ~size:11. (status_text status)
            ])
        ~on_request:navigate
        ()
      |> ok
    ; V.row
        ~style:(style [ Gap (px 8.); Wrap Wrap ])
        [ V.button
            ~disabled:(Stepper.is_disabled model)
            ~on_click:(navigate Previous)
            "Previous stage"
        ; V.button
            ~disabled:(Stepper.is_disabled model)
            ~on_click:(navigate Next)
            "Next stage"
        ; Palette.text p ("Current stage: " ^ current)
        ]
    ; Editor.view notes
    ; Palette.text
        p
        ~muted:true
        "Move between stages — your notes stay in place. Stage selection does not \
         validate or submit them."
    ; V.row
        ~style:(style [ Gap (px 12.); Wrap Wrap ])
        [ V.switch ~checked:vertical ~on_toggle:toggle_vertical "Vertical workflow"
        ; V.switch ~checked:centered ~on_toggle:toggle_centered "Centered step labels"
        ; V.switch ~checked:symbols ~on_toggle:toggle_symbols "Status symbols"
        ]
    ; V.switch
        ~checked:(review_disabled model)
        ~on_toggle:(inject Toggle_review)
        "Skip review stage"
    ; V.switch
        ~checked:(Stepper.is_disabled model)
        ~on_toggle:(inject Toggle_disabled)
        "Disable stage navigation"
    ]
;;
