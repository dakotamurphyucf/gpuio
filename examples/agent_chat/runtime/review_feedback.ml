open Core
module B = Bonsai.Cont
module V = Gpuio_bonsai.View
module P = Gpuio.Presentation
module R = Gpuio.Rating
module D = Gpuio.Disclosure
module Input = Gpuio.Text_input

let ok = Or_error.ok_exn
let px = Gpuio.Length.px_exn
let style = Gpuio.Style.create_exn
let key = Gpuio.Key.of_string_exn
let id value = D.Id.of_string value |> ok

module Guidance = struct
  type t =
    | Sources
    | Changes
    | Verification

  let all = [ Sources; Changes; Verification ]

  let name = function
    | Sources -> "Check the sources"
    | Changes -> "Inspect the changes"
    | Verification -> "Verify the outcome"
  ;;

  let description = function
    | Sources -> "Check that the supplied context supports the response and its claims."
    | Changes -> "Read the proposed patch, including errors and edge cases."
    | Verification ->
      "Run meaningful checks before accepting changes. This run is a local simulation."
  ;;

  let items =
    List.map all ~f:(fun item ->
      Gpuio.Choice.create ~id:(id (name item)) ~label:(name item) () |> ok)
    |> Gpuio.Choice.Collection.create
    |> ok
  ;;
end

module Model = struct
  type t =
    { rating : R.Config.t
    ; note : string
    ; note_open : bool
    ; guidance : D.t
    ; preview : bool
    }

  let initial =
    { rating = R.Config.create ~label:"Run usefulness" ~value:0 () |> ok
    ; note = ""
    ; note_open = false
    ; guidance =
        D.create
          ~items:Guidance.items
          ~mode:(Single { allow_empty = true })
          ~expanded:[]
          ()
        |> ok
    ; preview = false
    }
  ;;
end

module Action = struct
  type t =
    | Rate of R.Request.t
    | Note of string
    | Toggle_note
    | Guidance of D.Request.t
    | Guidance_mode of D.Mode.t
    | Preview of bool
end

let apply (model : Model.t) = function
  | Action.Rate request ->
    { model with rating = R.Config.apply_request model.rating request }
  | Note note -> { model with note }
  | Toggle_note -> { model with note_open = not model.note_open }
  | Guidance request -> { model with guidance = D.apply_request model.guidance request }
  | Guidance_mode mode -> { model with guidance = D.with_mode model.guidance mode }
  | Preview preview -> { model with preview }
;;

