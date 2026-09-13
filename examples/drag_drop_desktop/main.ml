open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module View = Gpuio_bonsai.View
module App = Gpuio_eio.App
module Drag = Gpuio.Drag_and_drop

module Scenario = struct
  type t =
    | Drop
    | Reenter
    | Cancel
    | Remove_source
  [@@deriving equal]
end

let region color =
  Gpuio.Style.create_exn
    [ Width (Gpuio.Length.px_exn 280.)
    ; Height (Gpuio.Length.px_exn 120.)
    ; Background (Gpuio.Background.solid (Gpuio.Color.rgb_exn color))
    ; Foreground (Gpuio.Color.rgb_exn 0xffffff)
    ; Padding (Gpuio.Length.px_exn 16.)
    ]
;;

let column children =
  View.column
    ~style:(Gpuio.Style.create_exn [ Padding (Gpuio.Length.px_exn 20.) ])
    children
;;

let source_component
      config
      observe
      reentry_config
      observe_target
      rendered
      ~remove_after_offer
      ~present_observed
      _window
      graph
  =
  let status, set_status = B.state "Drag the file to the other window" graph in
  let present, set_present = B.state true graph in
  B.Edge.on_change
    present
    ~equal:Bool.equal
    ~callback:(B.return (fun value -> E.of_thunk (fun () -> present_observed := value)))
    graph;
  let received, set_received = B.state false graph in
  B.Edge.on_change
    received
    ~equal:Bool.equal
    ~callback:(B.return (fun value -> E.of_thunk (fun () -> rendered := value)))
    graph;
  let open B.Let_syntax in
  let%arr status = status
  and set_status = set_status
  and received = received
  and set_received = set_received
  and present = present
  and set_present = set_present in
  column
    [ View.drop_target
        ~config:reentry_config
        ~on_event:(fun (event : Drag.Target_event.t) ->
          E.Many
            [ E.of_thunk (fun () -> observe_target event)
            ; (match event.phase with
               | Dropped _ -> set_received true
               | Entered _ | Moved | Left | Rejected _ -> E.Ignore)
            ])
        (if not present
         then [ View.text "Source removed during OS drag" ]
         else
           [ View.drag_source
               ~config
               ~style:(region (if received then 0x337744 else 0x3c5e7e))
               ~on_event:(fun (event : Drag.Source_event.t) ->
                 E.Many
                   [ E.of_thunk (fun () -> observe event)
                   ; (match event.phase with
                      | Desktop_offered when remove_after_offer -> set_present false
                      | Started _ | Desktop_offered | Desktop_unavailable | Ended _ ->
                        E.Ignore)
                   ; set_status
                       (Sexp.to_string_hum (Drag.Source_phase.sexp_of_t event.phase))
                   ])
               [ View.text "File source" ]
           ])
    ; View.text status
    ]
;;

let target_component config observe rendered _window graph =
  let received, set_received = B.state false graph in
  B.Edge.on_change
    received
    ~equal:Bool.equal
    ~callback:(B.return (fun value -> E.of_thunk (fun () -> rendered := value)))
    graph;
  let open B.Let_syntax in
  let%arr received = received
  and set_received = set_received in
  column
    [ View.drop_target
        ~config
        ~style:(region (if received then 0x337744 else 0x444b55))
        ~on_event:(fun (event : Drag.Target_event.t) ->
          E.Many
            [ E.of_thunk (fun () -> observe event)
            ; (match event.phase with
               | Dropped _ -> set_received true
               | Entered _ | Moved | Left | Rejected _ -> E.Ignore)
            ])
        [ View.text "File receiver" ]
    ; View.text (if received then "Received file path; no file opened" else "Drop here")
    ]
;;

