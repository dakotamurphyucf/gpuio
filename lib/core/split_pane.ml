open Core
module Wire = Gpuio_protocol.Split_wire

module Axis = struct
  type t =
    | Horizontal
    | Vertical
  [@@deriving equal, sexp_of]
end

module Config = struct
  type t = Wire.Config.t [@@deriving equal, sexp_of]

  let create
        ~label
        ?(axis = Axis.Horizontal)
        ?(initial_first = 280.)
        ?(minimum_first = 80.)
        ?(maximum_first = 4096.)
        ?(minimum_second = 80.)
        ?(keyboard_step = 16.)
        ?(reset_generation = 0L)
        ()
    =
    let axis =
      match axis with
      | Axis.Horizontal -> Wire.Axis.Horizontal
      | Vertical -> Vertical
    in
    let t =
      { Wire.Config.label
      ; axis
      ; initial_first
      ; minimum_first
      ; maximum_first
      ; minimum_second
      ; keyboard_step
      ; reset_generation
      }
    in
    if Wire.Config.valid t
    then Ok t
    else Or_error.error_string "invalid split-pane label, range or generation"
  ;;
end

module Snapshot = struct
  type t = Wire.Snapshot.t =
    { first : float
    ; second : float
    }
  [@@deriving equal, sexp_of]
end

module Expert = struct
  let to_wire t = t
  let generation (t : Config.t) = t.reset_generation

  let snapshot_of_wire t =
    if Wire.Snapshot.valid t
    then Ok t
    else Or_error.error_string "invalid native split sizes"
  ;;
end
