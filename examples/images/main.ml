open Core
module App = Gpuio_eio.App
module Scope = Gpuio_eio.Scope
module Asset = Gpuio_eio.Asset
module Image = Gpuio.Image
module View = Gpuio_bonsai.View
module B = Bonsai.Cont
module E = Bonsai.Effect

let component ~clicks ~icon ~asset ~phase ~observed ~status _window graph =
  let phase = B.Expert.Var.value phase in
  B.Edge.on_change
    phase
    ~equal:Int.equal
    ~callback:(B.return (fun phase -> E.of_thunk (fun () -> observed := phase)))
    graph;
  let open B.Let_syntax in
  let%arr asset = B.Expert.Var.value asset
  and count = B.Expert.Var.value clicks
  and phase = phase in
  match asset with
  | None -> View.text "Registering image…"
  | Some asset ->
    let description = Image.Description.label "Generated preview" |> Or_error.ok_exn in
    let fit = if phase = 0 then Image.Fit.Contain else Cover in
    let key = Gpuio.Key.of_string_exn (if phase < 2 then "preview" else "remounted") in
    let style =
      Gpuio.Style.create_exn
        [ Width (Gpuio.Length.px_exn 192.)
        ; Height (Gpuio.Length.px_exn 192.)
        ; Foreground
            (Gpuio.Color.rgba ~red:51 ~green:102 ~blue:204 ~alpha:255 |> Or_error.ok_exn)
        ]
    in
    let on_change state = E.of_thunk (fun () -> status := Some state) in
    let preview =
      if icon
      then
        View.icon
          ~key
          ~style
          ~on_change
          (Gpuio.Icon.Config.create ~asset ~description ~fit () |> Or_error.ok_exn)
      else
        View.image
          ~key
          ~style
          ~on_change
          (Image.Config.create ~asset ~description ~fit ())
    in
    let controls =
      if not icon
      then []
      else (
        let decoration = Gpuio.Icon.Decoration.create ~asset () |> Or_error.ok_exn in
        let invoke () = E.of_thunk (fun () -> B.Expert.Var.set clicks (count + 1)) in
        let command = Gpuio.Command.Id.of_string "send" |> Or_error.ok_exn in
        let commands =
          [ Gpuio.Command.create
              ~id:command
              ~label:("Run command (" ^ Int.to_string count ^ ")")
              ~on_invoke:invoke
              ()
            |> Or_error.ok_exn
          ]
          |> Gpuio.Command.Registry.create
          |> Or_error.ok_exn
        in
        [ View.row
            [ View.button
                ~leading_icon:decoration
                ?trailing_icon:(if phase = 0 then Some decoration else None)
                ~on_click:(invoke ())
                "Send"
            ; View.icon_button ~label:"Send icon" ~on_click:(invoke ()) decoration
            ]
        ; View.command_scope
            ~commands
            [ View.command_button ~command ~trailing_icon:decoration () ]
        ])
    in
    View.column
      ([ View.text "An encoded asset, rendered by the native image view"; preview ]
       @ controls)
;;

let () =
  let self_test = Array.exists (Sys.get_argv ()) ~f:(String.equal "--self-test") in
  let icon = Array.exists (Sys.get_argv ()) ~f:(String.equal "--icon") in
  let svg = icon || Array.exists (Sys.get_argv ()) ~f:(String.equal "--svg") in
  let completed = ref false in
  App.run (fun env app ->
    let scope = App.scope app in
    let asset = B.Expert.Var.create None in
    let clicks = B.Expert.Var.create 0 in
    let phase = B.Expert.Var.create 0 in
    let observed = ref (-1) in
    let status = ref None in
    let window =
      App.open_window
        app
        ~title:"GPUIO images"
        ~width:480.
        ~height:300.
        (component ~clicks ~icon ~asset ~phase ~observed ~status)
      |> Or_error.ok_exn
    in
    Scope.start
      scope
      ~f:(fun () ->
        let clock = Eio.Stdenv.clock env in
        Eio.Time.with_timeout_exn clock 20. (fun () ->
          let on_ui ui_effect =
            let promise, resolver = Eio.Promise.create () in
            Scope.Expert.enqueue scope (fun () ->
              E.Expert.handle (E.map ui_effect ~f:(Eio.Promise.resolve resolver)));
            Eio.Promise.await promise
          in
          let source =
            (if svg
             then
               Asset.Source.of_bytes
                 ~format:Svg
                 {|<svg xmlns="http://www.w3.org/2000/svg" width="4" height="4"><rect width="2" height="4" fill="red"/><rect x="2" width="2" height="4" fill="blue"/></svg>|}
             else
               Asset.Source.of_bytes
                 ~format:Pnm
                 ("P6\n4 4\n255\n"
                  ^ String.concat (List.init 16 ~f:(fun _ -> "\020\100\240"))))
            |> Or_error.ok_exn
          in
          let rec register () =
            match on_ui (Asset.register app ~scope source) with
            | Ok asset -> asset
            | Error Not_ready ->
              Eio.Time.sleep clock 0.005;
              register ()
            | Error error -> raise_s [%sexp (error : Asset.Error.t)]
          in
          let registered = register () in
          B.Expert.Var.set asset (Some (Asset.handle registered));
          let rec await_state predicate =
            match !status with
            | Some state when predicate state -> ()
            | Some (Failed error) -> raise_s [%sexp (error : Image.Error.t)]
            | Some Loading | Some (Ready _) | None ->
              Eio.Time.sleep clock 0.005;
              await_state predicate
          in
          await_state (function
            | Ready metadata ->
              Image.Metadata.width_px metadata = 4
              && Image.Metadata.height_px metadata = 4
            | Loading | Failed _ -> false);
          if self_test
          then (
            Asset.release registered;
            B.Expert.Var.set phase 1;
            while !observed <> 1 do
              Eio.Time.sleep clock 0.005
            done;
            let promise, resolver = Eio.Promise.create () in
            App.Window.request_frame window ~on_rendered:(fun ~revision:_ ->
              E.of_thunk (fun () -> Eio.Promise.resolve resolver ()))
            |> Or_error.ok_exn;
            Eio.Promise.await promise;
            assert (
              Option.exists !status ~f:(function
                | Ready _ -> true
                | Loading | Failed _ -> false));
            B.Expert.Var.set phase 2;
            await_state (function
              | Failed Released -> true
              | Loading | Ready _ | Failed _ -> false);
            completed := true)))
      ~on_result:(fun result ->
        E.of_thunk (fun () ->
          Or_error.ok_exn result;
          if self_test then App.Window.close window))
    |> Or_error.ok_exn
    |> fun (_ : Scope.Task.t) -> ());
  if self_test
  then (
    assert !completed;
    Eio.traceln
      "GPUIO_IMAGES_PUBLIC_OK (svg=%b icon=%b): scoped upload, Bonsai image, native \
       ready, restyle after retirement, remount rejection and shutdown"
      svg
      icon)
;;
