open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module View = Gpuio_bonsai.View
module App = Gpuio_eio.App
module Pointer = Gpuio.Pointer

module Action = struct
  type t =
    | Set of float
    | Adjust of float
end

let component ~phase ~observed _window graph =
  let width, inject =
    B.state_machine0
      ~default_model:180.
      ~apply_action:(fun _ width (action : Action.t) ->
        let width =
          match action with
          | Set value -> value
          | Adjust delta -> width +. delta
        in
        Float.clamp_exn width ~min:80. ~max:360.)
      graph
  in
  let phase = B.Expert.Var.value phase in
  B.Edge.on_change
    phase
    ~equal:Int.equal
    ~callback:(B.return (fun phase -> E.of_thunk (fun () -> observed := phase)))
    graph;
  let open B.Let_syntax in
  let%arr width = width
  and inject = inject
  and phase = phase in
  let config =
    Pointer.Config.create ~label:"Panel width drag handle" ~disabled:(phase = 1) ()
    |> Or_error.ok_exn
  in
  let handle =
    View.pointer_area
      ~key:(Gpuio.Key.of_string_exn "width-handle")
      ~config
      ~style:
        (Gpuio.Style.create_exn
           [ Width (Gpuio.Length.px_exn 360.)
           ; Height (Gpuio.Length.px_exn 48.)
           ; Background (Gpuio.Background.solid (Gpuio.Color.rgb_exn 0x3c5e7e))
           ; Foreground (Gpuio.Color.rgb_exn 0xffffff)
           ]
         |> fun style ->
         Gpuio.Style.with_state_exn
           style
           Pressed
           [ Foreground (Gpuio.Color.rgb_exn 0xffcc66) ])
      ~on_event:(fun event ->
        match event.phase with
        | Started | Moved | Released -> inject (Set event.local_position.x)
        | Cancelled _ -> E.Ignore)
      [ View.text "Drag to resize the panel below" ]
  in
  View.column
    ~style:
      (Gpuio.Style.create_exn
         [ Padding (Gpuio.Length.px_exn 24.); Gap (Gpuio.Length.px_exn 12.) ])
    ([ View.text (sprintf "Panel width: %.0f pixels" width)
     ; View.row
         [ View.button ~on_click:(inject (Adjust (-20.))) "Narrower"
         ; View.button ~on_click:(inject (Adjust 20.)) "Wider"
         ]
     ]
     @ (if phase = 2 then [] else [ handle ])
     @ [ View.column
           ~style:
             (Gpuio.Style.create_exn
                [ Width (Gpuio.Length.px_exn width)
                ; Height (Gpuio.Length.px_exn 72.)
                ; Background (Gpuio.Background.solid (Gpuio.Color.rgb_exn 0xccddff))
                ])
           [ View.text "Resizable content" ]
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
        ~title:"GPUIO pointer capture"
        ~width:640.
        ~height:360.
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
      "GPUIO_POINTER_PUBLIC_OK: native pointer region mount, disable/enable, keyed \
       removal and shutdown")
;;
