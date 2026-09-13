open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module View = Gpuio_bonsai.View
module App = Gpuio_eio.App
module Toast = Gpuio.Toast

module Model = struct
  type t =
    { next : int
    ; items : int list
    }
end

module Action = struct
  type t =
    | Add
    | Dismiss of int
end

let component ~self_test ~dismissed _window graph =
  let model, inject =
    B.state_machine0
      ~default_model:Model.{ next = 1; items = [ 0 ] }
      ~apply_action:(fun _ (model : Model.t) (action : Action.t) ->
        match action with
        | Add -> { next = model.next + 1; items = model.items @ [ model.next ] }
        | Dismiss id ->
          { model with items = List.filter model.items ~f:(fun item -> item <> id) })
      graph
  in
  let config =
    let timeout =
      Toast.Timeout.after (Time_ns.Span.of_sec (if self_test then 0.3 else 5.))
      |> Or_error.ok_exn
    in
    Toast.Config.create ~label:"Draft saved" ~timeout () |> Or_error.ok_exn
  in
  let open B.Let_syntax in
  let%arr model = model
  and inject = inject in
  let items =
    List.map model.items ~f:(fun id ->
      View.toast
        ~key:(Gpuio.Key.of_string_exn (Int.to_string id))
        ~config
        ~on_dismiss:(fun reason ->
          E.Many [ inject (Dismiss id); E.of_thunk (fun () -> dismissed := Some reason) ])
        [ View.text "Your draft has been saved."
        ; View.button ~on_click:(inject (Dismiss id)) "Dismiss from application"
        ])
  in
  View.column
    ~style:
      (Gpuio.Style.create_exn
         [ Padding (Gpuio.Length.px_exn 24.); Gap (Gpuio.Length.px_exn 12.) ])
    [ View.text "Native notifications with application-owned content"
    ; View.button ~on_click:(inject Add) "Save another draft"
    ; View.toast_stack items |> Or_error.ok_exn
    ]
;;

let () =
  let self_test = Array.exists (Sys.get_argv ()) ~f:(String.equal "--self-test") in
  let completed = ref false in
  App.run (fun env app ->
    let dismissed = ref None in
    let window =
      App.open_window
        app
        ~title:"GPUIO notifications"
        ~width:640.
        ~height:360.
        (component ~self_test ~dismissed)
      |> Or_error.ok_exn
    in
    if self_test
    then (
      let clock = Eio.Stdenv.clock env in
      Gpuio_eio.Scope.start
        (App.scope app)
        ~f:(fun () ->
          Eio.Time.with_timeout_exn clock 15. (fun () ->
            while Option.is_none !dismissed do
              Eio.Time.sleep clock 0.005
            done;
            assert (Option.equal Toast.Dismissal.equal !dismissed (Some Timeout));
            let promise, resolver = Eio.Promise.create () in
            App.Window.request_frame window ~on_rendered:(fun ~revision ->
              E.of_thunk (fun () -> Eio.Promise.resolve resolver revision))
            |> Or_error.ok_exn;
            ignore (Eio.Promise.await promise : int64)))
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
      "GPUIO_TOAST_PUBLIC_OK: native expiry crosses the bridge, Bonsai removes the keyed \
       notification, rendered acknowledgement and shutdown")
;;
