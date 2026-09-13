open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module View = Gpuio_bonsai.View
module App = Gpuio_eio.App
module Drag = Gpuio.Drag_and_drop

let component ~phase ~observed _window graph =
  let result, set_result = B.state "Drop text or files here." graph in
  let source_status, set_source_status = B.state "Ready" graph in
  let hovered, set_hovered = B.state false graph in
  let phase = B.Expert.Var.value phase in
  B.Edge.on_change
    phase
    ~equal:Int.equal
    ~callback:(B.return (fun phase -> E.of_thunk (fun () -> observed := phase)))
    graph;
  let open B.Let_syntax in
  let%arr phase = phase
  and result = result
  and set_result = set_result
  and source_status = source_status
  and set_source_status = set_source_status
  and hovered = hovered
  and set_hovered = set_hovered in
  let payload = Drag.Payload.text "Hello from OCaml" |> Or_error.ok_exn in
  let source =
    Drag.Source.create ~label:"Greeting" ~payload ~disabled:(phase = 1) ()
    |> Or_error.ok_exn
  in
  let target =
    Drag.Target.create
      ~label:"Drop text or files"
      ~accept:[ Text; Files ]
      ~disabled:(phase = 1)
      ()
    |> Or_error.ok_exn
  in
  let region color =
    Gpuio.Style.create_exn
      [ Width (Gpuio.Length.px_exn 250.)
      ; Height (Gpuio.Length.px_exn 100.)
      ; Padding (Gpuio.Length.px_exn 16.)
      ; Background (Gpuio.Background.solid (Gpuio.Color.rgb_exn color))
      ; Foreground (Gpuio.Color.rgb_exn 0xffffff)
      ]
  in
  let describe_payload = function
    | Drag.Payload.Text text -> "Received: " ^ text
    | Files files ->
      sprintf "Received %d path(s); no files were opened." (List.length files)
    | Custom { kind; data } ->
      sprintf
        "Received %d bytes of %s"
        (String.length data)
        (Drag.Custom_kind.to_string kind)
  in
  View.column
    ~style:
      (Gpuio.Style.create_exn
         [ Padding (Gpuio.Length.px_exn 24.); Gap (Gpuio.Length.px_exn 16.) ])
    ([ View.text "Native drag and drop"; View.text source_status ]
     @ (if phase = 2
        then []
        else
          [ View.row
              ~style:(Gpuio.Style.create_exn [ Gap (Gpuio.Length.px_exn 16.) ])
              [ View.drag_source
                  ~key:(Gpuio.Key.of_string_exn "greeting")
                  ~config:source
                  ~style:(region 0x3c5e7e)
                  ~on_event:(fun event ->
                    match event.phase with
                    | Started _ -> set_source_status "Dragging"
                    | Desktop_offered -> set_source_status "Files offered to the desktop"
                    | Desktop_unavailable ->
                      set_source_status "Desktop file export unavailable"
                    | Ended Internal_drop -> set_source_status "Dropped in the app"
                    | Ended (Cancelled _) -> set_source_status "Cancelled"
                    | Ended Unconfirmed ->
                      set_source_status "Drag finished without an acknowledged app drop")
                  [ View.text "Drag this greeting" ]
              ; View.drop_target
                  ~key:(Gpuio.Key.of_string_exn "inbox")
                  ~config:target
                  ~style:(region (if hovered then 0x337744 else 0x444b55))
                  ~on_event:(fun event ->
                    match event.phase with
                    | Entered _ -> set_hovered true
                    | Moved -> E.Ignore
                    | Left -> set_hovered false
                    | Dropped payload ->
                      E.Many [ set_hovered false; set_result (describe_payload payload) ]
                    | Rejected reason ->
                      set_result (Sexp.to_string_hum (Drag.Rejection.sexp_of_t reason)))
                  [ View.text "Drop target" ]
              ]
          ])
     @ [ View.text result
       ; View.button
           ~on_click:(set_result (describe_payload payload))
           "Receive greeting with keyboard or click"
       ])
;;

let () =
  let self_test = Array.exists (Sys.get_argv ()) ~f:(String.equal "--self-test") in
  let completed = ref false in
  App.run (fun env app ->
    let phase = B.Expert.Var.create 0 in
    let observed = ref (-1) in
    let window =
      App.open_window
        app
        ~title:"GPUIO drag and drop"
        ~width:640.
        ~height:380.
        (component ~phase ~observed)
      |> Or_error.ok_exn
    in
    if self_test
    then (
      let clock = Eio.Stdenv.clock env in
      Gpuio_eio.Scope.start
        (App.scope app)
        ~f:(fun () ->
          Eio.Time.with_timeout_exn clock 15. (fun () ->
            let render value =
              B.Expert.Var.set phase value;
              while !observed <> value do
                Eio.Time.sleep clock 0.005
              done;
              let promise, resolver = Eio.Promise.create () in
              App.Window.request_frame window ~on_rendered:(fun ~revision ->
                E.of_thunk (fun () -> Eio.Promise.resolve resolver revision))
              |> Or_error.ok_exn;
              Eio.Promise.await promise
            in
            let mounted = render 0 in
            let disabled = render 1 in
            let enabled = render 0 in
            let removed = render 2 in
            assert (Int64.(mounted < disabled && disabled < enabled && enabled < removed))))
        ~on_result:(fun result ->
          E.of_thunk (fun () ->
            Or_error.ok_exn result;
            completed := true;
            App.Window.close window))
      |> Or_error.ok_exn
      |> fun (_ : Gpuio_eio.Scope.Task.t) -> ()));
  if self_test
  then (
    assert !completed;
    Eio.traceln
      "GPUIO_DRAG_DROP_PUBLIC_OK: mount, disable/enable, keyed removal and shutdown (no \
       injected gestures)")
;;