let () =
  let args = Sys.get_argv () in
  let scenario =
    match Array.to_list args with
    | [ _; _ ] -> Scenario.Drop
    | [ _; _; "--reenter" ] -> Reenter
    | [ _; _; "--cancel" ] -> Cancel
    | [ _; _; "--remove-source" ] -> Remove_source
    | _ ->
      failwith
        "usage: main.exe ABSOLUTE_EXISTING_FILE [--reenter|--cancel|--remove-source]"
  in
  let reenter = Scenario.equal scenario Reenter in
  let cancel = Scenario.equal scenario Cancel in
  let remove_source = Scenario.equal scenario Remove_source in
  (* The test harness creates this regular file. There is no synchronous file
     lookup in a UI constructor, nor any implicit file read after dropping. *)
  let path = Gpuio.File_path.of_string args.(1) |> Or_error.ok_exn in
  let payload =
    Drag.Payload.files [ Drag.File.create ~is_directory:false path ] |> Or_error.ok_exn
  in
  let source_config =
    Drag.Source.create ~label:"File source" ~payload ~allow_desktop_files:true ()
    |> Or_error.ok_exn
  in
  let target_config =
    Drag.Target.create ~label:"File receiver" ~accept:[ Files ] () |> Or_error.ok_exn
  in
  let reentry_config =
    Drag.Target.create ~label:"Reentry receiver" ~accept:[ Files ] () |> Or_error.ok_exn
  in
  let sources = ref [] in
  let targets = ref [] in
  let rendered = ref false in
  let completed = ref false in
  let present = ref true in
  let observe_source (event : Drag.Source_event.t) =
    sources := event :: !sources;
    Eio.traceln
      "DESKTOP_SOURCE: %s"
      (Sexp.to_string_hum (Drag.Source_phase.sexp_of_t event.phase))
  in
  let observe_target (event : Drag.Target_event.t) =
    match event.phase with
    | Moved -> ()
    | Entered _ | Left | Dropped _ | Rejected _ ->
      targets := event :: !targets;
      Eio.traceln
        "DESKTOP_TARGET: %s"
        (Sexp.to_string_hum (Drag.Target_phase.sexp_of_t event.phase))
  in
  App.run (fun env app ->
    let source =
      App.open_window
        app
        ~title:"GPUIO file source"
        ~width:320.
        ~height:280.
        (source_component
           source_config
           observe_source
           reentry_config
           observe_target
           rendered
           ~remove_after_offer:remove_source
           ~present_observed:present)
      |> Or_error.ok_exn
    in
    let target =
      App.open_window
        app
        ~title:"GPUIO file receiver"
        ~width:320.
        ~height:280.
        (target_component target_config observe_target rendered)
      |> Or_error.ok_exn
    in
    let clock = Eio.Stdenv.clock env in
    Gpuio_eio.Scope.start
      (App.scope app)
      ~f:(fun () ->
        Eio.Time.with_timeout_exn clock 30. (fun () ->
          while
            not
              ((cancel || !rendered)
               &&
               if remove_source
               then not !present
               else
                 List.exists !sources ~f:(fun (e : Drag.Source_event.t) ->
                   match e.phase with
                   | Ended _ -> true
                   | Started _ | Desktop_offered | Desktop_unavailable -> false))
          do
            Eio.Time.sleep clock 0.005
          done;
          let starts =
            List.filter_map !sources ~f:(fun (e : Drag.Source_event.t) ->
              match e.phase with
              | Started p -> Some (e.gesture, p)
              | Ended _ | Desktop_offered | Desktop_unavailable -> None)
          in
          let offers =
            List.filter_map !sources ~f:(fun (e : Drag.Source_event.t) ->
              match e.phase with
              | Desktop_offered -> Some e.gesture
              | Started _ | Ended _ | Desktop_unavailable -> None)
          in
          let ends =
            List.filter_map !sources ~f:(fun (e : Drag.Source_event.t) ->
              match e.phase with
              | Ended outcome -> Some (e.gesture, outcome)
              | Started _ | Desktop_offered | Desktop_unavailable -> None)
          in
          (match starts, offers, ends with
           | [ (started, p) ], [ offered ], [] when remove_source ->
             assert (Drag.Gesture_id.equal started offered);
             assert (Drag.Payload.equal p payload);
             assert (not !present)
           | [ (started, p) ], [ offered ], [ (ended, outcome) ] when not remove_source ->
             assert (
               Drag.Outcome.equal outcome (if reenter then Internal_drop else Unconfirmed));
             assert (
               Drag.Gesture_id.equal started offered
               && Drag.Gesture_id.equal started ended);
             assert (Drag.Payload.equal p payload)
           | _ -> failwith "expected one source start, OS offer and unconfirmed end");
          let drops =
            List.filter_map !targets ~f:(fun (e : Drag.Target_event.t) ->
              match e.phase with
              | Dropped p -> Some (e.gesture, p)
              | Entered _ | Moved | Left | Rejected _ -> None)
          in
          assert (
            List.exists !targets ~f:(fun (e : Drag.Target_event.t) ->
              match e.phase with
              | Entered offer -> Drag.Origin.equal offer.origin Desktop
              | Moved | Left | Dropped _ | Rejected _ -> false));
          (match drops with
           | [] when cancel -> ()
           | [ (gesture, Files [ file ]) ] when not cancel ->
             assert (Gpuio.File_path.equal file.path path);
             if reenter
             then (
               assert (Option.value_map file.is_directory ~default:false ~f:not);
               match starts with
               | [ (started, _) ] -> assert (Drag.Gesture_id.equal gesture started)
               | _ -> assert false)
             else (
               assert (Option.is_none file.is_directory);
               match starts with
               | [ (started, _) ] -> assert (not (Drag.Gesture_id.equal gesture started))
               | _ -> assert false);
             assert (
               List.exists !targets ~f:(fun (e : Drag.Target_event.t) ->
                 match e.phase with
                 | Entered offer ->
                   Drag.Gesture_id.equal e.gesture gesture
                   && Drag.Origin.equal
                        offer.origin
                        (if reenter then Internal else Desktop)
                 | Moved | Left | Dropped _ | Rejected _ -> false))
           | _ -> failwith "expected one OS-mediated file drop with unknown metadata");
          let promise, resolver = Eio.Promise.create () in
          App.Window.request_frame
            (if reenter then source else target)
            ~on_rendered:(fun ~revision:_ ->
              E.of_thunk (fun () -> Eio.Promise.resolve resolver ()))
          |> Or_error.ok_exn;
          Eio.Promise.await promise;
          assert (List.length !sources = if remove_source then 2 else 3)))
      ~on_result:(fun result ->
        E.of_thunk (fun () ->
          Or_error.ok_exn result;
          completed := true;
          App.Window.close source;
          App.Window.close target))
    |> Or_error.ok_exn
    |> fun (_ : Gpuio_eio.Scope.Task.t) -> ());
  assert !completed;
  Eio.traceln
    "%s"
    (match scenario with
     | Drop ->
       "GPUIO_DRAG_DROP_DESKTOP_OK: OS file offer, second-window desktop drop, exact \
        path, rendered callback and clean shutdown"
     | Reenter ->
       "GPUIO_DRAG_DROP_REENTRY_OK: OS offer and source-window reentry restore identity \
        and metadata"
     | Cancel -> "GPUIO_DRAG_DROP_CANCEL_OK: OS Escape ends unconfirmed without a drop"
     | Remove_source ->
       "GPUIO_DRAG_DROP_REMOVAL_OK: source unmount suppresses late callbacks; immutable \
        OS offer remains receivable")
;;
