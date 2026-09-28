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
  let completed = ref false in
  App.run (fun env app ->
    let family = B.Expert.Var.create 0 in
    let handle = B.Expert.Var.create None in
    let status = B.Expert.Var.create "Preparing your workspace" in
    let registration = ref None in
    let observation = ref None in
    let phase = ref 0. in
    let choose n =
      Option.iter !registration ~f:(fun chart ->
        Registered.reset chart (Gallery.data n !phase) |> checked);
      observation := None;
      B.Expert.Var.set family n;
      B.Expert.Var.set status "Updating chart"
    in
    let on_event (event : Chart.Event.t) =
      E.of_thunk (fun () ->
        observation := Some event;
        match event.observation with
        | Failed error ->
          B.Expert.Var.set status (Sexp.to_string_hum [%sexp (error : Chart.Error.t)])
        | Ready metrics ->
          B.Expert.Var.set
            status
            (sprintf
               "%s values · native rendering"
               (Int.to_string_hum metrics.source_values)))
    in
    let component _window _graph =
      let open B.Let_syntax in
      let%arr family = B.Expert.Var.value family
      and handle = B.Expert.Var.value handle
      and status = B.Expert.Var.value status in
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
                  button name ~selected:(i = family) (fun () -> choose i))))
        ; V.column
            ~style:(style [ bg 0x172230; Radius 18.; Padding (px 26.); Gap (px 22.) ])
            [ V.row
                ~style:
                  (style
                     [ Gap (px 20.); Justify_content Space_between; Align_items Center ])
                [ V.column
                    ~style:(style [ Gap (px 6.) ])
                    [ text ~size:21. ~tint:0xf0f5ff Gallery.descriptions.(family)
                    ; text "A small dataset. A clear perspective."
                    ]
                ; text ~size:12. ~tint:0x72d8c4 Gallery.names.(family)
                ]
            ; chart
            ; V.row
                ~style:
                  (style
                     [ Gap (px 12.); Justify_content Space_between; Align_items Center ])
                [ text ~size:12. status
                ; button "Update data ↗" ~selected:false (fun () ->
                    phase := !phase +. 1.;
                    Option.iter !registration ~f:(fun chart ->
                      Registered.set chart (Gallery.data family !phase) |> checked))
                ]
            ]
        ; text ~size:12. "Built with OCaml, Bonsai and GPUI  ·  Sample data"
        ]
    in
    let window =
      App.open_window
        app
        ~focus:(not (self_test || background))
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
          let chart =
            on_ui (Registered.create app ~scope (Gallery.data 0 0.)) |> checked
          in
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
                | None -> false)
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
