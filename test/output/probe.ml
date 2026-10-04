module Posix = Unix
open Core

let () =
  Eio_main.run (fun env ->
    let sink = Eio.Stdenv.stdout env in
    let descriptor = Eio_unix.Resource.fd sink in
    let blocking = Eio_unix.Fd.is_blocking descriptor in
    let text =
      String.init (2 * 1024 * 1024) ~f:(fun index -> Char.of_int_exn (65 + (index % 26)))
    in
    match Array.to_list (Sys.get_argv ()) with
    | [ _; "--raw" ] -> Eio.Flow.copy_string text sink
    | [ _; "--character-responsive" ] ->
      Eio.Fiber.both
        (fun () -> Gpuio_eio.Output.write sink text)
        (fun () ->
           Eio.Time.sleep (Eio.Stdenv.clock env) 0.05;
           Eio.traceln "OUTPUT_TIMER_FIRED")
    | [ _; "--read-only" ] ->
      (match Gpuio_eio.Output.write sink text with
       | () -> failwith "Read-only output unexpectedly succeeded"
       | exception Eio.Io (Eio.Exn.X (Eio_unix.Unix_error (Posix.EBADF, _, _)), _) -> ())
    | [ _; "--cancel" ] ->
      Eio.Switch.run (fun sw ->
        let _reader, writer = Eio_unix.pipe sw in
        match
          Eio.Time.with_timeout (Eio.Stdenv.clock env) 0.1 (fun () ->
            Gpuio_eio.Output.write writer text;
            Ok ())
        with
        | Error `Timeout -> ()
        | Ok () -> failwith "Expected nonblocking pipe backpressure cancellation")
    | [ _ ] ->
      Gpuio_eio.Output.write sink (String.prefix text 17);
      Gpuio_eio.Output.write sink (String.drop_prefix text 17);
      assert (Eio_unix.Fd.is_open descriptor);
      assert (Bool.equal blocking (Eio_unix.Fd.is_blocking descriptor))
    | _ -> failwith "Unknown probe argument")
;;
