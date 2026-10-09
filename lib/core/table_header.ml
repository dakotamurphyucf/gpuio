open Core
module W = Gpuio_protocol.Table_header_wire

module Target = struct
  type t = W.t [@@deriving compare, equal, sexp_of]

  let column id = W.Column (Table_column.Id.to_string id)

  let group ~level ~columns =
    let invalid () =
      Or_error.error_string "header group requires level 0..3 and 1..64 distinct columns"
    in
    if level < 0 || level >= 4 || List.is_empty columns || List.length columns > 64
    then invalid ()
    else (
      let columns =
        List.map columns ~f:Table_column.Id.to_string |> List.sort ~compare:String.compare
      in
      let target = W.Group { level; columns } in
      if W.valid target then Ok target else invalid ())
  ;;

  let matches t columns = W.matches t (Table_column.Expert.to_wire columns)
end

type 'view t =
  { key : Key.t
  ; target : Target.t
  ; content : 'view
  }

let create ~key ~target content = { key; target; content }
let key t = t.key
let target t = t.target
let content t = t.content

let validate_all headers ~columns =
  if List.length headers > 320
  then Or_error.error_string "at most 320 custom table headers are allowed"
  else if List.contains_dup (List.map headers ~f:key) ~compare:Key.compare
  then Or_error.error_string "duplicate custom table header key"
  else if List.contains_dup (List.map headers ~f:target) ~compare:Target.compare
  then Or_error.error_string "duplicate custom table header target"
  else (
    let schema = Table_column.Expert.to_wire columns in
    if List.for_all headers ~f:(fun header -> W.matches header.target schema)
    then Ok ()
    else Or_error.error_string "custom table header target is absent from the schema")
;;

module Expert = struct
  let target_to_wire target = target

  let target_of_wire target =
    if W.valid target
    then Ok target
    else Or_error.error_string "invalid or noncanonical custom table header target"
  ;;
end
