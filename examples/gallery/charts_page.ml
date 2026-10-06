open Core
open Gpuio
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View
module Registered = Gpuio_eio.Chart
module Samples = Gpuio_chart_samples
module Family = Samples.Family

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn
let error e = Error.create_s [%sexp (e : Registered.Error.t)]

module Mode = struct
  type t =
    | Family of Family.t
    | Mixed
    | Categorical
    | Stacked_bars
    | Stacked_areas
    | Ordinal_colors
    | Flow_styling
    | Flow_labels
  [@@deriving equal]

  let all =
    List.map Family.all ~f:(fun f -> Family f)
    @ [ Mixed
      ; Categorical
      ; Stacked_bars
      ; Stacked_areas
      ; Ordinal_colors
      ; Flow_styling
      ; Flow_labels
      ]
  ;;

  let label = function
    | Family family -> Family.label family
    | Mixed -> "Mixed layers"
    | Categorical -> "Categorical"
    | Stacked_bars -> "Stacked bars"
    | Stacked_areas -> "Stacked areas"
    | Ordinal_colors -> "Ordinal colors"
    | Flow_styling -> "Flow styling"
    | Flow_labels -> "Flow labels"
  ;;

  let data t phase =
    match t with
    | Family family -> Samples.data_exn family phase
    | Mixed -> Samples.preset_data_exn Mixed Line phase
    | Categorical -> Samples.Categorical.data_exn phase
    | Stacked_bars -> Samples.Stacked.data_exn ~area:false phase
    | Stacked_areas -> Samples.Stacked.data_exn ~area:true phase
    | Ordinal_colors -> Samples.Ordinal_colors.data_exn phase
    | Flow_styling -> Samples.Sankey_presentation.data_exn phase
    | Flow_labels -> Samples.Sankey_presentation.placement_data_exn phase
  ;;
end

module Category_layout = struct
  type t =
    | Auto
    | Point
    | Band
  [@@deriving equal]

  let all = [ Auto; Point; Band ]

  let label = function
    | Auto -> "Auto categories"
    | Point -> "Point categories"
    | Band -> "Band categories"
  ;;

  let options = function
    | Auto -> Chart_options.Category_layout.auto
    | Point -> Chart_options.Category_layout.point ~padding:0.75 () |> ok
    | Band ->
      Chart_options.Category_layout.band ~inner_padding:0.35 ~outer_padding:0.25 () |> ok
  ;;
end

module Radar_scale = struct
  type t = Chart_options.Radar.Scale.t =
    | Per_axis
    | Data_max
    | Maximum of float
  [@@deriving equal]

  let all = [ Per_axis; Data_max; Maximum 50. ]

  let label = function
    | Per_axis -> "Per-axis maxima"
    | Data_max -> "Shared data maximum"
    | Maximum _ -> "Shared maximum 50"
  ;;
end

let radar_labels p ~enabled ~activations ~on_activate ~editor =
  if not enabled
  then Chart_radar_labels.empty
  else (
    let entry id content =
      Chart_radar_labels.Entry.create
        ~axis:(Chart_data.Datum_id.of_int64 id |> ok)
        content
    in
    Chart_radar_labels.create
      [ entry
          1L
          (V.button
             ~accessible_name:"Inspect radar quality"
             ~on_click:on_activate
             ~style:
               (style
                  [ Padding (px 6.)
                  ; Radius 7.
                  ; Font_size (Palette.size p 11.)
                  ; Background (Background.solid (Palette.surface p))
                  ; Foreground (Palette.accent p)
                  ; Border_width 1.
                  ; Border_color (Palette.border p)
                  ])
             (sprintf "Quality · %d" activations))
      ; entry
          3L
          (V.column
             [ Palette.text p ~size:11. "Cost"
             ; Palette.text p ~size:10. ~muted:true "per request"
             ])
      ; entry
          4L
          (Gpuio_eio.Text_input.view
             editor
             ~style:
               (style
                  [ Width (px (Palette.size p 128.))
                  ; Height (px (Palette.size p 30.))
                  ; Font_size (Palette.size p 11.)
                  ; Padding (px 4.)
                  ; Radius 6.
                  ; Background (Background.solid (Palette.surface p))
                  ; Foreground (Palette.foreground p)
                  ; Border_width 1.
                  ; Border_color (Palette.border p)
                  ]))
      ]
    |> ok)
;;

