open Core
module W = Gpuio_protocol.Table_wire
module Part = W.Part

module Padding = struct
  type t = W.Padding.t [@@deriving equal, sexp_of]

  let create ~top ~right ~bottom ~left =
    let t : t = { top; right; bottom; left } in
    if W.Padding.valid t
    then Ok t
    else Or_error.error_string "table padding edges must be finite in 0..4096"
  ;;

  let all n = create ~top:n ~right:n ~bottom:n ~left:n
  let zero = all 0. |> Or_error.ok_exn
end

type t =
  { striped : bool
  ; colors : (Part.t * Color.t) list
  ; padding : Padding.t option
  ; column_padding : (Table_column.Id.t * Padding.t) list
  }
[@@deriving equal, sexp_of]

let create ?(striped = false) ?(colors = []) ?padding ?(column_padding = []) () =
  if
    List.length colors > 13
    || List.contains_dup (List.map colors ~f:fst) ~compare:Part.compare
    || List.length column_padding > Table_column.Collection.max_columns
    || List.contains_dup (List.map column_padding ~f:fst) ~compare:Table_column.Id.compare
  then
    Or_error.error_string "duplicate table appearance parts/columns or too many overrides"
  else
    Ok
      { striped
      ; colors = List.sort colors ~compare:(fun (a, _) (b, _) -> Part.compare a b)
      ; padding
      ; column_padding =
          List.sort column_padding ~compare:(fun (a, _) (b, _) ->
            Table_column.Id.compare a b)
      }
;;

let default = create () |> Or_error.ok_exn
let is_striped t = t.striped

module Expert = struct
  let valid_columns t columns =
    List.for_all t.column_padding ~f:(fun (id, _) ->
      Option.is_some (Table_column.Collection.find columns id))
  ;;

  let to_wire t ~theme =
    let%map.Or_error colors =
      List.map t.colors ~f:(fun (part, c) ->
        let%map.Or_error c = Theme.resolve theme c in
        part, c)
      |> Or_error.all
    in
    { W.Appearance.striped = t.striped
    ; colors
    ; padding = t.padding
    ; column_padding =
        List.map t.column_padding ~f:(fun (id, p) -> Table_column.Id.to_string id, p)
    }
  ;;
end
