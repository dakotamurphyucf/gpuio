open Core

module Value = struct
  type t =
    | Indeterminate
    | Determinate of float
  [@@deriving equal, sexp_of]

  let indeterminate = Indeterminate

  let determinate ~fraction =
    if Float.is_finite fraction && Float.(fraction >= 0. && fraction <= 1.)
    then Ok (Determinate fraction)
    else Or_error.error_string "progress fraction must be finite and between 0 and 1"
  ;;
end

module Config = struct
  type t =
    { label : string
    ; value : Value.t
    }
  [@@deriving equal, sexp_of]

  let create ~label ~value =
    if
      String.length label > 4096
      || (not (Stdlib.String.is_valid_utf_8 label))
      || String.contains label '\000'
      || String.is_empty (String.strip label)
    then
      Or_error.error_string "progress label must be bounded, nonblank UTF-8 without NUL"
    else Ok { label; value }
  ;;
end

module Expert = struct
  let to_wire (t : Config.t) : Gpuio_protocol.Wire.Progress.t =
    { label = t.label
    ; fraction =
        (match t.value with
         | Indeterminate -> None
         | Determinate value -> Some value)
    }
  ;;
end
