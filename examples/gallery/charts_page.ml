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
  [@@deriving equal]

  let all =
    List.map Family.all ~f:(fun f -> Family f)
    @ [ Mixed; Categorical; Stacked_bars; Stacked_areas; Ordinal_colors ]
  ;;

  let label = function
    | Family family -> Family.label family
    | Mixed -> "Mixed layers"
    | Categorical -> "Categorical"
    | Stacked_bars -> "Stacked bars"
    | Stacked_areas -> "Stacked areas"
    | Ordinal_colors -> "Ordinal colors"
  ;;

  let data t phase =
    match t with
    | Family family -> Samples.data_exn family phase
    | Mixed -> Samples.preset_data_exn Mixed Line phase
    | Categorical -> Samples.Categorical.data_exn phase
    | Stacked_bars -> Samples.Stacked.data_exn ~area:false phase
    | Stacked_areas -> Samples.Stacked.data_exn ~area:true phase
    | Ordinal_colors -> Samples.Ordinal_colors.data_exn phase
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
  let horizontal, toggle_horizontal = B.toggle ~default_model:false graph in
  let reversed, toggle_reversed = B.toggle ~default_model:false graph in
  let category_layout, set_category_layout = B.state Category_layout.Auto graph in
  let unknown_color, toggle_unknown_color = B.toggle ~default_model:true graph in
  let stacked, toggle_stacked = B.toggle ~default_model:true graph in
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
  and disabled = disabled
  and toggle_disabled = toggle_disabled
  and notice = notice
  and set_notice = set_notice in
  match resources with
  | Preview_scope.Loading -> Palette.text p "Preparing chart…"
  | Failed e -> Palette.text p ("Chart unavailable: " ^ Error.to_string_hum e)
  | Ready source ->
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
             "Ready: %s · %d source values · %s"
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
              else direction))
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
      | Family _ | Mixed | Categorical | Ordinal_colors -> false
    in
    let options =
      Chart_options.create
        ~cartesian:
          (Chart_options.Cartesian.create
             ~orientation
             ~stacking:(if stack_mode && stacked then Stacked else Grouped)
             ~category_layout:(Category_layout.options category_layout)
             ()
           |> ok)
        ~pie:(Chart_options.Pie.create ~inner_radius:0.5 () |> ok)
        ()
    in
    let chart_style =
      Chart_style.create
        ?ordinal:
          (Option.some_if
             (Mode.equal current_mode Ordinal_colors)
             (Samples.Ordinal_colors.mapping ~unknown:unknown_color))
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
        ~style:
          (style
             [ Width (Length.percent_exn 100.)
             ; Height (px (Palette.size p 300.))
             ; Font_size (Palette.size p 12.)
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
       @ [ chart
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
