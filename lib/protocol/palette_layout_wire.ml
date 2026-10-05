open Core

module Entry = struct
  type t =
    | Command of int
    | Group of string * string option * int list
    | Separator
  [@@deriving bin_io, equal, sexp_of]
end

type t = Entry.t list [@@deriving bin_io, equal, sexp_of]

let text_bytes t =
  List.sum
    (module Int)
    t
    ~f:(function
      | Entry.Command _ | Separator -> 0
      | Group (id, label, _) ->
        String.length id + Option.value_map label ~default:0 ~f:String.length)
;;
