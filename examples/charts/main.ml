open Core
module App = Gpuio_eio.App
module Scope = Gpuio_eio.Scope
module Registered = Gpuio_eio.Chart
module Chart = Gpuio.Chart
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View

let ok = Or_error.ok_exn

let checked = function
  | Ok x -> x
  | Error e -> raise_s [%sexp (e : Registered.Error.t)]
;;

let color = Gpuio.Color.rgb_exn
let px = Gpuio.Length.px_exn
let style = Gpuio.Style.create_exn
let bg c = Gpuio.Style.Property.Background (Gpuio.Background.solid (color c))

let text ?(size = 14.) ?(tint = 0x96a4bc) s =
  V.text ~style:(style [ Font_size size; Foreground (color tint) ]) s
;;

let () =
  let self_test = Array.exists (Sys.get_argv ()) ~f:(String.equal "--self-test") in
  let background = Array.exists (Sys.get_argv ()) ~f:(String.equal "--background") in
  let foreground = Array.exists (Sys.get_argv ()) ~f:(String.equal "--foreground") in
  let completed = ref false in
  App.run (fun env app ->
    let family = B.Expert.Var.create 0 in
    let edge_cases = B.Expert.Var.create false in
    let dataset family phase =
      if B.Expert.Var.get edge_cases
      then Gallery.edge_data family
      else Gallery.data family phase
    in
    let handle = B.Expert.Var.create None in
    let status = B.Expert.Var.create "Preparing your workspace" in
    let selection = B.Expert.Var.create None in
    let registration = ref None in
    let observation = ref None in
    let phase = ref 0. in
    let choose n =
      Option.iter !registration ~f:(fun chart ->
        Registered.reset chart (dataset n !phase) |> checked);
      observation := None;
      B.Expert.Var.set selection None;
      B.Expert.Var.set family n;
      B.Expert.Var.set status "Updating chart"
    in
    let on_event (event : Chart.Event.t) =
      E.of_thunk (fun () ->
        match event.observation with
        | Selection_changed target ->
          let description =
            Option.bind !registration ~f:(fun chart ->
              if Registered.is_published chart
              then
                Option.bind (Registered.data chart) ~f:(fun data ->
                  Option.bind target ~f:(Gallery.describe_selection data))
              else None)
          in
          B.Expert.Var.set selection description
        | Failed error ->
          observation := Some event;
          B.Expert.Var.set status (Sexp.to_string_hum [%sexp (error : Chart.Error.t)])
        | Ready metrics ->
          observation := Some event;
          B.Expert.Var.set
            status
            (sprintf
               "%s values · native rendering"
               (Int.to_string_hum metrics.source_values)))
    in
    let toggle_data () =
      B.Expert.Var.set edge_cases (not (B.Expert.Var.get edge_cases));
      choose (B.Expert.Var.get family)
    in
    let clear_selection () = B.Expert.Var.set selection None in
    let component _window _graph =
      let open B.Let_syntax in
      let%arr family = B.Expert.Var.value family
      and edge_cases = B.Expert.Var.value edge_cases
      and handle = B.Expert.Var.value handle
      and status = B.Expert.Var.value status
      and selection = B.Expert.Var.value selection in
      let button label ~selected f =
        V.button
          label
          ~on_click:(E.of_thunk f)
          ~style:
            (style
               [ bg (if selected then 0x29485b else 0x182332)
               ; Foreground (color (if selected then 0xa3e7df else 0xa5b2c8))
               ; Radius 10.
               ; Padding (px 11.)
               ])
      in
      let chart =
        match handle with
        | None -> text "Loading chart data…"
        | Some data ->
          let options =
            Gpuio.Chart_options.create
              ~pie:(Gpuio.Chart_options.Pie.create ~inner_radius:0.56 () |> ok)
              ()
          in
          let config =
            Chart.Config.create ~data ~label:Gallery.descriptions.(family) ~options ()
            |> ok
          in
          V.chart
            ~key:(Gpuio.Key.of_string_exn "main-chart")
            ~on_event
            config
            ~style:(style [ Width (px 760.); Height (px 330.) ])
      in
      V.column
        ~style:
          (style
             [ Width (Gpuio.Length.percent_exn 100.)
             ; Height (Gpuio.Length.percent_exn 100.)
             ; Padding (px 32.)
             ; Gap (px 24.)
             ; bg 0x101823
             ])
        [ V.row
            ~style:(style [ Gap (px 12.); Align_items Center ])
            [ V.column
                ~style:(style [ Gap (px 7.) ])
                [ text ~size:12. ~tint:0x72d8c4 "GPUIO   /   CHART STUDIO"
                ; text ~size:30. ~tint:0xf0f5ff "Make your data visible."
                ]
            ]
        ; V.row
            ~style:(style [ Gap (px 8.) ])
            (Array.to_list
               (Array.mapi Gallery.names ~f:(fun i name ->
                  button name ~selected:(i = family) (fun () -> choose i)))
             @ [ button
                   (if edge_cases then "Sample data" else "Edge cases")
                   ~selected:edge_cases
                   toggle_data
               ])
        ; V.column
            ~style:(style [ bg 0x172230; Radius 18.; Padding (px 26.); Gap (px 22.) ])
            [ V.row
                ~style:
                  (style
                     [ Gap (px 20.); Justify_content Space_between; Align_items Center ])
                [ V.column
                    ~style:(style [ Gap (px 6.) ])
                    [ text ~size:21. ~tint:0xf0f5ff Gallery.descriptions.(family)
                    ; text
                        (if edge_cases
                         then
                           "Original data, including gaps, zero values and empty \
                            datasets."
                         else "A small dataset. A clear perspective.")
                    ]
                ; text ~size:12. ~tint:0x72d8c4 Gallery.names.(family)
                ]
            ; chart
            ; text
                ~size:12.
                ~tint:0xa3e7df
                (Option.value_map
                   selection
                   ~default:
                     "Select a value · Click or use arrows, then Enter · Escape clears"
                   ~f:(fun label -> "Selected: " ^ label))
            ; V.row
                ~style:
                  (style
                     [ Gap (px 12.); Justify_content Space_between; Align_items Center ])
                [ text ~size:12. status
                ; button "Update data ↗" ~selected:false (fun () ->
                    phase := !phase +. 1.;
                    clear_selection ();
                    Option.iter !registration ~f:(fun chart ->
                      Registered.set chart (dataset family !phase) |> checked))
                ]
            ]
        ; text ~size:12. "Built with OCaml, Bonsai and GPUI  ·  Sample data"
        ]
    in
    let window =
      App.open_window
        app
        ~focus:(foreground || not (self_test || background))
        ~title:"GPUIO · Chart Studio"
        ~width:920.
        ~height:820.
        component
      |> ok
    in
    let scope = App.Window.scope window in
    Scope.start
      scope
      ~f:(fun () ->
        let clock = Eio.Stdenv.clock env in
        Eio.Time.with_timeout_exn clock 60. (fun () ->
          let on_ui ui_effect =
            let promise, resolver = Eio.Promise.create () in
            Scope.Expert.enqueue scope (fun () ->
              E.Expert.handle (E.map ui_effect ~f:(Eio.Promise.resolve resolver)));
            Eio.Promise.await promise
          in
          let ui f = on_ui (E.of_thunk f) in
          let rec until f =
            if not (ui f)
            then (
              Eio.Time.sleep clock 0.005;
              until f)
          in
          let chart = on_ui (Registered.create app ~scope (dataset 0 0.)) |> checked in
          ui (fun () ->
            registration := Some chart;
            B.Expert.Var.set handle (Some (Registered.handle chart)));
          if self_test
          then (
            let wait_ready generation =
              until (fun () ->
                match !observation with
                | Some { Chart.Event.data_generation; observation = Ready _; _ } ->
                  Int64.equal data_generation generation
                | Some { observation = Failed e; _ } ->
                  raise_s [%sexp (e : Chart.Error.t)]
                | Some { observation = Selection_changed _; _ } | None -> false)
            in
            wait_ready 1L;
            for n = 1 to 6 do
              ui (fun () -> choose n);
              wait_ready (Int64.of_int (n + 1));
              Eio.traceln "CHART_PUBLIC_READY: %s" Gallery.names.(n)
            done;
            let frame = Eio.Promise.create () in
            ui (fun () ->
              App.Window.request_frame window ~on_rendered:(fun ~revision:_ ->
                E.of_thunk (fun () -> Eio.Promise.resolve (snd frame) ()))
              |> ok);
            Eio.Promise.await (fst frame);
            ui (fun () ->
              Registered.release chart;
              B.Expert.Var.set handle None);
            until (fun () -> (App.diagnostics app).charts = 0);
            ui (fun () ->
              assert ((App.diagnostics app).chart_data_bytes = 0);
              completed := true);
            Eio.traceln
              "GPUIO_CHART_PUBLIC_OK: seven families through Bonsai/Eio/FFI, reset \
               epochs, frame callback, scoped release")))
      ~on_result:(fun result ->
        E.of_thunk (fun () ->
          ok result;
          if self_test then App.shutdown app))
    |> ok
    |> fun (_ : Scope.Task.t) -> ());
  if self_test then assert !completed
;;
