open Core
open Gpuio
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View
module Registered = Gpuio_eio.Chart
module Data = Chart_backgrounds_data

let ok = Or_error.ok_exn
let px = Length.px_exn
let style = Style.create_exn

let appearance p ~sparse ~uniform =
  let data =
    if not sparse
    then []
    else
      [ Chart_appearance.Datum.create
          ~series:Data.series_id
          ~datum:Data.first_id
          ~bar:
            (Chart_appearance.Bar.create
               ~fill:
                 (Chart_appearance.Bar_fill.background
                    (Background.solid (Color.rgb_exn 0xf59e0b)))
               ~corners:(Chart_appearance.Corners.all 4. |> ok)
               ())
          ()
      ]
  in
  Chart_style.create
    ~palette:[ Palette.accent p ]
    ~label_color:(Palette.foreground p)
    ~axis_color:(Palette.muted p)
    ~grid_color:(Palette.border p)
    ~selection_color:(Palette.accent p)
    ~appearance:
      (Chart_appearance.create
         ~data
         ~aggregates:(if uniform then Uniform else Inherit_series)
         ()
       |> ok)
    ()
  |> ok
;;

let component app window palette graph =
  let model = B.Expert.Var.create Data.initial in
  let selected = B.Expert.Var.create None in
  let color_applications = B.Expert.Var.create 0 in
  let source_theme = ref Theme.default in
  let resources =
    Preview_scope.acquire
      window
      ~name:"gallery-bar-backgrounds"
      ~create:(fun scope ->
        B.Expert.Var.set selected None;
        E.map
          (Registered.create
             app
             ~scope
             (Data.data_exn (B.Expert.Var.get model) ~theme:!source_theme))
          ~f:
            (Result.map_error ~f:(fun error ->
               Error.create_s [%sexp (error : Registered.Error.t)])))
      graph
  in
  let horizontal, toggle_horizontal = B.toggle ~default_model:false graph in
  let reversed, toggle_reversed = B.toggle ~default_model:false graph in
  let sparse, toggle_sparse = B.toggle ~default_model:false graph in
  let aggregate, toggle_aggregate = B.toggle ~default_model:false graph in
  let uniform, toggle_uniform = B.toggle ~default_model:false graph in
  let notice, set_notice = B.state "Preparing bar backgrounds…" graph in
  let open B.Let_syntax in
  B.Edge.lifecycle
    ~on_deactivate:
      (let%arr set_notice = set_notice in
       set_notice "Preparing bar backgrounds…")
    graph;
  let%arr p = palette
  and resources = resources
  and current = B.Expert.Var.value model
  and selection = B.Expert.Var.value selected
  and applications = B.Expert.Var.value color_applications
  and horizontal = horizontal
  and toggle_horizontal = toggle_horizontal
  and reversed = reversed
  and toggle_reversed = toggle_reversed
  and sparse = sparse
  and toggle_sparse = toggle_sparse
  and aggregate = aggregate
  and toggle_aggregate = toggle_aggregate
  and uniform = uniform
  and toggle_uniform = toggle_uniform
  and notice = notice
  and set_notice = set_notice in
  match resources with
  | Preview_scope.Loading -> Palette.text p "Preparing bar backgrounds…"
  | Failed error -> Palette.text p (Error.to_string_hum error)
  | Ready source ->
    let publish ?theme change =
      E.bind
        (E.of_thunk (fun () ->
           let next = change (B.Expert.Var.get model) in
           let next_theme = Option.value theme ~default:!source_theme in
           Result.map
             (Registered.set source (Data.data_exn next ~theme:next_theme))
             ~f:(fun () ->
               B.Expert.Var.set model next;
               source_theme := next_theme;
               if Option.is_some theme
               then
                 B.Expert.Var.set
                   color_applications
                   (B.Expert.Var.get color_applications + 1))))
        ~f:(function
          | Ok () -> set_notice "Publishing bar backgrounds…"
          | Error error ->
            set_notice (Sexp.to_string_hum [%sexp (error : Registered.Error.t)]))
    in
    let orientation, direction =
      match horizontal, reversed with
      | false, false -> Chart_options.Orientation.Vertical, "Vertical"
      | true, false -> Horizontal, "Horizontal"
      | false, true -> Vertical_reversed, "Vertical reversed"
      | true, true -> Horizontal_reversed, "Horizontal reversed"
    in
    let on_event (event : Chart.Event.t) =
      match event.observation with
      | Ready metrics ->
        if not (Registered.is_published source)
        then E.Ignore
        else
          set_notice
            (sprintf
               "Ready: Bar backgrounds · %d source values · %s · %s"
               metrics.source_values
               direction
               (if aggregate then "Mean" else "Exact"))
      | Failed error -> set_notice (Sexp.to_string_hum [%sexp (error : Chart.Error.t)])
      | Selection_changed target ->
        E.of_thunk (fun () -> B.Expert.Var.set selected target)
    in
    let description =
      if not (Registered.is_published source)
      then None
      else
        Option.bind (Registered.data source) ~f:(fun data ->
          Option.bind selection ~f:(Gpuio_chart_samples.describe_selection data))
    in
    let sampling =
      Chart_sampling.create
        ~bars:
          (if aggregate
           then Chart_sampling.Bar.mean ~max_buckets:6 |> ok
           else Chart_sampling.Bar.exact)
        ()
    in
    let chart =
      V.chart
        ~key:(Key.of_string_exn "background-chart")
        ~on_event
        ~style:
          (style
             [ Width (Length.percent_exn 100.)
             ; Height (px (Palette.size p 320.))
             ; Font_size (Palette.size p 11.)
             ])
        (Chart.Config.create
           ~data:(Registered.handle source)
           ~label:"Chart preview: Bar backgrounds"
           ~style:(appearance p ~sparse ~uniform)
           ~options:
             (Chart_options.create
                ~cartesian:(Chart_options.Cartesian.create ~orientation () |> ok)
                ())
           ~sampling
           ()
         |> ok)
    in
    Palette.card
      p
      ~title:"Every batch has its own color"
      [ Palette.text
          p
          ~muted:true
          "Colors follow batch identities as the source changes or reorders. Pattern \
           gaps reveal the chart surface."
      ; V.row
          ~style:(style [ Gap (px 8.); Wrap Wrap ])
          [ Palette.button p "Update batch values" (publish Data.advance)
          ; Palette.button p "Reorder batches" (publish Data.reorder)
          ; Palette.button
              p
              ~selected:(Data.has_patterns current)
              "Pattern backgrounds"
              (publish Data.toggle_patterns)
          ; Palette.button
              p
              "Apply preview colors"
              (publish ~theme:(Palette.theme p) Fn.id)
          ]
      ; V.row
          ~style:(style [ Gap (px 12.); Wrap Wrap ])
          [ V.switch
              ~checked:horizontal
              ~on_toggle:toggle_horizontal
              "Horizontal background bars"
          ; V.switch
              ~checked:reversed
              ~on_toggle:toggle_reversed
              "Reverse background axis"
          ; V.switch ~checked:sparse ~on_toggle:toggle_sparse "Highlight Batch 01"
          ; V.switch ~checked:aggregate ~on_toggle:toggle_aggregate "Mean background bars"
          ; V.switch
              ~checked:uniform
              ~on_toggle:toggle_uniform
              "Uniform background agreement"
          ]
      ; chart
      ; Palette.text p notice
      ; Palette.text
          p
          (sprintf
             "Sample step: %d · %s · %s · Color applications: %d"
             (Data.phase current)
             (if Data.is_reordered current then "Reordered" else "Original order")
             (if Data.has_patterns current then "Patterns" else "Solid fills")
             applications)
      ; Palette.text
          p
          (Option.value_map
             description
             ~default:"Select a batch to inspect its original value."
             ~f:(fun text -> "Selected: " ^ text))
      ; Palette.text
          p
          ~size:12.
          ~muted:true
          "Switch the preview theme, then Apply preview colors to publish new source \
           colors. View styling alone does not rewrite the stored brushes. Mean bars \
           retain original data for inspection."
      ]
;;
