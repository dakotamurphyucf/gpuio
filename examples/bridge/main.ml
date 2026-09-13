open Core
open Gpuio_protocol

let id slot = Node_id.create ~slot ~generation:1L |> Or_error.ok_exn
let window slot = Window_id.create ~slot ~generation:1L |> Or_error.ok_exn

let worker native notification_read =
  Eio_main.run (fun env ->
    let send message =
      Gpuio_native.submit native message
      |> Result.map_error ~f:(fun code -> Error.create_s (Wire.Error_code.sexp_of_t code))
      |> Or_error.ok_exn
    in
    let accepted = ref 0L
    and painted = ref 0L
    and stopped = ref false
    and other_closed = ref false in
    let rollback = ref false
    and containment = ref false in
    let apply window base operations =
      send (Apply { window; base; revision = Int64.succ base; operations })
    in
    let initial window =
      apply
        window
        0L
        [ Create (id 0L, Container, "", None)
        ; Create (id 1L, Text, "OCaml → bounded bin_prot → Rust → GPUI", None)
        ; Set_style
            ( id 0L
            , [ Padding 24.
              ; Gap 12.
              ; Background (Rgba 0x172136ffL)
              ; Foreground (Rgba 0xf1f5ffffL)
              ] )
        ; Splice (id 0L, 0L, 0L, [ id 1L ])
        ; Set_root (Some (id 0L))
        ]
    in
    let finish () = if Int64.equal !painted 50L && !other_closed then send Shutdown in
    let process = function
      | Wire.Event.Welcome _ ->
        (* This must raise without poisoning the registry or harming the live app. *)
        (match Result.try_with (fun () -> Gpuio_native.dispose native) with
         | Error (Failure _) -> containment := true
         | Error exn -> raise exn
         | Ok () -> failwith "disposed running runtime");
        send (Open (1L, window 0L, "GPUIO production bridge", 640., 480.))
      | Opened (_, w) ->
        if Window_id.equal w (window 0L)
        then send (Open (2L, window 1L, "Independent GPUIO window", 400., 240.))
        else (
          initial (window 0L);
          initial (window 1L))
      | Accepted (w, revision) ->
        if Window_id.equal w (window 0L)
        then (
          assert (Int64.equal revision (Int64.succ !accepted));
          accepted := revision);
        send (Request_frame (revision, w))
      | Rendered (w, revision) ->
        if Window_id.equal w (window 0L) then assert (Int64.(revision <= !accepted))
      | Frame_requested (correlation, w, revision) ->
        assert (Int64.equal correlation revision);
        if Window_id.equal w (window 1L)
        then send (Close (3L, w))
        else (
          assert (Int64.equal revision (Int64.succ !painted));
          painted := revision;
          if Int64.equal revision 1L
          then
            apply
              w
              revision
              [ Set_text (id 1L, "must roll back"); Splice (id 0L, 0L, 0L, [ id 0L ]) ]
          else if Int64.(revision < 50L)
          then
            apply
              w
              revision
              [ Set_text (id 1L, sprintf "Production revision %Ld" (Int64.succ revision))
              ]
          else finish ())
      | Rejected (w, revision, Invalid_tree) ->
        assert (Window_id.equal w (window 0L) && Int64.equal revision 2L);
        rollback := true;
        apply w 1L [ Set_text (id 1L, "Rollback preserved revision 1") ]
      | Closed (_, w) ->
        assert (Window_id.equal w (window 1L));
        other_closed := true;
        finish ()
      | Stopped ->
        assert (Int64.equal !painted 50L && !other_closed && !rollback && !containment);
        stopped := true
      | Failed _ | Rejected _ | Overloaded _ -> failwith "unexpected bridge failure"
      | Press _
      | Editor_event _
      | Editor_result _
      | Choice _
      | Combobox_selected _
      | Overlay_dismissed _
      | Tooltip_open_changed _
      | Palette_dismissed _
      | Command_invoked _ -> ()
    in
    send (Hello (Wire.version, Wire.capabilities));
    Eio.Time.with_timeout_exn (Eio.Stdenv.clock env) 30. (fun () ->
      let buffer = Cstruct.create 4096 in
      while not !stopped do
        List.iter (Gpuio_native.drain native |> Or_error.ok_exn) ~f:process;
        if not !stopped then ignore (Eio.Flow.single_read notification_read buffer : int)
      done);
    Eio.Flow.copy_string
      "PRODUCTION_BRIDGE_PASS commits=50 windows=2 rollback=true panic_contained=true\n"
      (Eio.Stdenv.stdout env))
;;

let () =
  Eio_main.run (fun _ ->
    Eio.Switch.run (fun sw ->
      let notification_read, notification_write = Eio_unix.pipe sw in
      let native =
        Eio_unix.Fd.use_exn
          "native bridge"
          (Eio_unix.Resource.fd notification_write)
          Gpuio_native.create
      in
      Eio.Flow.close notification_write;
      let domain =
        Domain.spawn (fun () ->
          try worker native notification_read with
          | exn ->
            let backtrace = Stdlib.Printexc.get_raw_backtrace () in
            Gpuio_native.abort native;
            Stdlib.Printexc.raise_with_backtrace exn backtrace)
      in
      let result = Result.try_with (fun () -> Gpuio_native.run native) in
      let worker_result = Result.try_with (fun () -> Domain.join domain) in
      Gpuio_native.dispose native;
      Result.ok_exn result;
      Result.ok_exn worker_result))
;;
