open Core

type t =
  { chunk_bytes : int
  ; interval_ms : int
  }
[@@deriving equal, sexp_of]

let default = { chunk_bytes = 17; interval_ms = 20 }
let chunk_bytes t = t.chunk_bytes
let interval_ms t = t.interval_ms

let with_chunk_bytes t chunk_bytes =
  if chunk_bytes < 4 || chunk_bytes > 128
  then Or_error.error_string "Chunk size must be between 4 and 128 bytes"
  else Ok { t with chunk_bytes }
;;

let with_interval_ms t interval_ms =
  if interval_ms < 10 || interval_ms > 200 || interval_ms mod 10 <> 0
  then Or_error.error_string "Stream interval must be 10–200 ms in 10 ms steps"
  else Ok { t with interval_ms }
;;

let backend t =
  Conversation.Backend.Config.create
    ~chunk_bytes:t.chunk_bytes
    ~delay_seconds:(Float.of_int t.interval_ms /. 1000.)
    ()
  |> Or_error.ok_exn
;;
