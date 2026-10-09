open Core
module App = Gpuio_eio.App
module N = Gpuio_eio.Notification
module Content = Gpuio.Notification
module Scope = Gpuio_eio.Scope
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View

let identity =
  Gpuio.Desktop.Identity.create
    ~identifier:"com.gpuio.notification-lab"
    ~name:"GPUIO Notification Lab"
    ()
  |> Or_error.ok_exn
;;

let tag = Content.Tag.of_string "build" |> Or_error.ok_exn

let action name label =
  Content.Action.create (Content.Action_id.of_string name |> Or_error.ok_exn) ~label
  |> Or_error.ok_exn
;;

let content ~updated ~index =
  Content.create
    ~title:
      (sprintf "GPUIO · Build %s #%d" (if updated then "updated" else "complete") index)
    ~body:
      (if updated
       then "The same notification now has a new action."
       else "Your native OCaml workspace is ready. 日本語 🦀")
    ~actions:
      [ (if updated
         then action "inspect" "Inspect build"
         else action "open" "Open workspace")
      ]
    ()
  |> Or_error.ok_exn
;;

let main () =
  let self_test = Array.exists (Sys.get_argv ()) ~f:(String.equal "--self-test") in
  let unbundled_check =
    Array.exists (Sys.get_argv ()) ~f:(String.equal "--unbundled-check")
  in
  App.run ~desktop:identity ~exit_on_last_window:false (fun env app ->
    let status = B.Expert.Var.create "Notifications are separate from in-app toasts." in
    let receipt = ref None in
    let posted_count = ref 0 in
    let current_window = ref None in
    let target_window = ref None in
    let emit value =
      Eio.traceln "NOTIFICATION_LAB: %s" value;
      B.Expert.Var.set status value
    in
    let service =
      N.attach app ~on_event:(fun event ->
        E.bind
          (E.of_thunk (fun () ->
             emit ("event " ^ Sexp.to_string_hum ([%sexp_of: N.Event.t] event));
             let delivered =
               match event with
               | Activated receipt | Action (receipt, _) | Closed (receipt, _) ->
                 Some receipt
               | Failed _ -> None
             in
             let matches =
               Option.is_some delivered
               && Option.equal Content.Receipt.equal !receipt delivered
             in
             let target = if matches then !target_window else None in
             if matches
             then (
               receipt := None;
               target_window := None);
             match event with
             | Activated _ | Action _ -> target
             | Closed _ | Failed _ -> None))
          ~f:(function
            | None -> E.Ignore
            | Some window ->
              E.map (App.Window.command window Activate) ~f:(fun result ->
                match result with
                | Ok _ -> emit "target-activated"
                | Error error ->
                  emit
                    ("target "
                     ^ Sexp.to_string_hum ([%sexp_of: Gpuio.Window.Error.t] error)))))
      |> function
      | Ok service -> service
      | Error error -> raise_s [%sexp (error : N.Error.t)]
    in
    let report label sexp result =
      emit
        (label ^ " " ^ Sexp.to_string_hum (Result.sexp_of_t sexp N.Error.sexp_of_t result))
    in
    let permission =
      E.map
        (N.request_authorization service)
        ~f:(report "permission" N.Authorization.sexp_of_t)
    in
    let probe =
      E.bind
        (E.map
           (N.capabilities service)
           ~f:(report "capabilities" N.Capabilities.sexp_of_t))
        ~f:(fun () ->
          E.map (N.authorization service) ~f:(fun result ->
            report "authorization" N.Authorization.sexp_of_t result;
            if unbundled_check
            then (
              assert (
                Result.equal
                  N.Authorization.equal
                  N.Error.equal
                  result
                  (Error Unavailable));
              emit "unbundled-check-passed";
              App.shutdown app)))
    in
    let post =
      E.bind
        (E.of_thunk (fun () -> !posted_count + 1))
        ~f:(fun index ->
          E.map
            (N.post service ~tag (content ~updated:false ~index))
            ~f:(fun result ->
              (match result with
               | Ok value ->
                 receipt := Some value;
                 target_window := !current_window;
                 posted_count := index
               | Error _ -> ());
              report "posted" Content.Receipt.sexp_of_t result))
    in
    let with_receipt operation =
      E.bind
        (E.of_thunk (fun () -> !receipt))
        ~f:(function
          | None -> E.of_thunk (fun () -> emit "no-live-receipt")
          | Some receipt -> operation receipt)
    in
    let replace =
      with_receipt (fun receipt ->
        E.map
          (N.replace service receipt (content ~updated:true ~index:!posted_count))
          ~f:(report "replaced" Unit.sexp_of_t))
    in
    let dismiss =
      with_receipt (fun current ->
        E.map (N.dismiss service current) ~f:(fun result ->
          (match result with
           | Ok () -> receipt := None
           | Error _ -> ());
          report "dismissed" Unit.sexp_of_t result))
    in
    let ready =
      E.of_thunk (fun () ->
        N.ready service;
        emit "ready")
    in
    let component _window _graph =
      let open B.Let_syntax in
      let%arr status = B.Expert.Var.value status in
      let style = Gpuio.Style.create_exn
      and color = Gpuio.Color.rgb_exn
      and px = Gpuio.Length.px_exn in
      let text size tint value =
        V.text ~style:(style [ Font_size size; Foreground (color tint) ]) value
      in
      V.column
        ~style:
          (style
             [ Width (Gpuio.Length.percent_exn 100.)
             ; Height (Gpuio.Length.percent_exn 100.)
             ; Padding (px 28.)
             ; Gap (px 18.)
             ; Background (Gpuio.Background.solid (color 0x101722))
             ])
        [ text 12. 0x65dec1 "GPUIO / NOTIFICATION LAB"
        ; text 29. 0xf1f5fb "Keep the conversation going."
        ; text 14. 0xa8b8ce "Native delivery. Explicit permission. OCaml-owned routing."
        ; V.row
            ~style:(style [ Gap (px 10.) ])
            [ V.button "Check support" ~on_click:probe
            ; V.button "Allow notifications" ~on_click:permission
            ; V.button "Ready for actions" ~on_click:ready
            ]
        ; V.row
            ~style:(style [ Gap (px 10.) ])
            [ V.button "Post notification" ~on_click:post
            ; V.button "Replace content" ~on_click:replace
            ; V.button "Dismiss notification" ~on_click:dismiss
            ]
        ; text 14. 0xcbd7e7 status
        ; V.button
            "Close notification service"
            ~on_click:
              (E.of_thunk (fun () ->
                 N.close service;
                 emit "service-closed"))
        ; V.button
            "Quit lab"
            ~on_click:
              (E.of_thunk (fun () ->
                 N.close service;
                 App.shutdown app))
        ]
    in
    let ensure_window () =
      match !current_window with
      | Some window when not (App.Window.is_closed window) -> ()
      | None | Some _ ->
        let window =
          App.open_window
            app
            ~focus:false
            ~title:"GPUIO · Notification Lab"
            ~width:780.
            ~height:490.
            component
          |> Or_error.ok_exn
        in
        current_window := Some window;
        emit "window-opened"
    in
    App.on_reopen app (fun () -> E.of_thunk ensure_window);
    ensure_window ();
    E.Expert.handle probe;
    emit "waiting";
    if not self_test then N.ready service;
    Scope.start
      (App.scope app)
      ~f:(fun () ->
        Eio.Time.sleep (Eio.Stdenv.clock env) (if self_test then 180. else 3600.))
      ~on_result:(fun _ ->
        E.of_thunk (fun () ->
          emit "deadline";
          App.shutdown app))
    |> Or_error.ok_exn
    |> fun (_ : Scope.Task.t) -> ())
;;

let () =
  match Array.to_list (Sys.get_argv ()) with
  | [ _; "--print-info-plist" ] ->
    let package =
      Gpuio.Desktop_package.macos_info_plist
        identity
        ~executable:"gpuio-notification"
        ~version:"0.1.0"
        ~build:"1"
      |> Or_error.ok_exn
    in
    Eio_main.run (fun env ->
      Gpuio_eio.Output.write
        (Eio.Stdenv.stdout env)
        (Gpuio.Desktop_package.contents package))
  | _ -> main ()
;;
