open Core
module Wire = Gpuio_protocol.Document_wire.Activation

module Source = struct
  type t =
    | Mouse of Pointer.Button.t
    | Keyboard
    | Touch of { long_press : bool }
  [@@deriving equal, sexp_of]
end

type t =
  { source : Source.t
  ; modifiers : Pointer.Modifiers.t
  }
[@@deriving equal, sexp_of]

module Expert = struct
  let of_wire (value : Wire.t) =
    if not (Wire.valid value)
    then Or_error.error_string "invalid document activation modifiers"
    else (
      let source : Source.t =
        match value.source with
        | Keyboard -> Keyboard
        | Touch { long_press } -> Touch { long_press }
        | Mouse button ->
          Mouse
            (match button with
             | Left -> Left
             | Right -> Right
             | Middle -> Middle
             | Back -> Back
             | Forward -> Forward)
      in
      let { Gpuio_protocol.Pointer_wire.Modifiers.shift
          ; control
          ; alt
          ; command
          ; function_
          }
        =
        value.modifiers
      in
      Ok { source; modifiers = { shift; control; alt; command; function_ } })
  ;;
end
