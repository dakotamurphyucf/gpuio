(* This is the narrow Unix-descriptor adapter for Eio's unsupported polling of
   blocking character-device output. Ordinary filesystem/network I/O stays Eio. *)
module Posix = Unix
open Core

type result =
  | Written
  | Use_flow

let rec single_write fd text offset =
  try Posix.single_write_substring fd text offset (String.length text - offset) with
  | Posix.Unix_error (EINTR, _, _) -> single_write fd text offset
;;

let rec write_device fd text offset =
  if offset < String.length text
  then (
    let written = single_write fd text offset in
    if written = 0 then raise (Posix.Unix_error (EIO, "write", "zero progress"));
    write_device fd text (offset + written))
;;

let write sink text =
  Eio.Fiber.check ();
  match Eio_unix.Resource.fd_opt sink with
  | None -> Eio.Flow.copy_string text sink
  | Some descriptor ->
    let result =
      Eio.Cancel.protect (fun () ->
        Eio_unix.run_in_systhread ~label:"character-device output" (fun () ->
          try
            Eio_unix.Fd.use_exn "output" descriptor (fun fd ->
              match (Posix.fstat fd).st_kind with
              | S_CHR when Eio_unix.Fd.is_blocking descriptor ->
                write_device fd text 0;
                Written
              | S_CHR | S_REG | S_DIR | S_BLK | S_LNK | S_FIFO | S_SOCK -> Use_flow)
          with
          | Posix.Unix_error (code, operation, argument) ->
            raise
              (Eio.Exn.create
                 (Eio.Exn.X (Eio_unix.Unix_error (code, operation, argument))))))
    in
    Eio.Fiber.check ();
    (match result with
     | Written -> ()
     | Use_flow -> Eio.Flow.copy_string text sink)
;;
