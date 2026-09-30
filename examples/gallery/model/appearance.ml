open Core

type t =
  | Light
  | Dark
[@@deriving equal, sexp_of]

let toggle = function
  | Light -> Dark
  | Dark -> Light
;;

let label = function
  | Light -> "Light"
  | Dark -> "Dark"
;;

module Scale = struct
  type t =
    | Compact
    | Comfortable
    | Large
  [@@deriving equal, sexp_of]

  let all = [ Compact; Comfortable; Large ]

  let next = function
    | Compact -> Comfortable
    | Comfortable -> Large
    | Large -> Compact
  ;;

  let label = function
    | Compact -> "Compact"
    | Comfortable -> "Comfortable"
    | Large -> "Large"
  ;;

  let factor = function
    | Compact -> 0.85
    | Comfortable -> 1.
    | Large -> 1.2
  ;;
end
