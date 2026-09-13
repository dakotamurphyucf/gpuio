open Core
open Gpuio
open Gpuio_protocol

module Action = struct
  type t =
    | Increment
    | Change_theme
end

let window = Window_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn

let worker native notification_read ~self_test =
  Eio_main.run (fun env ->
    let send message =
      Gpuio_native.submit native message
      |> Result.map_error ~f:(fun code -> Error.create_s (Wire.Error_code.sexp_of_t code))
      |> Or_error.ok_exn
    in
    let reconciler = Reconciler.create window in
    let value = ref 0
    and alternate = ref false
    and pending = ref None
    and dirty = ref true
    and opened = ref false
    and stopped = ref false in
    let theme () =
      if not !alternate
      then Theme.default
      else
        Theme.create
          [ "background", Color.rgb_exn 0xf4f6fb
          ; "foreground", Color.rgb_exn 0x182034
          ; "accent", Color.rgb_exn 0x3463ae
          ; "muted", Color.rgb_exn 0x8090ac
          ]
        |> Or_error.ok_exn
    in
    let pump () =
      if !opened && !dirty && Option.is_none !pending
      then (
        let view =
          Components.app
            ~value:!value
            ~on_increment:(fun () -> Action.Increment)
            ~on_theme:(fun () -> Action.Change_theme)
        in
        let update =
          Reconciler.prepare reconciler ~theme:(theme ()) (Some view) |> Or_error.ok_exn
        in
        dirty := false;
        match Reconciler.message update with
        | None -> Reconciler.accept reconciler update |> Or_error.ok_exn
        | Some message ->
          send message;
          pending := Some update)
    in
    let process = function
      | Wire.Event.Welcome _ ->
        send (Open (1L, window, "GPUIO · typed view API", 760., 520.))
      | Opened _ -> opened := true
      | Accepted (_, revision) ->
        let update = Option.value_exn !pending in
        Reconciler.accept reconciler update |> Or_error.ok_exn;
        pending := None;
        if self_test then send (Request_frame (revision, window))
      | ( Press _
        | Choice _
        | Combobox_selected _
        | Overlay_dismissed _
        | Tooltip_open_changed _
        | Pointer_event _
        | Toast_dismissed _
        | Palette_dismissed _
        | Command_invoked _ ) as event ->
        (match Reconciler.dispatch reconciler event with
         | Some Action.Increment ->
           incr value;
           dirty := true
         | Some Change_theme ->
           alternate := not !alternate;
           dirty := true
         | None -> ())
      | Frame_requested (_, _, revision) when self_test ->
        if Int64.(revision >= 20L)
        then send Shutdown
        else (
          incr value;
          if !value mod 4 = 0 then alternate := not !alternate;
          dirty := true)
      | Stopped ->
        Reconciler.close reconciler;
        stopped := true
      | Closed _ -> Reconciler.close reconciler
      | Rejected _ | Failed _ | Overloaded _ ->
        failwith "native view example rejected an update"
      | Rendered _ | Frame_requested _ | Editor_event _ | Editor_result _ -> ()
    in
    send (Hello (Wire.version, Wire.capabilities));
    let loop () =
      let buffer = Cstruct.create 4096 in
      while not !stopped do
        List.iter (Gpuio_native.drain native |> Or_error.ok_exn) ~f:process;
        pump ();
        if not !stopped then ignore (Eio.Flow.single_read notification_read buffer : int)
      done
    in
    if self_test
    then Eio.Time.with_timeout_exn (Eio.Stdenv.clock env) 30. loop
    else loop ();
    if self_test
    then
      Eio.Flow.copy_string
        "TYPED_VIEW_PASS revisions=20 themes=true native_selection=true\n"
        (Eio.Stdenv.stdout env))
;;

let () =
  let self_test = Array.exists (Sys.get_argv ()) ~f:(String.equal "--self-test") in
  Eio_main.run (fun _ ->
    Eio.Switch.run (fun sw ->
      let notification_read, notification_write = Eio_unix.pipe sw in
      let native =
        Eio_unix.Fd.use_exn
          "native view API"
          (Eio_unix.Resource.fd notification_write)
          Gpuio_native.create
      in
      Eio.Flow.close notification_write;
      let domain =
        Domain.spawn (fun () ->
          try worker native notification_read ~self_test with
          | exn ->
            let bt = Stdlib.Printexc.get_raw_backtrace () in
            Gpuio_native.abort native;
            Stdlib.Printexc.raise_with_backtrace exn bt)
      in
      let result = Result.try_with (fun () -> Gpuio_native.run native) in
      let worker_result = Result.try_with (fun () -> Domain.join domain) in
      Gpuio_native.dispose native;
      Result.ok_exn result;
      Result.ok_exn worker_result))
;;
