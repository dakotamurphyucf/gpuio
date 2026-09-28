open Core
module W = Signal_studio_model.Workspace
module Alerts = Signal_studio_notifications.Run_alerts
module V = Gpuio_bonsai.View
module A = Gpuio.Animation
module Q = Gpuio.Container_query
module Counter = Gpuio_example_counter

module Snapshot = struct
  type t =
    { workspace : W.t
    ; canvas : Gpuio.Canvas_scene.Handle.t option
    ; chart : Gpuio.Chart_resource.t option
    ; command : Gpuio.Canvas.Command.t option
    ; inspector : bool
    ; compact : bool
    ; running : bool
    ; status : string
    ; extension_generation : int64
    ; extension_command : (int64 * int) option
    ; extension_disabled : bool
    ; extension_visible : bool
    ; documents : Documents.State.t
    ; alerts : Alerts.State.t
    ; alerts_open : bool
    }
end

module Actions = struct
  type t =
    { select : Gpuio.Canvas_scene.Item_id.t -> unit Bonsai.Effect.t
    ; canvas : Gpuio.Canvas.Event.t -> unit Bonsai.Effect.t
    ; chart : Gpuio.Chart.Event.t -> unit Bonsai.Effect.t
    ; extension : int Gpuio.Extension.Event.t -> unit Bonsai.Effect.t
    ; inspector : unit Bonsai.Effect.t
    ; run : unit Bonsai.Effect.t
    ; reset : unit Bonsai.Effect.t
    ; reset_viewport : unit Bonsai.Effect.t
    ; lock_control : unit Bonsai.Effect.t
    ; hide_control : unit Bonsai.Effect.t
    ; on_layout : Gpuio.Container_query.Selection.t -> unit Bonsai.Effect.t
    ; open_document : unit Bonsai.Effect.t
    ; save_document : unit Bonsai.Effect.t
    ; reveal_document : unit Bonsai.Effect.t
    ; quit : unit Bonsai.Effect.t
    ; toggle_alerts : unit Bonsai.Effect.t
    ; close_alerts : unit Bonsai.Effect.t
    ; enable_alerts : unit Bonsai.Effect.t
    ; notify_run : unit Bonsai.Effect.t
    ; dismiss_alert : unit Bonsai.Effect.t
    ; on_motion : name:string -> Gpuio.Animation.Program.Event.t -> unit Bonsai.Effect.t
    }
end

let ok = Or_error.ok_exn
let px = Gpuio.Length.px_exn
let full = Gpuio.Length.percent_exn 100.
let color = Gpuio.Color.rgb_exn
let style = Gpuio.Style.create_exn
let key = Gpuio.Key.of_string_exn
let bg rgb = Gpuio.Style.Property.Background (Gpuio.Background.solid (color rgb))

let text ?(size = 13.) ?(tint = 0x93a6bd) value =
  V.text ~style:(style [ Font_size size; Foreground (color tint) ]) value
;;

let button ?(disabled = false) label action =
  V.button
    ~disabled
    label
    ~on_click:action
    ~style:
      (style
         [ Padding (px 9.)
         ; Radius 9.
         ; bg 0x23354a
         ; Foreground (color 0xdce9f4)
         ; Font_size 12.
         ])
;;

let target property value = A.Target.create [ property, value ] |> ok
let stage timing value = A.Stage.create ~timing ~target:value () |> ok
let tween ms = A.Timing.tween ~easing:A.Easing.ease_out (Time_ns.Span.of_ms ms) |> ok

let indicator ~name ~label ~running =
  let program =
    A.Program.create
      ~initial:(target Opacity 1.)
      ~repeat:Alternate
      ~clock:(A.Clock.group "signal-activity" |> ok)
      [ stage (tween 700.) (target Opacity 0.35) ]
    |> ok
  in
  let program = if running then program else A.Program.with_playback program Paused in
  V.animate_program
    ~key:(key name)
    program
    [ V.with_accessibility
        (text ~size:11. ~tint:0x73dcc1 (if running then "●  UPDATING" else "●  READY"))
        (Gpuio.Accessibility.create
           ~label
           ~description:(if running then "Updating" else "Ready")
           ()
         |> ok)
      |> ok
    ]
;;

