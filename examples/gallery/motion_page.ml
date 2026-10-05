open Core
open Gpuio
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View
module A = Animation

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn
let target width radius = A.Target.create [ Width, width; Radius, radius ] |> ok
let tween ms = A.Timing.tween ~easing:A.Easing.ease_in_out (Time_ns.Span.of_ms ms) |> ok

let stage timing width radius =
  A.Stage.create ~timing ~target:(target width radius) () |> ok
;;

let sequence =
  let spring =
    A.Spring.create
      ~stiffness:150.
      ~damping:16.
      ~mass:1.
      ~max_duration:(Time_ns.Span.of_sec 2.)
      ()
    |> ok
  in
  A.Program.create
    ~initial:(target 64. 24.)
    [ stage (tween 550.) 180. 8.
    ; stage (A.Timing.spring spring) 310. 16.
    ; stage (tween 550.) 120. 24.
    ]
  |> ok
;;

let shared =
  A.Program.create
    ~initial:(target 56. 12.)
    ~repeat:Alternate
    ~clock:(A.Clock.group "gallery-activity" |> ok)
    [ stage (tween 1200.) 260. 24. ]
  |> ok
;;

let preference_label = function
  | A.Preference.System -> "System"
  | Reduce -> "Reduced"
  | Full -> "Full"
;;

let observation_label = function
  | A.Program.Observation.Stage_completed (index, Played) ->
    sprintf "Stage %d played" (index + 1)
  | Stage_completed (index, Reduced_motion) -> sprintf "Stage %d reduced" (index + 1)
  | Finished -> "Finished"
  | Cancelled reason ->
    "Cancelled: " ^ Sexp.to_string_hum ([%sexp_of: A.Program.Cancel_reason.t] reason)
;;

