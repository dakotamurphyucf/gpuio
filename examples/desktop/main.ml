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

let scheme = Gpuio.Deep_link.Scheme.of_string "gpuio-desktop-lab" |> Or_error.ok_exn

let unpackaged =
  Gpuio.Deep_link.Scheme.of_string "gpuio-desktop-unpackaged" |> Or_error.ok_exn
;;

let identity ~self_test =
  Desktop.Identity.create
    ~identifier:"com.gpuio.desktop-lab"
    ~name:"GPUIO Desktop Lab"
    ~schemes:(if self_test then [ scheme; unpackaged ] else [ scheme ])
    ()
  |> Or_error.ok_exn
;;

let main () =
  let self_test = Array.exists (Sys.get_argv ()) ~f:(String.equal "--self-test") in
  let service_path =
    Array.find_map (Sys.get_argv ()) ~f:(String.chop_prefix ~prefix:"--service-path=")
  in
  let identity = identity ~self_test in
  let startup_links =
    Sys.get_argv ()
    |> Array.to_list
    |> List.tl_exn
    |> List.drop_while ~f:(fun arg -> not (String.equal arg "--open-uris"))
    |> function
    | [] -> []
    | _marker :: links -> links
  in
  App.run_desktop identity ~startup_links ~exit_on_last_window:false (fun env app ->
    let state = B.Expert.Var.create "Waiting for an application link…" in
    let current_window = ref None in
    let retired_window = ref None in
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
    let ensure_window ?(focus = false) () =
      match !current_window with
      | Some window when not (App.Window.is_closed window) -> window
      | None | Some _ ->
        let window =
          App.open_window
            app
            ~focus
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
    App.on_reopen app (fun () ->
      let window = ensure_window ~focus:true () in
      match App.Window.snapshot window with
      | None -> E.of_thunk (fun () -> emit "reopened")
      | Some _ ->
        E.map (App.Window.command window Activate) ~f:(fun result ->
          match result with
          | Ok _ -> emit "reopened"
          | Error error -> raise_s [%sexp (error : Gpuio.Window.Error.t)]));
    let rec attach () = Desktop.attach app ~on_event |> desktop_ok
    and on_event event =
      match event with
      | Desktop.Event.Link link when String.equal (Gpuio.Deep_link.route link) "services"
        ->
        let path () =
          Option.value_exn service_path |> Gpuio.File_path.of_string |> Or_error.ok_exn
        in
        let action = Gpuio.Deep_link.path link in
        let operation =
          match action with
          | "/activate" -> Desktop.activate app ~ignoring_other_apps:true ()
          | "/open" -> Desktop.open_file app (path ())
          | "/reveal" -> Desktop.reveal_file app (path ())
          | "/missing" ->
            Desktop.open_file
              app
              (Gpuio.File_path.of_string (Gpuio.File_path.to_string (path ()) ^ ".missing")
               |> Or_error.ok_exn)
          | "/register" -> Desktop.register_scheme app scheme
          | "/unpackaged" -> Desktop.register_scheme app unpackaged
          | "/undeclared" ->
            Desktop.register_scheme
              app
              (Gpuio.Deep_link.Scheme.of_string "gpuio-undeclared-test" |> Or_error.ok_exn)
          | _ -> E.return (Error Desktop.Error.Invalid_request)
        in
        E.map operation ~f:(fun result ->
          emit
            ("service "
             ^ action
             ^ " "
             ^ Sexp.to_string ([%sexp_of: (unit, Desktop.Error.t) Result.t] result)))
      | Desktop.Event.Link link when String.equal (Gpuio.Deep_link.route link) "metadata"
        ->
        let stale = String.equal (Gpuio.Deep_link.path link) "/stale" in
        let clear = String.equal (Gpuio.Deep_link.path link) "/clear" in
        let window =
          if stale then Option.value_exn !retired_window else ensure_window ()
        in
        let path = Gpuio.File_path.of_string "/tmp/GPUIO résumé.txt" |> Or_error.ok_exn in
        let document =
          Desktop.Document.create
            ?path:(if clear then None else Some path)
            ~edited:(not clear)
            ()
        in
        E.map (Desktop.set_document window document) ~f:(fun result ->
          if stale
          then (
            match result with
            | Error Closed -> emit "metadata-stale-closed"
            | Error error -> raise_s [%sexp (error : Gpuio.Window.Error.t)]
            | Ok _ -> failwith "retired window accepted document metadata")
          else (
            let snapshot =
              match result with
              | Ok snapshot -> snapshot
              | Error error -> raise_s [%sexp (error : Gpuio.Window.Error.t)]
            in
            assert (
              Option.equal
                Desktop.Document.equal
                (Desktop.Document.of_snapshot snapshot)
                (Some document));
            emit (if clear then "metadata-cleared" else "metadata-edited")))
      | _ ->
        E.of_thunk (fun () ->
          match event with
          | Desktop.Event.Link link ->
            emit ("link " ^ Gpuio.Deep_link.to_string link);
            (match Gpuio.Deep_link.route link with
             | "document" ->
               ignore (ensure_window () : App.Window.t);
               B.Expert.Var.set state ("Document " ^ Gpuio.Deep_link.path link)
             | "close" ->
               retired_window := !current_window;
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
  |> desktop_ok
  |> fun outcome ->
  match outcome with
  | App.Launch_outcome.Exited -> ()
  | Forwarded -> Eio.traceln "DESKTOP_LAB: forwarded"
;;

let check_invalid_launch () =
  let identity =
    Desktop.Identity.create ~identifier:"com.gpuio.desktop-lab" ~name:"Desktop Lab" ()
    |> Or_error.ok_exn
  in
  (* More than the native handle limit proves failed preflights dispose handles. *)
  for _ = 1 to 12 do
    let result =
      App.run_desktop identity ~startup_links:[ "invalid\000input" ] (fun _ _ ->
        failwith "invalid launch initialized the application")
    in
    match result with
    | Error Desktop.Error.Invalid_request -> ()
    | Ok _ | Error _ -> failwith "invalid launch did not return Invalid_request"
  done;
  Eio.traceln "DESKTOP_LAB: invalid-launch-cleanup"
;;

let print_package package =
  let contents = package |> Or_error.ok_exn |> Gpuio.Desktop_package.contents in
  Eio_main.run (fun env -> Eio.Flow.copy_string contents (Eio.Stdenv.stdout env))
;;

let () =
  match Array.to_list (Sys.get_argv ()) with
  | [ _; "--print-info-plist" ] ->
    print_package
      (Gpuio.Desktop_package.macos_info_plist
         (identity ~self_test:false)
         ~executable:"gpuio-desktop"
         ~version:"0.1.0"
         ~build:"1")
  | [ _; "--print-desktop-entry"; executable ] ->
    let executable = Gpuio.File_path.of_string executable |> Or_error.ok_exn in
    print_package
      (Gpuio.Desktop_package.linux_entry (identity ~self_test:false) ~executable ())
  | [ _; "--print-desktop-entry"; executable; "--self-test" ] ->
    let executable = Gpuio.File_path.of_string executable |> Or_error.ok_exn in
    print_package
      (Gpuio.Desktop_package.linux_entry
         (identity ~self_test:false)
         ~executable
         ~arguments:[ "--self-test" ]
         ())
  | [ _; "--check-invalid-launch" ] -> check_invalid_launch ()
  | _ -> main ()
;;
