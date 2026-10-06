open Core

module Position = struct
  type t =
    { x : float
    ; y : float
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    List.for_all [ t.x; t.y ] ~f:(fun value ->
      Float.is_finite value && Float.(abs value <= 1_000_000.))
  ;;
end

module Command = struct
  type t =
    | Show of Position.t
    | Close
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Show position -> Position.valid position
    | Close -> true
  ;;
end

module Error = struct
  type t =
    | Not_mounted
    | Closed
    | Stale_menu
    | Unavailable
    | Invalid_position
    | Busy
    | Native_failure
  [@@deriving bin_io, equal, sexp_of]
end

module Response = struct
  type t =
    | Applied
    | Failed of Error.t
  [@@deriving bin_io, equal, sexp_of]
end
