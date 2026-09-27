open Core
module App = Gpuio_eio.App
module Scope = Gpuio_eio.Scope
module Scene = Gpuio_eio.Canvas
module Canvas = Gpuio.Canvas
module G = Gpuio.Canvas_geometry
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View

module Snapshot = struct
  type t =
    { plot : Plot.t
    ; handle : Gpuio.Canvas_scene.Handle.t option
    ; command : Canvas.Command.t option
    ; selected : Gpuio.Canvas_scene.Item_id.t option
    ; disabled : bool
    ; hidden : bool
    ; status : string
    ; ack : int64
    ; epoch : int64 * int64
    }
end

let scene_ok = function
  | Ok value -> value
  | Error error -> raise_s [%sexp (error : Scene.Error.t)]
;;

let color = Gpuio.Color.rgb_exn
let px = Gpuio.Length.px_exn
let style = Gpuio.Style.create_exn
let background rgb = Gpuio.Style.Property.Background (Gpuio.Background.solid (color rgb))

let text ?(size = 14.) ?(tint = 0x9eadc2) value =
  V.text ~style:(style [ Font_size size; Foreground (color tint) ]) value
;;

let () =
  let argument name = Array.exists (Sys.get_argv ()) ~f:(String.equal name) in
  let self_test = argument "--self-test" in
  let large = argument "--large" in
  let completed = ref false in
  App.run (fun env app ->
    let initial = Plot.create ~large in
    let state =
      B.Expert.Var.create
        { Snapshot.plot = initial
        ; handle = None
        ; command = None
        ; selected = None
        ; disabled = false
        ; hidden = false
        ; status = "Preparing native scene…"
        ; ack = 0L
        ; epoch = 0L, 0L
        }
    in
    let registered = ref None in
    let sequence = ref 0L in
    let update f = B.Expert.Var.set state (f (B.Expert.Var.get state)) in
    let command action =
      sequence := Int64.succ !sequence;
      let command = Canvas.Command.create ~sequence:!sequence action |> Or_error.ok_exn in
      update (fun state -> { state with command = Some command });
      !sequence
    in
    let publish ~reset plot =
      let canvas = Option.value_exn !registered in
      (if reset then Scene.reset else Scene.set) canvas (Plot.scene plot) |> scene_ok;
      update (fun state ->
        { state with
          plot
        ; status = (if reset then "Dataset reset" else "Published sample positions")
        })
    in
    let name state id =
      Plot.find state.Snapshot.plot id
      |> Option.value_map ~default:"Unknown" ~f:Plot.Sample.name
    in
    let on_event (event : Canvas.Event.t) =
      E.of_thunk (fun () ->
        update (fun state ->
          { state with epoch = event.scene_revision, event.scene_generation });
        if self_test
        then
          Eio.traceln
            "CANVAS_PUBLIC_EVENT: %s"
            (Sexp.to_string_hum ([%sexp_of: Canvas.Event.t] event));
        match event.observation with
        | Selection_changed selected -> update (fun state -> { state with selected })
        | Activated id ->
          update (fun state -> { state with status = "Activated: " ^ name state id })
        | Moved (id, transform) ->
          let current = B.Expert.Var.get state in
          let plot = Plot.move current.plot id transform |> Or_error.ok_exn in
          publish ~reset:false plot;
          update (fun state -> { state with status = "Moved: " ^ name state id })
        | Viewport_changed viewport ->
          update (fun state ->
            { state with
              status = sprintf "Viewport · %.0f%%" (Canvas.Viewport.zoom viewport *. 100.)
            })
        | Command_completed ack -> update (fun state -> { state with ack })
        | Failed error ->
          raise_s [%sexp "Canvas rendering failed", (error : Canvas.Error.t)])
    in
    let thunk f = E.of_thunk f in
    let component _window _graph =
      let open B.Let_syntax in
      let%arr snapshot = B.Expert.Var.value state in
      let button title action =
        V.button
          title
          ~on_click:(thunk action)
          ~style:
            (style
               [ background 0x233046
               ; Foreground (color 0xe2eaf6)
               ; Radius 8.
               ; Padding (px 10.)
               ])
      in
      let chart =
        match snapshot.handle with
        | None -> text "Registering the scene…"
        | Some scene ->
          let config =
            Canvas.Config.create
              ~scene
              ~label:"Model evaluation plot"
              ~disabled:snapshot.disabled
              ~selection_color:(color 0xffffff)
              ?command:snapshot.command
              ()
            |> Or_error.ok_exn
          in
          V.canvas
            ~key:(Gpuio.Key.of_string_exn "plot")
            ~on_event
            config
            ~style:
              (style
                 [ Width (px 700.)
                 ; Height (px 470.)
                 ; Shrink 0.
                 ; background 0x111923
                 ; Radius 14.
                 ; Display (if snapshot.hidden then Hidden else Flex)
                 ])
      in
      let details =
        match snapshot.selected with
        | None -> "Choose a sample to inspect its position."
        | Some id ->
          let sample = Plot.find snapshot.plot id |> Option.value_exn in
          let p = Plot.Sample.position sample in
          sprintf
            "%s · x %.1f · y %.1f"
            (Plot.Sample.name sample)
            (G.Point.x p)
            (G.Point.y p)
      in
      V.column
        ~style:
          (style
             [ Width (Gpuio.Length.percent_exn 100.)
             ; Height (Gpuio.Length.percent_exn 100.)
             ; Padding (px 20.)
             ; Gap (px 12.)
             ; Overflow_y Scroll
             ; background 0x0b111b
             ; Foreground (color 0xe2eaf6)
             ])
        [ V.row
            ~style:(style [ Justify_content Space_between; Align_items Center ])
            [ V.column
                ~style:(style [ Gap (px 6.) ])
                [ text ~size:12. ~tint:0x65dec1 "GPUIO / CANVAS LAB"
                ; text ~size:28. ~tint:0xf1f5fb "A native view of your data"
                ; text "Simulated model evaluations · OCaml scenes, native interaction"
                ]
            ; button "Close lab" (fun () -> App.shutdown app)
            ]
        ; V.row
            ~style:(style [ Gap (px 20.); Align_items Start ])
            [ chart
            ; V.column
                ~style:(style [ Width (px 230.); Gap (px 12.) ])
                ([ text ~tint:0xe2eaf6 "SAMPLES" ]
                 @ List.map (Plot.samples snapshot.plot) ~f:(fun sample ->
                   V.button
                     (Plot.Sample.name sample)
                     ~on_click:
                       (thunk (fun () ->
                          ignore (command (Select (Some (Plot.Sample.id sample))) : int64)))
                     ~style:
                       (style
                          [ Foreground (Plot.Sample.color sample)
                          ; background 0x172130
                          ; Padding (px 8.)
                          ; Radius 8.
                          ]))
                 @ [ text ~size:12. "SELECTED"
                   ; text ~tint:0xe2eaf6 details
                   ; text
                       ~size:12.
                       "Drag a sample. Use arrows to select, Shift+arrows to move, and \
                        Enter to activate."
                   ; text
                       ~size:12.
                       "Wheel or middle-drag pans. Control+wheel and +/− zoom. Escape \
                        cancels a drag."
                   ])
            ]
        ; V.row
            ~style:(style [ Gap (px 10.) ])
            [ button "Reset viewport" (fun () -> ignore (command Reset_viewport : int64))
            ; button "Reset dataset" (fun () -> publish ~reset:true initial)
            ; button
                (if snapshot.disabled then "Enable input" else "Disable input")
                (fun () ->
                   update (fun state -> { state with disabled = not state.disabled }))
            ; button
                (if snapshot.hidden then "Show plot" else "Hide plot")
                (fun () -> update (fun state -> { state with hidden = not state.hidden }))
            ]
        ; text
            ~size:12.
            ~tint:0x65dec1
            (let revision, generation = snapshot.epoch in
             sprintf
               "%s  ·  Native scene revision %Ld · generation %Ld  ·  %s decorative \
                points"
               snapshot.status
               revision
               generation
               (if large then "19,000" else "160"))
        ]
    in
    let window =
      App.open_window
        app
        ~focus:true
        ~title:"GPUIO · Canvas Lab"
        ~width:1040.
        ~height:840.
        component
      |> Or_error.ok_exn
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
          let ui f = on_ui (thunk f) in
          let rec until predicate =
            if not (ui predicate)
            then (
              Eio.Time.sleep clock 0.005;
              until predicate)
          in
          let scene = Plot.scene initial in
          let canvas = on_ui (Scene.create app ~scope scene) |> scene_ok in
          Eio.traceln "CANVAS_PUBLIC_STAGE: registered";
          ui (fun () ->
            registered := Some canvas;
            update (fun state ->
              { state with
                handle = Some (Scene.handle canvas)
              ; status = "Ready to explore"
              }));
          if self_test
          then (
            let first = List.hd_exn (Plot.samples initial) |> Plot.Sample.id in
            let selected = on_ui (E.of_thunk (fun () -> command (Select (Some first)))) in
            until (fun () -> Int64.((B.Expert.Var.get state).ack >= selected));
            Eio.traceln "CANVAS_PUBLIC_STAGE: selection acknowledged";
            ui (fun () ->
              assert (
                Option.equal
                  Gpuio.Canvas_scene.Item_id.equal
                  (B.Expert.Var.get state).selected
                  (Some first)));
            let viewport =
              Canvas.Viewport.create
                ~origin:(G.Point.create ~x:10. ~y:20. |> Or_error.ok_exn)
                ~zoom:1.25
              |> Or_error.ok_exn
            in
            let zoom = on_ui (E.of_thunk (fun () -> command (Set_viewport viewport))) in
            until (fun () -> Int64.((B.Expert.Var.get state).ack >= zoom));
            Eio.traceln "CANVAS_PUBLIC_STAGE: zoom acknowledged";
            let moved =
              Plot.move
                initial
                first
                (G.Transform.translate ~x:190. ~y:310. |> Or_error.ok_exn)
              |> Or_error.ok_exn
            in
            ui (fun () -> publish ~reset:false moved);
            until (fun () -> Scene.is_published canvas);
            Eio.traceln "CANVAS_PUBLIC_STAGE: publication accepted";
            ui (fun () -> publish ~reset:true initial);
            until (fun () -> Scene.is_published canvas);
            Eio.traceln "CANVAS_PUBLIC_STAGE: publication accepted";
            let reset = on_ui (E.of_thunk (fun () -> command Reset_viewport)) in
            until (fun () -> Int64.((B.Expert.Var.get state).ack >= reset));
            Eio.traceln "CANVAS_PUBLIC_STAGE: reset acknowledged";
            ui (fun () ->
              let revision, generation = (B.Expert.Var.get state).epoch in
              assert (Int64.(revision >= 3L));
              assert (Int64.equal generation 2L));
            let frame () =
              let promise, resolver = Eio.Promise.create () in
              ui (fun () ->
                App.Window.request_frame window ~on_rendered:(fun ~revision:_ ->
                  thunk (fun () -> Eio.Promise.resolve resolver ()))
                |> Or_error.ok_exn);
              Eio.Promise.await promise
            in
            Eio.traceln "CANVAS_PUBLIC_STAGE: requesting frame";
            frame ();
            Eio.Time.sleep clock 0.1;
            let before = on_ui (E.of_thunk (fun () -> App.stats app)) in
            Eio.Time.sleep clock 0.2;
            ui (fun () -> assert ((App.stats app).commits = before.commits));
            ui (fun () ->
              Scene.release canvas;
              update (fun state -> { state with handle = None }));
            Eio.traceln "CANVAS_PUBLIC_STAGE: requesting frame";
            frame ();
            ui (fun () ->
              assert (Scene.is_released canvas);
              completed := true);
            Eio.traceln
              "GPUIO_CANVAS_PUBLIC_OK: %d items, %d encoded bytes; scoped create, typed \
               view, command observations, set/reset epochs, idle commits and \
               release/unmount"
              (Gpuio.Canvas_scene.item_count scene)
              (Gpuio.Canvas_scene.encoded_bytes scene))))
      ~on_result:(fun result ->
        thunk (fun () ->
          Or_error.ok_exn result;
          if self_test then App.shutdown app))
    |> Or_error.ok_exn
    |> fun (_ : Scope.Task.t) -> ());
  if self_test then assert !completed;
  print_endline "GPUIO_CANVAS_APP_RETURNED"
;;