let component app ~motion palette graph =
  let trace = Array.exists (Sys.get_argv ()) ~f:(String.equal "--trace-motion") in
  let expanded, toggle_expanded = B.toggle ~default_model:false graph in
  let endpoint, set_endpoint = B.state "Ready to resize" graph in
  let resize_easing, set_resize_easing = B.state A.Easing.ease_in_out graph in
  let program, set_program = B.state (A.Program.with_playback sequence Paused) graph in
  let observations, set_observations = B.state "Ready to play" graph in
  let repeating, set_repeating = B.state false graph in
  let second, toggle_second = B.toggle ~default_model:false graph in
  let open B.Let_syntax in
  B.Edge.lifecycle
    ~on_deactivate:
      (let%arr program = program
       and set_program = set_program
       and set_repeating = set_repeating
       and set_observations = set_observations in
       E.Many
         [ set_program (A.Program.with_playback program Paused)
         ; set_repeating false
         ; set_observations "Ready to play"
         ])
    graph;
  let%arr p = palette
  and preference = B.Expert.Var.value motion
  and expanded = expanded
  and toggle_expanded = toggle_expanded
  and endpoint = endpoint
  and set_endpoint = set_endpoint
  and resize_easing = resize_easing
  and set_resize_easing = set_resize_easing
  and program = program
  and set_program = set_program
  and observations = observations
  and set_observations = set_observations
  and repeating = repeating
  and set_repeating = set_repeating
  and second = second
  and toggle_second = toggle_second in
  let controls children =
    V.row ~style:(style [ Gap (px 8.); Wrap Wrap; Align_items Center ]) children
  in
  let bar color =
    style
      [ Height (px 48.)
      ; Shrink 0.
      ; Background (Background.solid color)
      ; Overflow_x Hidden
      ; Overflow_y Hidden
      ]
  in
  let sample label =
    V.column ~style:(style [ Width (Length.percent_exn 100.); Height (px 48.) ]) []
    |> fun view ->
    V.with_accessibility view (Accessibility.create ~role:Group ~label () |> ok) |> ok
  in
  let track children =
    V.column
      ~style:
        (style
           [ Padding (px 12.)
           ; Gap (px 12.)
           ; Radius 16.
           ; Background (Background.solid (Palette.background p))
           ])
      children
  in
  let set_preference preference =
    E.of_thunk (fun () ->
      B.Expert.Var.set motion preference;
      Gpuio_eio.App.set_motion app preference)
  in
  let change_program f = set_program (f program) in
  V.column
    ~style:(style [ Gap (px 20.) ])
    [ Palette.card
        p
        ~title:"Motion with intention"
        [ Palette.text p ~muted:true "Motion preference applies to every gallery window."
        ; controls
            (List.map [ A.Preference.System; Reduce; Full ] ~f:(fun candidate ->
               Palette.button
                 p
                 ~selected:(A.Preference.equal preference candidate)
                 ("Use " ^ String.lowercase (preference_label candidate) ^ " motion")
                 (set_preference candidate)))
        ; Palette.text p ("Motion preference: " ^ preference_label preference)
        ; track
            [ V.animate
                ~key:(Key.of_string_exn "resize-preview")
                ~style:(bar (Palette.accent p))
                ~on_event:(fun event ->
                  set_endpoint
                    (match event.A.Event.outcome with
                     | Finished -> "Resize finished"
                     | Cancelled _ -> "Resize interrupted"))
                (A.Config.create
                   ~target:(if expanded then target 310. 12. else target 96. 24.)
                   ~duration:(Time_ns.Span.of_ms 1000.)
                   ~easing:resize_easing
                   ()
                 |> ok)
                [ sample "Resize sample" ]
            ]
        ; controls
            [ Palette.button
                p
                (if expanded then "Contract preview" else "Expand preview")
                toggle_expanded
            ; Palette.text p endpoint
            ]
        ; Palette.text
            p
            ~muted:true
            "Change direction mid-flight. The panel continues from its painted position."
        ; controls
            (List.map
               [ "CSS ease-in-out", A.Easing.ease_in_out
               ; "Cubic ease-in", A.Easing.ease_in_cubic
               ; "Cubic ease-out", A.Easing.ease_out_cubic
               ; "Cubic ease-in-out", A.Easing.ease_in_out_cubic
               ]
               ~f:(fun (label, easing) ->
                 Palette.button
                   p
                   ~selected:(A.Easing.equal resize_easing easing)
                   label
                   (set_resize_easing easing)))
        ; controls
            (List.map
               [ "Steps start", A.Easing.Step_position.Jump_start
               ; "Steps end", A.Easing.Step_position.Jump_end
               ; "Steps none", A.Easing.Step_position.Jump_none
               ; "Steps both", A.Easing.Step_position.Jump_both
               ]
               ~f:(fun (label, position) ->
                 let easing = A.Easing.steps ~count:4 ~position |> ok in
                 Palette.button
                   p
                   ~selected:(A.Easing.equal resize_easing easing)
                   label
                   (set_resize_easing easing)))
        ]
    ; Palette.card
        p
        ~title:"A sequence with a spring in its step"
        [ track
            [ V.animate_program
                ~key:(Key.of_string_exn "sequence-preview")
                ~style:(bar (Color.rgb_exn 0xa897ed))
                ~on_event:(fun event ->
                  E.Many
                    [ (if trace
                       then
                         E.of_thunk (fun () ->
                           Eio.traceln
                             "GALLERY_MOTION_EVENT: %s"
                             (Sexp.to_string_hum [%sexp (event : A.Program.Event.t)]))
                       else E.Ignore)
                    ; set_observations
                        (sprintf
                           "Run %Ld · %s"
                           (A.Run_id.to_int64 event.run_id)
                           (String.concat
                              ~sep:" · "
                              (List.map event.observations ~f:observation_label)))
                    ])
                program
                [ sample "Sequence sample" ]
            ]
        ; controls
            [ Palette.button
                p
                "Replay sequence"
                (change_program (fun p -> A.Program.restart p |> ok))
            ; Palette.button
                p
                "Pause sequence"
                (change_program (fun p -> A.Program.with_playback p Paused))
            ; Palette.button
                p
                "Resume sequence"
                (change_program (fun p -> A.Program.with_playback p Running))
            ; Palette.button
                p
                "Reverse sequence"
                (change_program (fun p -> A.Program.reverse p |> ok))
            ; Palette.button
                p
                "Cancel sequence"
                (change_program (fun p -> A.Program.with_playback p Cancelled))
            ]
        ; Palette.text p observations
        ; Palette.text
            p
            ~muted:true
            "A gentle reveal, a physical spring, then a soft landing."
        ]
    ; Palette.card
        p
        ~title:"Keep a shared rhythm"
        [ controls
            [ Palette.button
                p
                (if repeating then "Stop shared motion" else "Start shared motion")
                (set_repeating (not repeating))
            ; Palette.button
                p
                (if second then "Remove second member" else "Join a second member")
                toggle_second
            ]
        ; (if repeating
           then
             track
               (List.init
                  (if second then 2 else 1)
                  ~f:(fun index ->
                    V.animate_program
                      ~key:(Key.of_string_exn (sprintf "shared-%d" index))
                      ~style:
                        (bar
                           (if index = 0 then Palette.accent p else Color.rgb_exn 0xa897ed))
                      shared
                      [ sample (sprintf "Shared member %d" (index + 1)) ]))
           else Palette.text p ~muted:true "Shared motion is stopped.")
        ; Palette.text
            p
            ~muted:true
            "Members share an application clock. Reduced motion keeps them still. \
             Leaving this page stops the preview."
        ]
    ]
;;
