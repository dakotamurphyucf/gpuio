open Core

module Dismissal = struct
  type t =
    | Escape
    | Outside_pointer
  [@@deriving equal, sexp_of]
end

module Config = struct
  type t =
    { label : string
    ; placement : Placement.t
    ; width : float
    ; dismiss_on_escape : bool
    ; dismiss_on_outside_pointer : bool
    }
  [@@deriving equal, sexp_of]

  let create
        ~label
        ?(width = 480.)
        ?(placement = Placement.default)
        ?(dismiss_on_escape = true)
        ?(dismiss_on_outside_pointer = false)
        ()
    =
    if
      String.is_empty (String.strip label)
      || String.length label > 4096
      || (not (Stdlib.String.is_valid_utf_8 label))
      || String.contains label '\000'
    then Or_error.error_string "overlay label must be nonempty UTF-8, at most 4096 bytes"
    else if (not (Float.is_finite width)) || Float.(width < 1. || width > 16384.)
    then
      Or_error.error_string "overlay width must be finite and in 1..16384 logical pixels"
    else Ok { label; width; placement; dismiss_on_escape; dismiss_on_outside_pointer }
  ;;
end

module Expert = struct
  let placement (t : Config.t) = t.placement

  let to_wire (t : Config.t) ~kind : Gpuio_protocol.Wire.Overlay.t =
    { kind
    ; label = t.label
    ; width = t.width
    ; dismiss_on_escape = t.dismiss_on_escape
    ; dismiss_on_outside_pointer = t.dismiss_on_outside_pointer
    }
  ;;

  let allows (t : Config.t) = function
    | Dismissal.Escape -> t.dismiss_on_escape
    | Outside_pointer -> t.dismiss_on_outside_pointer
  ;;
end