module Source = struct
  type t =
    { chart : Registered.t
    ; mutable mode : Mode.t
    ; mutable phase : int
    }

  let create app scope mode =
    E.map
      (Registered.create app ~scope (Mode.data mode 0.))
      ~f:(fun result ->
        Result.map result ~f:(fun chart -> { chart; mode; phase = 0 })
        |> Result.map_error ~f:error)
  ;;

  let choose t mode =
    Result.map
      (Registered.reset t.chart (Mode.data mode 0.))
      ~f:(fun () ->
        t.mode <- mode;
        t.phase <- 0)
    |> Result.map_error ~f:error
  ;;

  let update t =
    let next = (t.phase + 1) % 13 in
    Result.map
      (Registered.set t.chart (Mode.data t.mode (Float.of_int next)))
      ~f:(fun () -> t.phase <- next)
    |> Result.map_error ~f:error
  ;;
end

let component app window palette graph =
  (* These variables belong to one constructed page branch in one window.
     Retained choices survive departure; the scope reacquires native data. *)
  let mode = B.Expert.Var.create (Mode.Family Line) in
  let selected = B.Expert.Var.create None in
  let resources =
    Preview_scope.acquire
      window
      ~name:"gallery-charts"
      ~create:(fun scope ->
        B.Expert.Var.set selected None;
        Source.create app scope (B.Expert.Var.get mode))
      graph
  in
  let inspection, set_inspection = B.state Samples.Inspection.Default graph in
  let flow_style, set_flow_style = B.state Samples.Sankey_presentation.Default graph in
  let narrow_flow, toggle_narrow_flow = B.toggle ~default_model:false graph in
  let outside, toggle_outside = B.toggle ~default_model:true graph in
  let long_labels, toggle_long_labels = B.toggle ~default_model:false graph in
  let bold_labels, toggle_bold_labels = B.toggle ~default_model:false graph in
  let show_labels, toggle_show_labels = B.toggle ~default_model:true graph in
  let horizontal, toggle_horizontal = B.toggle ~default_model:false graph in
  let reversed, toggle_reversed = B.toggle ~default_model:false graph in
  let category_layout, set_category_layout = B.state Category_layout.Auto graph in
  let unknown_color, toggle_unknown_color = B.toggle ~default_model:true graph in
  let stacked, toggle_stacked = B.toggle ~default_model:true graph in
  let fixed_pie, toggle_fixed_pie = B.toggle ~default_model:false graph in
  let variable_pie, toggle_variable_pie = B.toggle ~default_model:false graph in
  let radar_scale, set_radar_scale = B.state Radar_scale.Per_axis graph in
  let fixed_radius, toggle_fixed_radius = B.toggle ~default_model:false graph in
  let spaced_radar, toggle_spaced_radar = B.toggle ~default_model:false graph in
  let rich_radar, toggle_rich_radar = B.toggle ~default_model:false graph in
  let show_radar_labels, toggle_radar_labels = B.toggle ~default_model:true graph in
  let label_activations, set_label_activations = B.state 0 graph in
  let label_editor =
    Gpuio_eio.Text_input.create
      window
      ~config:
        (B.return
           (Text_input.Config.create
              ~mode:Single_line
              ~label:"Radar axis note"
              ~placeholder:"Context note"
              ()
            |> ok))
      graph
  in
  let disabled, toggle_disabled = B.toggle ~default_model:false graph in
  let notice, set_notice = B.state "Preparing chart…" graph in
  let open B.Let_syntax in
  B.Edge.lifecycle
    ~on_deactivate:
      (let%arr set_notice = set_notice in
       set_notice "Preparing chart…")
    graph;
  let%arr p = palette
  and resources = resources
  and current_mode = B.Expert.Var.value mode
  and selection = B.Expert.Var.value selected
  and flow_style = flow_style
  and set_flow_style = set_flow_style
  and inspection = inspection
  and set_inspection = set_inspection
  and narrow_flow = narrow_flow
  and toggle_narrow_flow = toggle_narrow_flow
  and outside = outside
  and toggle_outside = toggle_outside
  and long_labels = long_labels
  and toggle_long_labels = toggle_long_labels
  and bold_labels = bold_labels
  and toggle_bold_labels = toggle_bold_labels
  and show_labels = show_labels
  and toggle_show_labels = toggle_show_labels
  and horizontal = horizontal
  and toggle_horizontal = toggle_horizontal
  and reversed = reversed
  and toggle_reversed = toggle_reversed
  and category_layout = category_layout
  and set_category_layout = set_category_layout
  and unknown_color = unknown_color
  and toggle_unknown_color = toggle_unknown_color
  and stacked = stacked
  and toggle_stacked = toggle_stacked
  and fixed_pie = fixed_pie
  and toggle_fixed_pie = toggle_fixed_pie
  and variable_pie = variable_pie
  and toggle_variable_pie = toggle_variable_pie
  and radar_scale = radar_scale
  and set_radar_scale = set_radar_scale
  and fixed_radius = fixed_radius
  and toggle_fixed_radius = toggle_fixed_radius
  and spaced_radar = spaced_radar
  and toggle_spaced_radar = toggle_spaced_radar
  and rich_radar = rich_radar
  and toggle_rich_radar = toggle_rich_radar
  and show_radar_labels = show_radar_labels
  and toggle_radar_labels = toggle_radar_labels
  and label_editor = label_editor
  and label_activations = label_activations
  and set_label_activations = set_label_activations
  and disabled = disabled
  and toggle_disabled = toggle_disabled
  and notice = notice
  and set_notice = set_notice in
  match resources with
  | Preview_scope.Loading -> Palette.text p "Preparing chart…"
  | Failed e -> Palette.text p ("Chart unavailable: " ^ Error.to_string_hum e)
  | Ready source ->
    let placement_mode = Mode.equal current_mode Flow_labels in
    let flow_mode = placement_mode || Mode.equal current_mode Flow_styling in
    let orientation, direction =
      match horizontal, reversed with
      | false, false -> Chart_options.Orientation.Vertical, "Vertical"
      | true, false -> Horizontal, "Horizontal"
      | false, true -> Vertical_reversed, "Vertical reversed"
      | true, true -> Horizontal_reversed, "Horizontal reversed"
    in
    let choose next =
      E.bind
        (E.of_thunk (fun () ->
           Result.map (Source.choose source next) ~f:(fun () ->
             B.Expert.Var.set selected None;
             B.Expert.Var.set mode next)))
        ~f:(function
          | Ok () -> set_notice "Preparing chart…"
          | Error e -> set_notice (Error.to_string_hum e))
    in
    let update =
      E.bind
        (E.of_thunk (fun () -> Source.update source))
        ~f:(function
          | Ok () ->
            if Registered.is_published source.chart
            then E.Ignore
            else set_notice "Publishing updated samples…"
          | Error e -> set_notice (Error.to_string_hum e))
    in
    let on_event (event : Chart.Event.t) =
      match event.observation with
      | Ready metrics ->
        set_notice
          (sprintf
             "Ready: %s · %d source values · %s · %s"
             (Mode.label source.mode)
             metrics.source_values
             (if Mode.equal current_mode Categorical
              then direction ^ " · " ^ Category_layout.label category_layout
              else if
                Mode.equal current_mode Stacked_bars
                || Mode.equal current_mode Stacked_areas
              then direction ^ if stacked then " · Stacked" else " · Grouped"
              else if Mode.equal current_mode Ordinal_colors
              then
                direction
                ^ if unknown_color then " · Explicit unknown" else " · Palette fallback"
              else if flow_mode
              then
                direction
                ^ " · "
                ^ Samples.Sankey_presentation.label flow_style
                ^
                if placement_mode
                then if outside then " · Outside" else " · Inside"
                else ""
              else if Mode.equal current_mode (Family Pie)
              then
                sprintf
                  "%s · %s"
                  (if fixed_pie then "80 px" else "Fit")
                  (if variable_pie then "Per-slice radii" else "Uniform radii")
              else if Mode.equal current_mode (Family Radar)
              then
                sprintf
                  "%s · %s · gap %d"
                  (Radar_scale.label radar_scale)
                  (if fixed_radius then "80 px" else "Fit")
                  (if spaced_radar then 24 else 0)
              else direction)
             (Samples.Inspection.label inspection))
      | Failed e -> set_notice (Sexp.to_string_hum [%sexp (e : Chart.Error.t)])
      | Selection_changed target ->
        E.of_thunk (fun () -> B.Expert.Var.set selected target)
    in
    let description =
      if Registered.is_published source.chart
      then
        Option.bind (Registered.data source.chart) ~f:(fun data ->
          Option.bind selection ~f:(Samples.describe_selection data))
      else None
    in
    let stack_mode =
      match current_mode with
      | Mode.Stacked_bars | Stacked_areas -> true
      | Family _ | Mixed | Categorical | Ordinal_colors | Flow_styling | Flow_labels ->
        false
    in
    let options =
      Chart_options.create
        ~sankey:
          (if flow_mode
           then
             Samples.Sankey_presentation.options
               ~label_placement:(if placement_mode && outside then Outside else Inside)
               ~labels:((not placement_mode) || show_labels)
               flow_style
           else Chart_options.Sankey.default)
        ~cartesian:
          (Chart_options.Cartesian.create
             ~orientation
             ~stacking:(if stack_mode && stacked then Stacked else Grouped)
             ~category_layout:(Category_layout.options category_layout)
             ()
           |> ok)
        ~pie:
          (Chart_options.Pie.create
             ~inner_radius:0.5
             ~radius:(if fixed_pie then Pixels 80. else Fit)
             ~slice_radii:
               (if variable_pie
                then
                  List.map
                    [ 1L, 20., 90.; 2L, 35., 65.; 3L, 15., 75.; 4L, 0., 50. ]
                    ~f:(fun (id, inner, outer) ->
                      Chart_options.Pie.Slice_radii.create
                        ~slice:(Chart_data.Datum_id.of_int64 id |> ok)
                        ~inner
                        ~outer
                        ()
                      |> ok)
                else [])
             ()
           |> ok)
        ~radar:
          (Chart_options.Radar.create
             ~labels:show_radar_labels
             ~scale:radar_scale
             ~radius:(if fixed_radius then Pixels 80. else Fit)
             ~label_gap:(if spaced_radar then 24. else 0.)
             ()
           |> ok)
        ()
    in
    let chart_style =
      Chart_style.create
        ?ordinal:
          (Option.some_if
             (Mode.equal current_mode Ordinal_colors)
             (Samples.Ordinal_colors.mapping ~unknown:unknown_color))
        ~inspection:(Samples.Inspection.config inspection)
        ~node_labels:
          (if flow_mode
           then
             Samples.Sankey_presentation.node_labels
               ~middle:placement_mode
               ~long:(placement_mode && long_labels)
               flow_style
           else Chart_node_labels.empty)
        ~label_color:(Palette.foreground p)
        ~axis_color:(Palette.muted p)
        ~grid_color:(Palette.border p)
        ~selection_color:(Palette.accent p)
        ()
      |> ok
    in
    let chart =
      V.chart
        ~key:(Key.of_string_exn "gallery-chart")
        ~on_event
        ~radar_labels:
          (radar_labels
             p
             ~enabled:(rich_radar && Mode.equal current_mode (Family Radar))
             ~activations:label_activations
             ~on_activate:(set_label_activations (label_activations + 1))
             ~editor:label_editor)
        ~style:
          (style
             [ Width
                 (if placement_mode && narrow_flow
                  then px 420.
                  else Length.percent_exn 100.)
             ; Max_width (Length.percent_exn 100.)
             ; Height (px (Palette.size p 300.))
             ; Font_size (Palette.size p 12.)
             ; Font_weight (if placement_mode && bold_labels then 700 else 400)
             ])
        (Chart.Config.create
           ~data:(Registered.handle source.chart)
           ~label:("Chart preview: " ^ Mode.label current_mode)
           ~disabled
           ~options
           ~style:chart_style
           ()
         |> ok)
    in
    Palette.card
      p
      ~title:"Find the story in your data"
      ([ V.row
           ~style:(style [ Gap (px 6.); Wrap Wrap ])
           (List.map Mode.all ~f:(fun candidate ->
              Palette.button
                p
                ~selected:(Mode.equal candidate current_mode)
                (Mode.label candidate)
                (choose candidate)))
       ; V.row
           ~style:(style [ Gap (px 12.); Wrap Wrap ])
           [ V.switch ~checked:horizontal ~on_toggle:toggle_horizontal "Horizontal axes"
           ; V.switch ~checked:reversed ~on_toggle:toggle_reversed "Reverse value axis"
           ; V.switch ~checked:disabled ~on_toggle:toggle_disabled "Disable chart input"
           ; Palette.button p "Update chart samples" update
           ]
       ]
       @ (if Mode.equal current_mode (Family Pie)
          then
            [ V.row
                ~style:(style [ Gap (px 12.); Wrap Wrap ])
                [ V.switch
                    ~checked:fixed_pie
                    ~on_toggle:toggle_fixed_pie
                    "Fixed pie radius 80"
                ; V.switch
                    ~checked:variable_pie
                    ~on_toggle:toggle_variable_pie
                    "Per-slice pie radii"
                ]
            ; Palette.text
                p
                ~muted:true
                "Radii change the picture; slice values and angular shares stay the same."
            ]
          else [])
       @ (if Mode.equal current_mode (Family Radar)
          then
            [ V.row
                ~style:(style [ Gap (px 6.); Wrap Wrap ])
                (List.map Radar_scale.all ~f:(fun candidate ->
                   Palette.button
                     p
                     ~selected:(Radar_scale.equal candidate radar_scale)
                     (Radar_scale.label candidate)
                     (set_radar_scale candidate)))
            ; V.row
                ~style:(style [ Gap (px 12.); Wrap Wrap ])
                [ V.switch
                    ~checked:fixed_radius
                    ~on_toggle:toggle_fixed_radius
                    "Fixed radar radius 80"
                ; V.switch
                    ~checked:spaced_radar
                    ~on_toggle:toggle_spaced_radar
                    "Radar label gap 24"
                ; V.switch
                    ~checked:rich_radar
                    ~on_toggle:toggle_rich_radar
                    "Custom radar labels"
                ; V.switch
                    ~checked:show_radar_labels
                    ~on_toggle:toggle_radar_labels
                    "Show radar labels"
                ]
            ; Palette.text
                p
                ~size:12.
                (sprintf "Radar label activations: %d" label_activations)
            ; Palette.text
                p
                ~muted:true
                "Scale changes geometry, not source values. Values above 50 extend \
                 beyond the grid."
            ]
          else [])
       @ (if Mode.equal current_mode Ordinal_colors
          then
            [ V.switch
                ~checked:unknown_color
                ~on_toggle:toggle_unknown_color
                "Explicit unknown color"
            ; Palette.text
                p
                ~muted:true
                "Update rotates slice order; Build and Research keep their colors. \
                 Review uses the unknown-key policy."
            ]
          else [])
       @ (if stack_mode
          then [ V.switch ~checked:stacked ~on_toggle:toggle_stacked "Stack layers" ]
          else [])
       @ (if Mode.equal current_mode Categorical
          then
            [ V.row
                ~style:(style [ Gap (px 6.); Wrap Wrap ])
                (List.map Category_layout.all ~f:(fun candidate ->
                   Palette.button
                     p
                     ~selected:(Category_layout.equal candidate category_layout)
                     (Category_layout.label candidate)
                     (set_category_layout candidate)))
            ]
          else [])
       @ (if flow_mode
          then
            [ V.row
                ~style:(style [ Gap (px 6.); Wrap Wrap ])
                (List.map Samples.Sankey_presentation.all ~f:(fun candidate ->
                   Palette.button
                     p
                     ~selected:(Samples.Sankey_presentation.equal candidate flow_style)
                     (Samples.Sankey_presentation.label candidate)
                     (set_flow_style candidate)))
            ]
          else [])
       @ (if placement_mode
          then
            [ V.row
                ~style:(style [ Gap (px 12.); Wrap Wrap ])
                [ V.switch
                    ~checked:outside
                    ~on_toggle:toggle_outside
                    "Outside flow labels"
                ; V.switch
                    ~checked:long_labels
                    ~on_toggle:toggle_long_labels
                    "Long flow captions"
                ; V.switch
                    ~checked:bold_labels
                    ~on_toggle:toggle_bold_labels
                    "Bold flow captions"
                ; V.switch
                    ~checked:narrow_flow
                    ~on_toggle:toggle_narrow_flow
                    "Narrow flow plot"
                ; V.switch
                    ~checked:show_labels
                    ~on_toggle:toggle_show_labels
                    "Show flow labels"
                ]
            ]
          else [])
       @ [ V.row
             ~style:(style [ Gap (px 6.); Wrap Wrap ])
             (List.map Samples.Inspection.all ~f:(fun candidate ->
                Palette.button
                  p
                  ~selected:(Samples.Inspection.equal candidate inspection)
                  (Samples.Inspection.label candidate)
                  (set_inspection candidate)))
         ; chart
         ; Palette.text p notice
         ; Palette.text
             p
             (Option.value_map
                description
                ~default:"Select a chart value to inspect it."
                ~f:(fun description -> "Selected: " ^ description))
         ; Palette.text
             p
             ~muted:true
             "Use arrows to explore and Enter to select. View data opens the original \
              values, including any values omitted from the picture."
         ])
;;
