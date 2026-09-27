open Core
module App = Gpuio_eio.App
module Desktop = Gpuio_eio.Desktop
module Scope = Gpuio_eio.Scope
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View

let desktop_ok = function
  | Ok value -> value
  | Error error -> raise_s [%sexp (error : Desktop.Error.t)]
;;

let () =
  let self_test = Array.exists (Sys.get_argv ()) ~f:(String.equal "--self-test") in
  let scheme = Gpuio.Deep_link.Scheme.of_string "gpuio-desktop-lab" |> Or_error.ok_exn in
  let identity =
    Desktop.Identity.create
      ~identifier:"com.gpuio.desktop-lab"
      ~name:"GPUIO Desktop Lab"
      ~schemes:[ scheme ]
      ()
    |> Or_error.ok_exn
  in
  App.run ~desktop:identity ~exit_on_last_window:false (fun env app ->
    let state = B.Expert.Var.create "Waiting for an application link…" in
    let current_window = ref None in
    let opened = ref 0 in
    let receiver = ref None in
    let scope = App.scope app in
    let emit value = Eio.traceln "DESKTOP_LAB: %s" value in
    let component _window _graph =
      let open B.Let_syntax in
      let%arr status = B.Expert.Var.value state in
      let color = Gpuio.Color.rgb_exn in
      let px = Gpuio.Length.px_exn in
      let style = Gpuio.Style.create_exn in
      let text size tint value =
        V.text ~style:(style [ Font_size size; Foreground (color tint) ]) value
      in
      V.column
        ~style:
          (style
             [ Width (Gpuio.Length.percent_exn 100.)
             ; Height (Gpuio.Length.percent_exn 100.)
             ; Padding (px 28.)
             ; Gap (px 20.)
             ; Background (Gpuio.Background.solid (color 0x101722))
             ])
        [ text 12. 0x65dec1 "GPUIO / DESKTOP LAB"
        ; text 30. 0xf1f5fb "One application. Any entry point."
        ; text 16. 0xa8b8ce status
        ; text 13. 0xa8b8ce "Links wait for readiness, then route through OCaml."
        ; V.button "Quit lab" ~on_click:(E.of_thunk (fun () -> App.shutdown app))
        ]
    in
    let ensure_window () =
      match !current_window with
      | Some window when not (App.Window.is_closed window) -> window
      | None | Some _ ->
        let window =
          App.open_window
            app
            ~focus:false
            ~title:"GPUIO · Desktop Lab"
            ~width:680.
            ~height:340.
            component
          |> Or_error.ok_exn
        in
        current_window := Some window;
        incr opened;
        emit (sprintf "window-opened %d" !opened);
        window
    in
    let rec attach () = Desktop.attach app ~on_event |> desktop_ok
    and on_event event =
      E.of_thunk (fun () ->
        match event with
        | Desktop.Event.Link link ->
          emit ("link " ^ Gpuio.Deep_link.to_string link);
          (match Gpuio.Deep_link.route link with
           | "document" ->
             ignore (ensure_window () : App.Window.t);
             B.Expert.Var.set state ("Document " ^ Gpuio.Deep_link.path link)
           | "close" ->
             Option.iter !current_window ~f:App.Window.close;
             emit "window-close-requested"
           | "replace" ->
             let old = Option.value_exn !receiver in
             Desktop.close old;
             let next = attach () in
             receiver := Some next;
             Desktop.ready next;
             Desktop.close old;
             emit "receiver-replaced"
           | "quit" -> App.shutdown app
           | _ -> emit "unknown-route")
        | Rejected_link { input; reason } ->
          emit
            (sprintf
               "rejected %s %s"
               input
               (Sexp.to_string ([%sexp_of: Gpuio.Deep_link.Error.t] reason)))
        | Overflow count -> emit (sprintf "overflow %Ld" count)
        | Failed error -> raise_s [%sexp (error : Desktop.Error.t)])
    in
    let subscription = attach () in
    receiver := Some subscription;
    (match Desktop.attach app ~on_event with
     | Error Busy -> emit "exclusive-receiver"
     | Error error -> raise_s [%sexp (error : Desktop.Error.t)]
     | Ok _ -> failwith "duplicate desktop receiver admitted");
    ignore (ensure_window () : App.Window.t);
    emit "waiting";
    Scope.start
      scope
      ~f:(fun () -> Eio.Time.sleep (Eio.Stdenv.clock env) (if self_test then 1.5 else 0.))
      ~on_result:(fun result ->
        E.of_thunk (fun () ->
          Or_error.ok_exn result;
          emit "ready";
          Desktop.ready subscription))
    |> Or_error.ok_exn
    |> fun (_ : Scope.Task.t) ->
    if self_test
    then
      Scope.start
        scope
        ~f:(fun () -> Eio.Time.sleep (Eio.Stdenv.clock env) 35.)
        ~on_result:(fun result ->
          E.of_thunk (fun () ->
            Or_error.ok_exn result;
            emit "deadline";
            App.shutdown app))
      |> Or_error.ok_exn
      |> fun (_ : Scope.Task.t) -> ())
;;
