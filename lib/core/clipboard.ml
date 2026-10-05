open Core

module Text = struct
  type t = string [@@deriving equal, sexp_of]

  let max_bytes = Gpuio_protocol.Desktop_wire.max_clipboard_text_bytes

  let of_string value =
    if Gpuio_protocol.Desktop_wire.valid_clipboard_text value
    then Ok value
    else
      Or_error.error_string
        "clipboard text must be valid UTF-8 without NUL and at most 256 KiB"
  ;;

  let to_string t = t
end

module Error = struct
  type t =
    | Invalid_request
    | Not_ready
    | Unsupported
    | Unavailable
    | Denied
    | Busy
    | Closed
    | Native_failure
  [@@deriving equal, sexp_of]
end
