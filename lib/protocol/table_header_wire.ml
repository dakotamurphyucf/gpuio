open Core

module Group = struct
  type t =
    { level : int
    ; columns : string list
    }
  [@@deriving bin_io, compare, equal, sexp_of]
end

type t =
  | Column of string
  | Group of Group.t
[@@deriving bin_io, compare, equal, sexp_of]

let valid = function
  | Column id -> Table_wire.valid_id id
  | Group { level; columns } ->
    level >= 0
    && level < Table_wire.max_header_levels
    && (not (List.is_empty columns))
    && List.length columns <= Table_wire.max_columns
    && List.for_all columns ~f:Table_wire.valid_id
    && List.is_sorted_strictly columns ~compare:String.compare
;;

let matches t (schema : Table_wire.Schema.t) =
  valid t
  &&
  match t with
  | Column id -> List.exists schema.columns ~f:(fun column -> String.equal column.id id)
  | Group { level; columns } ->
    List.nth schema.headers level
    |> Option.exists ~f:(fun groups ->
      List.exists groups ~f:(fun group ->
        List.equal String.equal columns (List.sort group.columns ~compare:String.compare)))
;;
