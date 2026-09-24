open Core
module Wire = Gpuio_protocol.Wire.Pointer
module Button = Wire.Button
module Cancel_reason = Wire.Cancel_reason
module Phase = Wire.Phase
module Modifiers = Wire.Modifiers

module Position = struct
  type t =
    { x : float
    ; y : float
    }
  [@@deriving equal, sexp_of]
end

module Gesture_id = struct
  type t = int64 [@@deriving equal, compare, sexp_of]
end

module Event = struct
  type t =
    { gesture : Gesture_id.t
    ; phase : Phase.t
    ; button : Button.t
    ; window_position : Position.t
    ; local_position : Position.t
    ; modifiers : Modifiers.t
    }
  [@@deriving equal, sexp_of]
end

module Config = struct
  type t = Wire.Config.t [@@deriving equal, sexp_of]

  let create
        ~label
        ?(button = Button.Left)
        ?(disabled = false)
        ?(prevent_default = true)
        ?(stop_propagation = true)
        ()
    =
    if
      String.length label > 4096
      || String.is_empty (String.strip label)
      || (not (Stdlib.String.is_valid_utf_8 label))
      || String.contains label '\000'
    then
      Or_error.error_string
        "pointer region label must be bounded nonblank UTF-8 without NUL"
    else Ok Wire.Config.{ label; button; disabled; prevent_default; stop_propagation }
  ;;
end

module Expert = struct
  let to_wire (t : Config.t) = t

  let event_of_wire (sample : Wire.Sample.t) =
    if
      Int64.(sample.gesture <= 0L)
      || not
           (List.for_all
              [ sample.window_x; sample.window_y; sample.local_x; sample.local_y ]
              ~f:Float.is_finite)
    then Or_error.error_string "invalid native pointer sample"
    else
      Ok
        Event.
          { gesture = sample.gesture
          ; phase = sample.phase
          ; button = sample.button
          ; window_position = { x = sample.window_x; y = sample.window_y }
          ; local_position = { x = sample.local_x; y = sample.local_y }
          ; modifiers = sample.modifiers
          }
  ;;
end
