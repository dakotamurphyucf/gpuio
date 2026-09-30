open Core

(* Fixed-size configuration for standalone and live operation decoding. *)
let max_text_bytes = 16384
let max_config_bytes = 32

module Spread = struct
  type t =
    | Relative of float
    | Pixels of float
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Relative value -> Float.is_finite value && Float.(value >= 0.05 && value <= 1.)
    | Pixels value -> Float.is_finite value && Float.(value >= 1. && value <= 1_000_000.)
  ;;
end

module Direction = struct
  type t =
    | Left_to_right
    | Right_to_left
  [@@deriving bin_io, equal, sexp_of]
end

module Repeat = struct
  type t =
    | Once
    | Loop
  [@@deriving bin_io, equal, sexp_of]
end

module Config = struct
  type t =
    { duration_ms : int
    ; spread : Spread.t
    ; direction : Direction.t
    ; repeat : Repeat.t
    ; animated : bool
    ; highlight : int64 option
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    t.duration_ms >= 1
    && t.duration_ms <= 60_000
    && Spread.valid t.spread
    && Option.for_all t.highlight ~f:(fun c -> Int64.(c >= 0L && c <= 0xffff_ffffL))
  ;;

  let decode bytes =
    if String.length bytes > max_config_bytes
    then Or_error.error_string "text shimmer config exceeds 32 bytes"
    else
      let open Or_error.Let_syntax in
      let buffer = Bigstring.of_string bytes in
      let pos_ref = ref 0 in
      let%bind t = Or_error.try_with (fun () -> bin_read_t buffer ~pos_ref) in
      if !pos_ref <> String.length bytes || not (valid t)
      then Or_error.error_string "invalid text shimmer config"
      else Ok t
  ;;
end
