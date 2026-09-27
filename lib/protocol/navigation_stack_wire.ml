open Core

module Motion = struct
  type t =
    | Immediate
    | Slide
    | Fade
  [@@deriving bin_io, equal, sexp_of]
end

module Config = struct
  type t =
    { selected : int64 option
    ; retain : bool
    ; motion : Motion.t
    ; duration_ms : int64
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Option.for_all t.selected ~f:(fun index -> Int64.(index >= 0L && index < 128L))
    && Int64.(t.duration_ms >= 0L && t.duration_ms <= 10_000L)
  ;;

  let valid_children t ~count =
    valid t
    && count >= 0
    && count <= 128
    &&
    match t.selected with
    | None -> count = 0
    | Some index -> Int64.(index < of_int count)
  ;;
end
