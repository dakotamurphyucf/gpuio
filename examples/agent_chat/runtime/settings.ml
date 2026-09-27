open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View
module P = Gpuio.Presentation
module N = Gpuio.Number_input
module S = Gpuio.Slider
module O = Gpuio.Otp_input
module G = Generation_settings

module Page = struct
  type t =
    | Generation
    | Connection
    | Schedule
    | Annotation
  [@@deriving equal]
end

module Modal = struct
  type t =
    | Closed
    | Open of Page.t
    | Reset of Page.t
end

module Model = struct
  type t =
    { modal : Modal.t
    ; epoch : int
    ; generation : G.t
    ; score : Score_range.t
    ; interval_preview : int option
    ; score_preview : Score_range.t option
    ; number_error : string option
    ; steps : N.Step_controls.t
    ; code : O.Value.t
    ; completed : bool
    ; notice : string
    }
end

type t =
  { model : Model.t B.Expert.Var.t
  ; schedule : Schedule_settings.t
  ; annotation : Annotation_settings.t
  }

let ok = Or_error.ok_exn
let get (t : t) = B.Expert.Var.get t.model
let set (t : t) value = B.Expert.Var.set t.model value
let px = Gpuio.Length.px_exn
let full = Gpuio.Length.percent_exn 100.
let style = Gpuio.Style.create_exn
let key = Gpuio.Key.of_string_exn

let create () =
  let model =
    B.Expert.Var.create
      { Model.modal = Closed
      ; epoch = 0
      ; generation = G.default
      ; score = Score_range.all
      ; interval_preview = None
      ; score_preview = None
      ; number_error = None
      ; steps = Sides
      ; code = O.Value.empty
      ; completed = false
      ; notice = "Preferences are stored in this window only."
      }
  in
  { model
  ; schedule = Schedule_settings.create ()
  ; annotation = Annotation_settings.create ()
  }
;;

let annotation t = Annotation_settings.value t.annotation

let move (t : t) modal =
  let model = get t in
  set
    t
    { model with
      modal
    ; epoch = model.epoch + 1
    ; interval_preview = None
    ; score_preview = None
    ; number_error = None
    }
;;

let toggle (t : t) =
  match (get t).modal with
  | Closed -> move t (Open Generation)
  | Open _ | Reset _ -> move t Closed
;;

let current (t : t) epoch =
  let model = get t in
  match model.modal with
  | Closed -> None
  | Open _ | Reset _ -> if model.epoch = epoch then Some model else None
;;

let update (t : t) epoch f =
  E.of_thunk (fun () -> Option.iter (current t epoch) ~f:(fun model -> set t (f model)))
;;

let number_error snapshot =
  match N.Snapshot.classification snapshot with
  | Empty -> Some "Enter a chunk size before committing."
  | Incomplete -> Some "Finish the number, or press Escape to restore it."
  | Invalid _ -> Some "Use a finite number, or press Escape to restore it."
  | Out_of_range _ -> Some "Return will clamp this value to 4–128 bytes."
  | Valid _ -> None
;;

