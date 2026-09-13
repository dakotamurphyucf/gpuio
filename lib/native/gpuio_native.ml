type file_descr = Unix.file_descr

open Core
module Wire = Gpuio_protocol.Wire

type t = int

external create : file_descr -> t = "gpuio_v1_create"
external create_options : file_descr -> bool -> t = "gpuio_v1_create_with_options"

let create_with_options ~exit_on_last_window fd = create_options fd exit_on_last_window

external run : t -> unit = "gpuio_v1_run"
external submit_bytes : t -> string -> int = "gpuio_v1_submit"
external drain_bytes : t -> string = "gpuio_v1_drain"
external dispose : t -> unit = "gpuio_v1_dispose"
external abort : t -> unit = "gpuio_v1_abort"

let submit t message =
  match Wire.Message.encode message with
  | Error _ -> Error Wire.Error_code.Limit_exceeded
  | Ok bytes ->
    (match submit_bytes t bytes with
     | 0 -> Ok ()
     | 1 -> Error Unsupported_version
     | 2 -> Error Unsupported_capability
     | 3 -> Error Malformed
     | 4 -> Error Limit_exceeded
     | 5 -> Error Not_ready
     | 6 -> Error Stale_handle
     | 7 -> Error Invalid_revision
     | 8 -> Error Invalid_tree
     | 9 -> Error Busy
     | 10 -> Error Closed
     | 11 -> Error Overloaded
     | 12 -> Error Native_failure
     | _ -> failwith "unknown native status")
;;

let drain t = Wire.Event.decode (drain_bytes t)
