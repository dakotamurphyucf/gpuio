open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View
module P = Gpuio.Presentation
module Counter = Gpuio_example_counter

let ok = Or_error.ok_exn
let px = Gpuio.Length.px_exn
let style = Gpuio.Style.create_exn

let component ~dark graph =
  let value, set_value = B.state 0 graph in
  let step, set_step = B.state 1 graph in
  let command, set_command = B.state None graph in
  let serial, set_serial = B.state 0L graph in
  let status, set_status = B.state "Ready to review" graph in
  let open B.Let_syntax in
  let%arr dark = dark
  and value = value
  and set_value = set_value
  and step = step
  and set_step = set_step
  and command = command
  and set_command = set_command
  and serial = serial
  and set_serial = set_serial
  and status = status
  and set_status = set_status in
  let palette = Palette.of_dark dark in
  let appearance = if dark then P.Appearance.dark else P.Appearance.light in
  let secondary_button =
    Gpuio.Style.with_state_exn
      (style
         [ Foreground palette.text
         ; Background (Gpuio.Background.solid palette.raised)
         ; Border_color palette.line
         ; Border_width 1.
         ; Padding (px 9.)
         ; Radius 8.
         ])
      Focused
      [ Border_color palette.accent ]
  in
  let properties = Counter.Properties.create ~value ~step () |> ok in
  let instance = Counter.instance properties ~generation:1L ?set_value:command () |> ok in
  let on_event = function
    | Gpuio.Extension.Event.Data count ->
      E.Many [ set_value count; set_status "Review updated" ]
    | Command_completed completed ->
      if Option.exists command ~f:(fun (pending, _) -> Int64.equal pending completed)
      then E.Many [ set_command None; set_status "Review reset" ]
      else E.Ignore
    | Mounted -> E.Ignore
    | Failed _ -> set_status "Review component unavailable"
  in
  let choice amount =
    V.button
      ~style:
        (Gpuio.Style.merge
           [ secondary_button
           ; style
               [ Background
                   (Gpuio.Background.solid
                      (if step = amount then palette.accent_surface else palette.raised))
               ; Foreground (if step = amount then palette.accent else palette.muted)
               ; Border_color (if step = amount then palette.accent else palette.line)
               ]
           ])
      ~on_click:(set_step amount)
      (sprintf "+%d per checkpoint" amount)
  in
  let reset =
    let next = Int64.succ serial in
    E.Many
      [ set_serial next
      ; set_command (Some (next, 0))
      ; set_value 0
      ; set_status "Resetting review…"
      ]
  in
  V.column
    ~style:(style [ Gap (px 20.); Min_width (px 0.) ])
    [ V.column
        ~style:(style [ Gap (px 8.) ])
        [ V.row [ P.badge appearance ~size:Small ~tone:Accent "REVIEW" ]
        ; V.text
            ~style:(style [ Font_size 23.; Font_weight 600 ])
            "Make every pass count."
        ; V.text
            ~style:
              (style [ Foreground palette.muted; Font_size 13.; Line_height (px 20.) ])
            "Keep a checkpoint as you inspect the agent's work. Your progress stays in \
             this window when you return to the conversation."
        ]
    ; P.group_box
        appearance
        ~style:(style [ Padding (px 16.); Gap (px 14.); Radius 12. ])
        ~header:(V.text ~style:(style [ Font_weight 600 ]) "Review checkpoints")
        [ V.extension ~key:(Gpuio.Key.of_string_exn "review-counter") ~on_event instance
        ; V.row ~style:(style [ Gap (px 8.) ]) [ choice 1; choice 5 ]
        ; P.description_list
            appearance
            ~stacked:true
            [ P.Description.create
                ~key:(Gpuio.Key.of_string_exn "review-progress")
                ~term:"Recorded progress"
                ~definition:(V.text (sprintf "%d of 100 checkpoints" value))
            ; P.Description.create
                ~key:(Gpuio.Key.of_string_exn "review-state")
                ~term:"Status"
                ~definition:(V.text status)
            ]
        ; V.row
            ~style:(style [ Justify_content Flex_end ])
            [ V.button ~style:secondary_button ~on_click:reset "Reset review" ]
        ]
    ; P.marker appearance "Saved locally in this window"
    ]
;;