let inspector snapshot actions =
  let spring =
    A.Spring.create
      ~stiffness:220.
      ~damping:29.
      ~mass:1.
      ~epsilon:0.05
      ~max_duration:(Time_ns.Span.of_sec 1.4)
      ()
    |> ok
  in
  let program =
    A.Program.create
      [ stage
          (A.Timing.spring spring)
          (target Height (if snapshot.Snapshot.inspector then 172. else 0.))
      ]
    |> ok
  in
  let detail =
    match W.selected snapshot.workspace with
    | None -> "Select a model on the canvas."
    | Some sample ->
      sprintf
        "%s · latency %.0f ms · quality %.0f%%"
        (W.Sample.name sample)
        (W.Sample.latency sample)
        (W.Sample.quality sample)
  in
  V.animate_program
    ~key:(key "inspector-spring")
    ~on_event:(actions.Actions.on_motion ~name:"inspector")
    ~style:(style [ Width full; Overflow_y Hidden; Shrink 0. ])
    program
    [ V.panel
        ~key:(key "selected-model-panel")
        ~label:"Selected model"
        ~active:snapshot.inspector
        ~hidden:Unmount
        ~style:
          (style
             [ Height (px 172.); Padding (px 14.); Gap (px 9.); Radius 12.; bg 0x1c2b3b ])
        [ text ~size:11. ~tint:0x73dcc1 "MODEL INSPECTOR"
        ; text ~tint:0xeaf2f9 detail
        ; text ~size:11. "Shift + arrows moves the selected model."
        ; text ~size:11. "Changes update the latency chart."
        ; V.row
            ~style:(style [ Gap (px 6.) ])
            [ button
                (if snapshot.extension_disabled then "Unlock control" else "Lock control")
                actions.lock_control
            ; button
                (if snapshot.extension_visible then "Hide control" else "Show control")
                actions.hide_control
            ]
        ]
    ]
;;

let body snapshot actions ~compact =
  let width, height, zoom = if compact then 490., 330., 0.7 else 700., 470., 1. in
  let canvas =
    match snapshot.Snapshot.canvas with
    | None -> text "Preparing canvas…"
    | Some scene ->
      let viewport =
        Gpuio.Canvas.Viewport.create
          ~origin:(Gpuio.Canvas_geometry.Point.create ~x:0. ~y:0. |> ok)
          ~zoom
        |> ok
      in
      V.canvas
        ~key:(key "signal-canvas")
        ~on_event:actions.Actions.canvas
        (Gpuio.Canvas.Config.create
           ~scene
           ~label:"Model evaluation canvas"
           ~initial_viewport:viewport
           ?command:snapshot.command
           ~selection_color:(color 0xffffff)
           ()
         |> ok)
        ~style:
          (style
             [ Width (px width); Height (px height); Shrink 0.; Radius 12.; bg 0x111923 ])
  in
  let chart_width = if compact then 490. else 310. in
  let chart =
    match snapshot.chart with
    | None -> text "Preparing latency chart…"
    | Some data ->
      V.chart
        ~key:(key "signal-chart")
        ~on_event:actions.chart
        (Gpuio.Chart.Config.create
           ~data
           ~label:"Latency across 24 evaluations"
           ~style:
             (Gpuio.Chart_style.create
                ~palette:(List.map (W.samples snapshot.workspace) ~f:W.Sample.color)
                ()
              |> ok)
           ()
         |> ok)
        ~style:
          (style [ Width (px chart_width); Height (px 235.); Shrink 0.; bg 0x14202d ])
  in
  let plot =
    V.column
      ~style:
        (style [ Gap (px 10.); Padding (px 12.); Radius 16.; bg 0x14202d; Shrink 0. ])
      [ V.row
          ~style:(style [ Justify_content Space_between; Align_items Center ])
          [ text ~size:15. ~tint:0xeaf2f9 "Quality × latency"
          ; indicator
              ~name:"canvas-live"
              ~label:"Canvas activity"
              ~running:snapshot.running
          ]
      ; canvas
      ; text ~size:11. "Drag to explore · Scroll to pan · Ctrl + scroll to zoom"
      ]
  in
  let side =
    V.column
      ~style:(style [ Width (px (chart_width +. 24.)); Gap (px 12.); Shrink 0. ])
      [ V.column
          ~style:(style [ Padding (px 12.); Gap (px 10.); Radius 16.; bg 0x14202d ])
          [ V.row
              ~style:(style [ Justify_content Space_between; Align_items Center ])
              [ text ~size:15. ~tint:0xeaf2f9 "Signal history"
              ; indicator
                  ~name:"chart-live"
                  ~label:"Chart activity"
                  ~running:snapshot.running
              ]
          ; chart
          ]
      ; button
          (if snapshot.inspector then "Hide inspector" else "Show inspector")
          actions.inspector
      ; inspector snapshot actions
      ]
  in
  (if compact then V.column else V.row)
    ~style:
      (style
         [ Gap (px 14.); Align_items Start; Overflow_y Scroll; Height full; Width full ])
    [ plot
    ; V.with_accessibility
        side
        (Gpuio.Accessibility.create ~role:Group ~label:"Signal history and inspector" ()
         |> ok)
      |> ok
    ]
;;

