open Core

type t =
  { color : Color.t
  ; offset_x : float
  ; offset_y : float
  ; blur : float
  ; spread : float
  ; inset : bool
  }
[@@deriving equal, sexp_of]

let create ?(inset = false) ~color ~offset_x ~offset_y ~blur ~spread () =
  if
    List.for_all [ offset_x; offset_y; blur; spread ] ~f:(fun v ->
      Float.is_finite v && Float.(abs v <= 1_000_000.))
    && Float.(blur >= 0.)
  then Ok { color; offset_x; offset_y; blur; spread; inset }
  else Or_error.error_string "invalid shadow dimensions"
;;

module Expert = struct
  type description = t =
    { color : Color.t
    ; offset_x : float
    ; offset_y : float
    ; blur : float
    ; spread : float
    ; inset : bool
    }

  let describe t = t
end
