open Core
module W = Gpuio_protocol.Loading_wire
module Kind = W.Kind

module Config = struct
  type t = W.Config.t [@@deriving equal, sexp_of]

  let create ~kind ~label ?(animated = true) ?(period = Time_ns.Span.of_ms 1200.) () =
    let period_ms = Time_ns.Span.to_ms period in
    if
      String.length label > 4096
      || String.is_empty (String.strip label)
      || (not (Stdlib.String.is_valid_utf_8 label))
      || String.contains label '\000'
    then Or_error.error_string "loading label must be bounded nonblank UTF-8 without NUL"
    else if Float.(period_ms < 100. || period_ms > 60000.)
    then Or_error.error_string "loading period must be between 100ms and 60s"
    else Ok { W.Config.kind; label; animated; period_ms = Float.iround_up_exn period_ms }
  ;;
end

module Expert = struct
  let to_wire t = t
end