let component (t : t) ~window ~results ~on_generation ~dark graph =
  let open B.Let_syntax in
  let state = B.Expert.Var.value t.model in
  let active page =
    B.map state ~f:(fun model ->
      match model.modal with
      | Closed -> false
      | Open selected | Reset selected -> Page.equal selected page)
  in
  let is_current =
    B.map state ~f:(fun model -> fun () -> Option.is_some (current t model.epoch))
  in
  let schedule =
    match%sub active Schedule with
    | false -> B.return (V.column [])
    | true -> Schedule_settings.component t.schedule ~window ~is_current ~dark graph
  in
  let annotation =
    match%sub active Annotation with
    | false -> B.return (V.column [])
    | true -> Annotation_settings.component t.annotation ~window ~is_current ~dark graph
  in
  let%arr model = state
  and schedule = schedule
  and annotation = annotation
  and dark = dark in
  let p = Palette.of_dark dark in
  let appearance = if dark then P.Appearance.dark else P.Appearance.light in
  let epoch = model.epoch in
  let mutate = update t epoch in
  let button ?(disabled = false) text on_click =
    V.button
      ~disabled
      ~on_click
      text
      ~style:
        (style
           [ Foreground p.text
           ; Background (Gpuio.Background.solid p.raised)
           ; Border_color p.line
           ; Border_width 1.
           ; Radius 8.
           ; Padding (px 9.)
           ])
  in
  let row children = V.row ~style:(style [ Gap (px 8.); Wrap Wrap ]) children in
  let text value =
    V.text
      ~style:(style [ Foreground p.muted; Font_size 12.; Line_height (px 18.) ])
      value
  in
  let field ?error label help control =
    Gpuio.Form.field
      (Gpuio.Form.Field.create ~label ~help ?error () |> ok)
      ~label_style:(style [ Foreground p.text; Font_size 13.; Font_weight 600 ])
      ~help_style:(style [ Foreground p.muted; Font_size 12. ])
      ~control
      ()
    |> ok
  in
  let control name = key (sprintf "settings-%s-%d" name epoch) in
  let accept_generation model generation notice =
    on_generation generation;
    { model with Model.generation; notice }
  in
  let generation_page () =
    let number =
      Gpuio.View.number_input
        ~controller:(control "chunk")
        ~config:
          (N.Config.create
             ~domain:(Gpuio.Numeric.Domain.create ~min:4. ~max:128. ~step:1. |> ok)
             ~label:"Stream chunk size"
             ~step_controls:model.steps
             ()
           |> ok)
        ~initial:(N.Value.of_float (Float.of_int (G.chunk_bytes model.generation)) |> ok)
        ~style:
          (style
             [ Width full
             ; Padding_left (px 10.)
             ; Padding_right (px 10.)
             ; Height (px 40.)
             ; Foreground p.text
             ; Background (Gpuio.Background.solid p.raised)
             ; Border_width 1.
             ; Border_color p.line
             ; Radius 8.
             ])
        ~on_event:(fun event ->
          mutate (fun model ->
            match event with
            | N.Event.Committed (_, snapshot) ->
              (match N.Snapshot.committed snapshot with
               | Empty -> { model with number_error = Some "A chunk size is required." }
               | Number value ->
                 let generation =
                   G.with_chunk_bytes model.generation (Float.to_int value) |> ok
                 in
                 let model =
                   accept_generation
                     model
                     generation
                     "Chunk size saved for the next send."
                 in
                 { model with number_error = None })
            | Observed snapshot | Changed snapshot | Rejected (_, snapshot) ->
              { model with number_error = number_error snapshot }
            | Cancelled (_, _) ->
              { model with number_error = None; notice = "Numeric draft restored." }))
        ()
    in
    let slider_style = style [ Width full; Height (px 32.); Foreground p.accent ] in
    let pace =
      Gpuio.View.slider
        ~controller:(control "pace")
        ~config:
          (S.Config.create
             ~domain:(Gpuio.Numeric.Domain.create ~min:10. ~max:200. ~step:10. |> ok)
             ~label:"Stream interval"
             ()
           |> ok)
        ~initial:(S.Value.single (Float.of_int (G.interval_ms model.generation)) |> ok)
        ~style:slider_style
        ~on_event:(fun event ->
          mutate (fun model ->
            let value snapshot =
              match S.Snapshot.value snapshot with
              | Single value -> Float.to_int value
              | Range _ -> G.interval_ms model.generation
            in
            match event with
            | S.Event.Observed _ -> model
            | Drag_started snapshot | Preview snapshot ->
              { model with interval_preview = Some (value snapshot) }
            | Committed (_, snapshot) ->
              let generation =
                G.with_interval_ms model.generation (value snapshot) |> ok
              in
              let model =
                accept_generation
                  model
                  generation
                  "Stream interval saved for the next send."
              in
              { model with interval_preview = None }
            | Cancelled (_, _) ->
              { model with
                interval_preview = None
              ; notice = "Stream interval preview cancelled."
              }))
        ()
    in
    let scores =
      Gpuio.View.slider
        ~controller:(control "scores")
        ~config:
          (S.Config.create
             ~domain:(Gpuio.Numeric.Domain.create ~min:0. ~max:100. ~step:1. |> ok)
             ~label:"Result score interval"
             ~lower_label:"Minimum result score"
             ~upper_label:"Maximum result score"
             ()
           |> ok)
        ~initial:
          (S.Value.range
             ~lower:(Float.of_int (Score_range.lower model.score))
             ~upper:(Float.of_int (Score_range.upper model.score))
           |> ok)
        ~style:slider_style
        ~on_event:(fun event ->
          mutate (fun model ->
            let value snapshot =
              match S.Snapshot.value snapshot with
              | Range { lower; upper } ->
                Score_range.create ~lower:(Float.to_int lower) ~upper:(Float.to_int upper)
                |> ok
              | Single _ -> model.score
            in
            match event with
            | S.Event.Observed _ -> model
            | Drag_started snapshot | Preview snapshot ->
              { model with score_preview = Some (value snapshot) }
            | Committed (_, snapshot) ->
              { model with
                score = value snapshot
              ; score_preview = None
              ; notice = "Score interval ready to apply."
              }
            | Cancelled (_, _) ->
              { model with
                score_preview = None
              ; notice = "Score interval preview cancelled."
              }))
        ()
    in
    [ P.settings_group
        appearance
        ~title:"Stream, at your pace"
        ~description:
          "Tune the local response simulation. New sends use committed preferences; \
           active streams keep their current settings."
        [ field
            ?error:model.number_error
            "Stream chunk size"
            "4–128 bytes. Return commits, Escape restores; arrow keys step."
            number
        ; row
            (List.map
               [ N.Step_controls.Sides, "Side steppers"
               ; Stacked, "Stacked steppers"
               ; Hidden, "Keyboard only"
               ]
               ~f:(fun (steps, label) ->
                 button
                   ~disabled:(N.Step_controls.equal steps model.steps)
                   label
                   (mutate (fun model -> { model with steps }))))
        ; text (sprintf "Saved chunk size: %d bytes" (G.chunk_bytes model.generation))
        ; field
            "Stream interval"
            "10–200 ms between chunks. Drag to preview; Escape cancels an active drag."
            pace
        ; text
            (sprintf
               "Saved interval: %d ms%s"
               (G.interval_ms model.generation)
               (Option.value_map
                  model.interval_preview
                  ~default:""
                  ~f:(sprintf " · Preview: %d ms")))
        ]
    ; P.settings_group
        appearance
        ~title:"Keep the useful findings"
        ~description:
          "Filter the complete results query, including rows outside the visible page."
        [ field
            "Result score interval"
            "Two independently accessible thumbs. Apply uses the committed interval."
            scores
        ; text
            ("Ready interval: "
             ^ Score_range.describe model.score
             ^ Option.value_map model.score_preview ~default:"" ~f:(fun score ->
               " · Preview: " ^ Score_range.describe score))
        ; button
            ~disabled:(Option.is_some model.score_preview)
            "Apply score interval"
            (let open E.Let_syntax in
             let%bind model = E.of_thunk (fun () -> current t epoch) in
             match model with
             | None -> E.Ignore
             | Some model ->
               let%bind () = Results.filter_scores results model.score in
               mutate (fun model ->
                 { model with
                   notice =
                     "Score filter applied. Open Workspace → Results to inspect it."
                 }))
        ]
    ]
  in
  let connection_page () =
    let policy = O.Policy.create ~length:6 () |> ok in
    [ P.settings_group
        appearance
        ~title:"A connection rehearsal"
        ~description:
          "Enter any six digits to complete this local demo. No credentials, account, or \
           connection are created."
        [ field
            "Simulated connection code"
            "Paste 123-456 or enter six digits. Arrows and selection use the native \
             editor."
            (Gpuio.View.otp_input
               ~controller:(control "code")
               ~config:
                 (O.Config.create ~policy ~label:"Simulated connection code" () |> ok)
               ~initial:model.code
               ~style:
                 (style
                    [ Height (px 44.)
                    ; Width full
                    ; Foreground p.text
                    ; Background (Gpuio.Background.solid p.raised)
                    ; Border_color p.line
                    ; Border_width 1.
                    ; Radius 8.
                    ])
               ~on_event:(fun event ->
                 mutate (fun model ->
                   match event with
                   | O.Event.Observed _ -> model
                   | Changed snapshot ->
                     let code = O.Snapshot.value snapshot in
                     { model with
                       code
                     ; completed = model.completed && O.Value.equal code model.code
                     }
                   | Complete snapshot ->
                     { model with
                       code = O.Snapshot.value snapshot
                     ; completed = true
                     ; notice = "Simulated connection complete. No service was contacted."
                     }
                   | Rejected (_, _) ->
                     { model with notice = "Only six digits fit this simulated code." }))
               ())
        ; P.badge
            appearance
            ~tone:(if model.completed then Success else Neutral)
            (if model.completed
             then "Demo complete"
             else sprintf "%d of 6 digits" (O.Value.length model.code))
        ; button
            "Clear simulated code"
            (mutate (fun model ->
               { model with
                 code = O.Value.empty
               ; completed = false
               ; epoch = model.epoch + 1
               ; notice = "Simulated code cleared."
               }))
        ]
    ]
  in
  let close =
    E.of_thunk (fun () -> if Option.is_some (current t epoch) then move t Closed)
  in
  let content =
    match model.modal with
    | Closed -> None
    | Open page | Reset page ->
      Some
        (V.column
           ~style:(style [ Height full; Min_height (px 0.); Gap (px 16.) ])
           [ V.row
               ~style:(style [ Align_items Center; Gap (px 8.) ])
               [ P.badge appearance ~tone:Accent ~size:Small "WORKSPACE SETTINGS"
               ; V.column ~style:(style [ Grow 1. ]) []
               ; button "Close settings" close
               ]
           ; V.text
               ~style:(style [ Font_size 26.; Font_weight 600; Foreground p.text ])
               "Make it your workspace."
           ; row
               [ button
                   ~disabled:(Page.equal page Generation)
                   "Generation"
                   (E.of_thunk (fun () -> move t (Open Generation)))
               ; button
                   ~disabled:(Page.equal page Connection)
                   "Connection demo"
                   (E.of_thunk (fun () -> move t (Open Connection)))
               ; button
                   ~disabled:(Page.equal page Schedule)
                   "Dates & reviews"
                   (E.of_thunk (fun () -> move t (Open Schedule)))
               ; button
                   ~disabled:(Page.equal page Annotation)
                   "Annotation color"
                   (E.of_thunk (fun () -> move t (Open Annotation)))
               ]
           ; V.column
               ~style:
                 (style [ Grow 1.; Min_height (px 0.); Overflow_y Scroll; Gap (px 18.) ])
               ((match page with
                 | Generation -> generation_page ()
                 | Connection -> connection_page ()
                 | Schedule -> [ schedule ]
                 | Annotation -> [ annotation ])
                @ [ P.banner
                      appearance
                      ~tone:Neutral
                      ~live:Off
                      ~style:
                        (style
                           [ Border_color p.line
                           ; Radius 10.
                           ; Background (Gpuio.Background.solid p.sidebar)
                           ])
                      ~title:"Local by design"
                      [ text
                          "These preferences and simulated flows stay in memory. Closing \
                           this sheet discards uncommitted numeric drafts."
                      ]
                  ])
           ; P.separator appearance ()
           ; text model.notice
           ; row
               [ P.shortcut_label appearance [ "Esc" ]
               ; text "Close"
               ; button
                   "Reset generation preferences"
                   (mutate (fun model -> { model with modal = Reset page }))
               ]
           ; V.alert_dialog
               ~key:(key "settings-reset")
               ~config:
                 (Gpuio.Alert_dialog.Config.create
                    ~label:"Reset generation preferences?"
                    ()
                  |> ok)
               ~style:
                 (style
                    [ Background (Gpuio.Background.solid p.surface); Foreground p.text ])
               ~on_dismiss:(fun _ ->
                 mutate (fun model -> { model with modal = Open page }))
               (match model.modal with
                | Closed | Open _ -> None
                | Reset _ ->
                  Some
                    (V.column
                       ~style:(style [ Gap (px 16.) ])
                       [ V.text
                           ~style:(style [ Font_size 20.; Font_weight 600 ])
                           "Reset generation preferences?"
                       ; text
                           "Restore chunk size and pacing. Active streams and result \
                            filters keep their current values."
                       ; row
                           [ button
                               "Keep preferences"
                               (mutate (fun model -> { model with modal = Open page }))
                           ; button
                               "Reset preferences"
                               (mutate (fun model ->
                                  let model =
                                    accept_generation
                                      model
                                      G.default
                                      "Generation preferences reset."
                                  in
                                  { model with
                                    modal = Open page
                                  ; epoch = model.epoch + 1
                                  ; number_error = None
                                  ; interval_preview = None
                                  ; score_preview = None
                                  }))
                           ]
                       ]))
           ])
  in
  V.sheet
    ~key:(key "workspace-settings")
    ~config:(Gpuio.Sheet.Config.create ~label:"Workspace settings" ~extent:490. () |> ok)
    ~style:
      (style
         [ Background (Gpuio.Background.solid p.surface)
         ; Foreground p.text
         ; Padding (px 24.)
         ; Border_color p.line
         ; Border_left_width 1.
         ])
    ~on_dismiss:(fun _ -> close)
    content
;;