let component ~active ~dark ~on_sources graph =
  let model, inject =
    B.state_machine0
      ~default_model:Model.initial
      ~apply_action:(fun _ model action -> apply model action)
      graph
  in
  let open B.Let_syntax in
  B.Edge.on_change
    active
    ~equal:Bool.equal
    ~callback:
      (B.map inject ~f:(fun inject active ->
         if active then Bonsai.Effect.Ignore else inject (Preview false)))
    graph;
  let%arr model = model
  and inject = inject
  and dark = dark in
  let p = Palette.of_dark dark in
  let appearance = if dark then P.Appearance.dark else P.Appearance.light in
  let text value =
    V.text
      ~style:(style [ Foreground p.muted; Font_size 13.; Line_height (px 20.) ])
      value
  in
  let button_style =
    style
      [ Foreground p.text
      ; Background (Gpuio.Background.solid p.raised)
      ; Border_color p.line
      ; Border_width 1.
      ; Radius 8.
      ; Padding (px 9.)
      ]
  in
  let button ?(disabled = false) label action =
    V.button ~disabled ~style:button_style ~on_click:action label
  in
  let avatar =
    V.avatar
      ~style:
        (style
           [ Background (Gpuio.Background.solid p.accent_surface)
           ; Foreground p.accent
           ; Width (px 36.)
           ; Height (px 36.)
           ; Font_size 13.
           ])
      (Gpuio.Avatar.Config.create
         ~fallback:(Gpuio.Avatar.Fallback.create "GP" |> ok)
         ~description:(Gpuio.Image.Description.label "GPUIO local assistant" |> ok)
         ())
  in
  let note =
    Gpuio.View.text_input
      ~controller:(key "run-feedback-note")
      ~config:
        (Input.Config.create
           ~mode:Multiline
           ~label:"Private review note"
           ~placeholder:"What would you improve?"
           ~min_rows:3
           ~max_rows:6
           ()
         |> ok)
      ~initial_text:model.note
      ~style:
        (style
           [ Height (px 105.)
           ; Foreground p.text
           ; Background (Gpuio.Background.solid p.surface)
           ; Padding (px 10.)
           ; Radius 8.
           ; Border_width 1.
           ; Border_color p.line
           ])
      ~on_event:(function
        | Input.Event.Changed snapshot -> inject (Note (Input.Snapshot.text snapshot))
        | Submitted submission -> inject (Note (Input.Submission.text submission)))
      ()
    |> ok
  in
  let rating = R.Config.value model.rating in
  let multiple = D.Mode.equal (D.mode model.guidance) Multiple in
  V.column
    ~style:(style [ Gap (px 18.); Min_width (px 0.) ])
    [ V.row [ P.badge appearance ~tone:Accent ~size:Small "LOCAL FEEDBACK" ]
    ; V.text ~style:(style [ Font_size 23.; Font_weight 600 ]) "A thoughtful second look."
    ; text
        "Record how useful this simulated run was. Your feedback stays in this window \
         until it closes."
    ; V.hover_card
        ~key:(key "review-contributor")
        ~config:
          (Gpuio.Hover_card.Config.create
             ~label:"About the local assistant"
             ~width:290.
             ~open_state:(Controlled model.preview)
             ()
           |> ok)
        ~on_open_change:(fun opened -> inject (Preview opened))
        ~style:
          (style
             [ Background (Gpuio.Background.solid p.surface)
             ; Foreground p.text
             ; Border_color p.line
             ; Radius 12.
             ; Padding (px 16.)
             ])
        ~anchor:(button "About this contributor" Bonsai.Effect.Ignore)
        ~content:
          (V.column
             ~style:(style [ Gap (px 12.) ])
             [ V.row
                 ~style:(style [ Gap (px 10.); Align_items Center ])
                 [ avatar
                 ; V.text ~style:(style [ Font_weight 600 ]) "GPUIO · Local assistant"
                 ]
             ; text
                 "A deterministic demo contributor. Review its sample sources and \
                  outputs before drawing conclusions."
             ; P.link
                 appearance
                 ~on_click:(fun () ->
                   Bonsai.Effect.Many [ inject (Preview false); on_sources ])
                 "Open contributor sources"
             ; button "Close contributor preview" (inject (Preview false))
             ])
        ()
    ; P.group_box
        appearance
        ~style:(style [ Radius 12.; Gap (px 12.) ])
        ~header:(P.label ~style:(style [ Font_weight 600 ]) "How useful was this run?")
        [ V.rating
            ~config:model.rating
            ~style:(style [ Foreground p.accent ])
            ~on_request:(fun request -> inject (Rate request))
            ()
        ; P.status_bar
            appearance
            ~leading:
              (P.marker
                 appearance
                 ~tone:(if rating = 0 then Neutral else Success)
                 (if rating = 0
                  then "Not rated yet"
                  else sprintf "Run usefulness: %d of 5" rating))
            ~trailing:
              (button
                 ~disabled:(rating = 0)
                 "Clear rating"
                 (inject (Rate R.Request.clear)))
            ()
        ; V.row
            ~style:(style [ Gap (px 8.); Align_items Center; Wrap Wrap ])
            [ P.shortcut_label appearance [ "←"; "→" ]; text "Adjust rating" ]
        ]
    ; V.disclosure
        ~key:(key "private-review-notes")
        ~label:"Private review notes"
        ~expanded:model.note_open
        ~hidden:Retain
        ~on_toggle:(inject Toggle_note)
        ~trigger_style:button_style
        ~panel_style:(style [ Gap (px 10.); Padding_top (px 10.) ])
        [ note
        ; text
            "Collapsing keeps this draft. Returning to Feedback restores your latest \
             note."
        ]
    ; P.separator appearance ()
    ; P.label ~style:(style [ Font_weight 600 ]) "Before you accept"
    ; V.row
        ~style:(style [ Gap (px 8.); Wrap Wrap ])
        [ button
            ~disabled:(not multiple)
            "One section"
            (inject (Guidance_mode (Single { allow_empty = true })))
        ; button ~disabled:multiple "Keep sections open" (inject (Guidance_mode Multiple))
        ]
    ; V.accordion
        ~key:(key "review-guidance")
        ~model:model.guidance
        ~hidden:Unmount
        ~trigger_style:button_style
        ~panel_style:(style [ Padding (px 12.); Gap (px 8.) ])
        ~on_request:(fun request -> inject (Guidance request))
        ~content:(fun selected ->
          match
            List.find Guidance.all ~f:(fun item ->
              D.Id.equal selected (id (Guidance.name item)))
          with
          | None -> []
          | Some item -> [ text (Guidance.description item) ])
        ()
    ]
;;
