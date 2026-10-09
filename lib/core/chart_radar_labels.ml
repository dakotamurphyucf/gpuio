open Core

module Entry = struct
  type 'view t =
    { axis : Chart_data.Datum_id.t
    ; content : 'view
    }

  let create ~axis content = { axis; content }
  let axis t = t.axis
  let content t = t.content
end

type 'view t = 'view Entry.t list

let create entries =
  if List.length entries > 64
  then Or_error.error_string "radar labels require at most 64 entries"
  else if
    List.contains_dup
      (List.map entries ~f:Entry.axis)
      ~compare:Chart_data.Datum_id.compare
  then Or_error.error_string "radar label axis IDs must be unique"
  else Ok entries
;;

let empty = []

module Expert = struct
  let entries t = t

  let axes t =
    List.map t ~f:(fun entry -> Chart_data.Datum_id.to_int64 (Entry.axis entry))
  ;;
end
