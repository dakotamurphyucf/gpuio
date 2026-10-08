open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View
module Editor = Gpuio_eio.Text_input
module History = Navigation_stack

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn

module Chapter = struct
  type t =
    | Imagine
    | Shape
    | Share

  let all = [ Imagine; Shape; Share ]

  let name = function
    | Imagine -> "Imagine"
    | Shape -> "Shape"
    | Share -> "Share"
  ;;

  let next = function
    | Imagine -> Shape
    | Shape -> Share
    | Share -> Imagine
  ;;
end

module Motion = struct
  type t =
    | Slide
    | Fade
    | Immediate

  let next = function
    | Slide -> Fade
    | Fade -> Immediate
    | Immediate -> Slide
  ;;

  let name = function
    | Slide -> "Slide"
    | Fade -> "Fade"
    | Immediate -> "Immediate"
  ;;

  let config = function
    | Slide -> History.Motion.default
    | Fade -> History.Motion.fade (Time_ns.Span.of_ms 200.) |> ok
    | Immediate -> History.Motion.immediate
  ;;
end

module Model = struct
  type t =
    { history : Chapter.t History.t
    ; root : History.Id.t
    ; serial : int
    ; hidden : Content_policy.t
    ; motion : Motion.t
    ; error : string option
    }

  module Action = struct
    type t =
      | Back
      | Forward
      | Root
      | Push
      | Replace
      | Reset
      | Toggle_retention
      | Cycle_motion
  end

  let entry serial chapter =
    History.Entry.create
      ~id:
        (History.Id.of_string (sprintf "visit-%d-%s" serial (Chapter.name chapter)) |> ok)
      ~label:(Chapter.name chapter)
      chapter
    |> ok
  ;;

  let reset serial hidden motion =
    let entries = List.map Chapter.all ~f:(entry serial) in
    let root = History.Entry.id (List.hd_exn entries) in
    { history = History.create ~current:root entries |> ok
    ; root
    ; serial = serial + 1
    ; hidden
    ; motion
    ; error = None
    }
  ;;

  let initial = reset 0 Retain Motion.Slide

  let install t result =
    match result with
    | Ok history -> { t with history; serial = t.serial + 1; error = None }
    | Error error -> { t with error = Some (Error.to_string_hum error) }
  ;;

  let apply t = function
    | Action.Back -> { t with history = History.pop t.history; error = None }
    | Forward -> { t with history = History.forward t.history; error = None }
    | Root -> { t with history = History.pop_to_root t.history; error = None }
    | Push ->
      let chapter =
        History.current t.history
        |> Option.value_exn
        |> History.Entry.data
        |> Chapter.next
      in
      install t (History.push t.history (entry t.serial chapter))
    | Replace -> install t (History.replace t.history (entry t.serial Share))
    | Reset -> reset t.serial t.hidden t.motion
    | Toggle_retention ->
      { t with
        hidden =
          (match t.hidden with
           | Retain -> Unmount
           | Unmount -> Retain)
      }
    | Cycle_motion -> { t with motion = Motion.next t.motion }
  ;;
end

let component window palette graph =
  let model, act =
    B.state_machine0
      ~default_model:Model.initial
      ~apply_action:(fun _ model action -> Model.apply model action)
      graph
  in
  let note =
    Editor.create
      window
      ~initial_text:"Return here and pick up where you left off."
      ~config:
        (B.return
           (Text_input.Config.create ~mode:Single_line ~label:"Journey note" () |> ok))
      graph
  in
  let open B.Let_syntax in
  let%arr p = palette
  and model = model
  and act = act
  and note = note in
  let current = History.current model.history |> Option.value_exn in
  let button ?(disabled = false) label action =
    V.button
      ~disabled
      ~style:
        (style
           [ Padding (px 8.)
           ; Radius 8.
           ; Foreground (Palette.foreground p)
           ; Background (Background.solid (Palette.border p))
           ])
      ~on_click:(act action)
      label
  in
  Palette.card
    p
    ~title:"Explore navigation history"
    [ V.row
        ~style:(style [ Gap (px 8.); Wrap Wrap ])
        [ button ~disabled:(not (History.can_pop model.history)) "Go back" Back
        ; button
            ~disabled:(not (History.can_forward model.history))
            "Continue journey"
            Forward
        ; button ~disabled:(not (History.can_pop model.history)) "Journey root" Root
        ; button "Visit next chapter" Push
        ; button
            ~disabled:(History.Id.equal (History.Entry.id current) model.root)
            "Replace with Share"
            Replace
        ; button "Reset journey" Reset
        ]
    ; V.row
        ~style:(style [ Gap (px 8.); Wrap Wrap ])
        [ button ("Journey motion: " ^ Motion.name model.motion) Cycle_motion
        ; V.switch
            ~checked:
              (match model.hidden with
               | Retain -> true
               | Unmount -> false)
            ~on_toggle:(act Toggle_retention)
            "Retain journey pages"
        ]
    ; V.navigation_stack
        model.history
        ~label:"Idea journey"
        ~hidden:model.hidden
        ~motion:(Motion.config model.motion)
        ~style:
          (style
             [ Height (px 170.)
             ; Shrink 0.
             ; Width (Length.percent_exn 100.)
             ; Background (Background.solid (Palette.background p))
             ; Radius 12.
             ])
        ~page_style:
          (style [ Padding (px 18.); Gap (px 12.); Height (Length.percent_exn 100.) ])
        ~content:(fun entry ->
          [ Palette.text
              p
              ~size:22.
              ("Journey: " ^ Chapter.name (History.Entry.data entry))
          ; Palette.text
              p
              ~muted:true
              ("Visit identity: " ^ History.Id.to_string (History.Entry.id entry))
          ; (if History.Id.equal (History.Entry.id entry) model.root
             then Editor.view note
             else V.column [])
          ])
        ()
    ; Palette.text p ("Current journey: " ^ History.Entry.label current)
    ; Palette.text
        p
        ~muted:true
        (sprintf
           "Journey history: %d back · %d forward"
           (List.length (History.back_entries model.history))
           (List.length (History.forward_entries model.history)))
    ; (match model.error with
       | None -> V.column []
       | Some error -> Palette.text p error)
    ]
;;
