type file_descr = Unix.file_descr

open Core
module Wire = Gpuio_protocol.Wire

module Traffic = struct
  type t =
    { submission_attempts : int
    ; attempted_bytes : int
    ; submitted_messages : int
    ; submitted_bytes : int
    ; drain_calls : int
    ; drained_bytes : int
    ; received_events : int
    }
  [@@deriving sexp_of]
end

type t =
  { raw : int
  ; mutable traffic : Traffic.t
  }

let wrap raw =
  { raw
  ; traffic =
      { submission_attempts = 0
      ; attempted_bytes = 0
      ; submitted_messages = 0
      ; submitted_bytes = 0
      ; drain_calls = 0
      ; drained_bytes = 0
      ; received_events = 0
      }
  }
;;

let traffic t = t.traffic

external create_raw : file_descr -> int = "gpuio_v1_create"
external create_options : file_descr -> bool -> int = "gpuio_v1_create_with_options"

let initialization = lazy (Backend.initialize ())

let create fd =
  Lazy.force initialization;
  wrap (create_raw fd)
;;

let create_with_options ~exit_on_last_window fd =
  Lazy.force initialization;
  wrap (create_options fd exit_on_last_window)
;;

external run_raw : int -> unit = "gpuio_v1_run"
external submit_bytes : int -> string -> int = "gpuio_v1_submit"
external drain_bytes : int -> string = "gpuio_v1_drain"
external dispose_raw : int -> unit = "gpuio_v1_dispose"
external abort_raw : int -> unit = "gpuio_v1_abort"

let run t = run_raw t.raw
let dispose t = dispose_raw t.raw
let abort t = abort_raw t.raw

let submit t message =
  match Wire.Message.encode message with
  | Error _ -> Error Wire.Error_code.Limit_exceeded
  | Ok bytes ->
    t.traffic
    <- { t.traffic with
         submission_attempts = t.traffic.submission_attempts + 1
       ; attempted_bytes = t.traffic.attempted_bytes + String.length bytes
       };
    (match submit_bytes t.raw bytes with
     | 0 ->
       t.traffic
       <- { t.traffic with
            submitted_messages = t.traffic.submitted_messages + 1
          ; submitted_bytes = t.traffic.submitted_bytes + String.length bytes
          };
       Ok ()
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

let drain t =
  let bytes = drain_bytes t.raw in
  t.traffic
  <- { t.traffic with
       drain_calls = t.traffic.drain_calls + 1
     ; drained_bytes = t.traffic.drained_bytes + String.length bytes
     };
  let decoded = Wire.Event.decode bytes in
  Result.iter decoded ~f:(fun events ->
    t.traffic
    <- { t.traffic with received_events = t.traffic.received_events + List.length events });
  decoded
;;

external extension_catalog_bytes : unit -> string = "gpuio_v1_extension_catalog"

let extension_catalog () =
  Lazy.force initialization;
  Gpuio_protocol.Extension_wire.Catalog.decode (extension_catalog_bytes ())
;;

external prepare_desktop_bytes : int -> string -> string = "gpuio_v1_desktop_prepare"

let prepare_desktop t request =
  match Gpuio_protocol.Desktop_wire.Launch_request.encode request with
  | Error error -> Gpuio_protocol.Desktop_wire.Launch_response.Failed error
  | Ok bytes ->
    prepare_desktop_bytes t.raw bytes
    |> Gpuio_protocol.Desktop_wire.Launch_response.decode
;;
