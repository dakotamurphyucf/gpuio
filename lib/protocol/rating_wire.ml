open Core

let max_stars = 32

module Request = struct
  type t =
    | Set of int
    | Toggle of int
    | Increase
    | Decrease
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Set value -> value >= 0 && value <= max_stars
    | Toggle value -> value > 0 && value <= max_stars
    | Increase | Decrease -> true
  ;;
end

module Config = struct
  type t =
    { label : string
    ; value : int
    ; maximum : int
    ; star_size : float
    ; disabled : bool
    ; read_only : bool
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    t.maximum >= 1
    && t.maximum <= max_stars
    && t.value >= 0
    && t.value <= t.maximum
    && Float.is_finite t.star_size
    && Float.(t.star_size >= 8. && t.star_size <= 128.)
    && String.length t.label <= 4096
    && (not (String.is_empty (String.strip t.label)))
    && Stdlib.String.is_valid_utf_8 t.label
    && not (String.contains t.label '\000')
  ;;

  let can_apply t request =
    (not t.disabled)
    && (not t.read_only)
    && Request.valid request
    &&
    match request with
    | Set value | Toggle value -> value <= t.maximum
    | Increase | Decrease -> true
  ;;
end