let alerts_panel snapshot (actions : Actions.t) =
  let alerts = snapshot.Snapshot.alerts in
  V.popover
    ~key:(key "run-alerts")
    ~config:
      (Gpuio.Overlay.Config.create
         ~label:"Run alerts"
         ~width:340.
         ~dismiss_on_outside_pointer:true
         ()
       |> ok)
    ~anchor:(button "Alerts" actions.toggle_alerts)
    ~on_dismiss:(fun _ -> actions.close_alerts)
    (if not snapshot.alerts_open
     then None
     else
       Some
         (V.column
            ~style:(style [ Padding (px 16.); Gap (px 12.); bg 0x14202d; Radius 14. ])
            [ text ~size:17. ~tint:0xeaf2f9 "Keep track of your runs"
            ; text "Get a desktop alert when a run finishes."
            ; button
                ~disabled:(alerts.busy || alerts.enabled)
                "Enable alerts"
                actions.enable_alerts
            ; V.row
                ~style:(style [ Gap (px 8.) ])
                [ button
                    ~disabled:(alerts.busy || not alerts.enabled)
                    "Notify current run"
                    actions.notify_run
                ; button
                    ~disabled:(alerts.busy || not alerts.has_notification)
                    "Dismiss alert"
                    actions.dismiss_alert
                ]
            ; text ~size:12. ~tint:0x73dcc1 alerts.message
            ; button "Close alerts" actions.close_alerts
            ]))
;;

let view snapshot (actions : Actions.t) =
  let run = W.run snapshot.Snapshot.workspace in
  let counter =
    Counter.instance
      (Counter.Properties.create ~value:run ~step:1 () |> ok)
      ~generation:snapshot.extension_generation
      ?set_value:snapshot.extension_command
      ~disabled:(snapshot.extension_disabled || run = 100)
      ()
    |> ok
  in
  let compact = Q.Branch_id.of_string "compact" |> ok in
  let wide = Q.Branch_id.of_string "wide" |> ok in
  let query =
    Q.Config.create
      ~default:compact
      [ Q.Rule.create
          ~branch:wide
          ~condition:
            (Q.Predicate.create ~width:(Q.Range.create ~minimum:1080. () |> ok) ())
      ]
    |> ok
  in
  let sequence =
    A.Program.create
      ~initial:(target Opacity 0.45)
      [ stage (tween 100.) (target Opacity 0.75); stage (tween 180.) (target Opacity 1.) ]
    |> ok
  in
  V.column
    ~style:
      (style
         [ Width full
         ; Height full
         ; Padding (px 22.)
         ; Gap (px 16.)
         ; Overflow_y Scroll
         ; bg 0x0b131e
         ])
    [ V.row
        ~style:(style [ Justify_content Space_between; Align_items Center; Gap (px 12.) ])
        [ V.column
            ~style:(style [ Gap (px 6.) ])
            [ text ~size:11. ~tint:0x73dcc1 "GPUIO  /  SIGNAL STUDIO"
            ; text
                ~size:(if snapshot.compact then 25. else 31.)
                ~tint:0xf1f6fb
                (if snapshot.compact
                 then "Follow the signal."
                 else "A clearer view of your models.")
            ; text
                (if snapshot.compact
                 then "Your model evaluation workbench."
                 else "Move a point. Compare a run. Follow the signal.")
            ]
        ; V.column
            ~style:(style [ Width (px 215.); Gap (px 6.); Shrink 0. ])
            [ text ~size:11. "RUN CONTROL"
            ; V.extension
                ~key:(key "run-control")
                ~on_event:actions.extension
                counter
                ~style:
                  (style
                     [ Display (if snapshot.extension_visible then Flex else Hidden)
                     ; Radius 12.
                     ])
            ]
        ]
    ; V.row
        ~style:(style [ Gap (px 8.); Align_items Center ])
        (List.map (W.samples snapshot.workspace) ~f:(fun sample ->
           button (W.Sample.name sample) (actions.select (W.Sample.id sample)))
         @ [ button
               (if snapshot.running then "Pause stream" else "Stream runs")
               actions.run
           ; button "Reset view" actions.reset_viewport
           ; button "Reset workspace" actions.reset
           ])
    ; V.row
        ~style:(style [ Gap (px 8.); Align_items Center ])
        [ button "Open workspace" actions.open_document
        ; button "Save workspace" actions.save_document
        ; button "Reveal file" actions.reveal_document
        ; button "Quit Studio" actions.quit
        ; alerts_panel snapshot actions
        ; text
            ~size:11.
            (if snapshot.documents.busy
             then "Working…"
             else if snapshot.documents.edited
             then "Unsaved changes"
             else if Option.is_some snapshot.documents.path
             then "Saved workspace"
             else "Untitled workspace")
        ]
    ; V.container_query
        ~key:(key "workspace-layout")
        ~on_select:actions.on_layout
        query
        ~style:
          (style
             [ Width full
             ; Height (px (if snapshot.compact then 620. else 550.))
             ; Min_width (px 0.)
             ; Shrink 0.
             ])
        [ compact, body snapshot actions ~compact:true
        ; wide, body snapshot actions ~compact:false
        ]
      |> ok
    ; V.animate_program
        ~key:(Gpuio.Key.of_int run)
        ~on_event:(actions.on_motion ~name:"run")
        sequence
        [ text ~size:12. ~tint:0x73dcc1 (sprintf "RUN %02d  /  %s" run snapshot.status) ]
    ; text ~size:11. "Simulated data · Built with OCaml, Bonsai and GPUI"
    ]
;;
