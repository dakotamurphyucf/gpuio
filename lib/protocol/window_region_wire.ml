open Core

module Edge = struct
  type t =
    | Top
    | Bottom
    | Left
    | Right
    | Top_left
    | Top_right
    | Bottom_left
    | Bottom_right
  [@@deriving bin_io, equal, sexp_of]
end

type t =
  | Title_bar
  | Exclude
  | Resize of Edge.t
[@@deriving bin_io, equal, sexp_of]
