open Core
module Wire = Gpuio_protocol.Chart_inspection_content_wire
module D = Chart_data

module Target = struct
  type t =
    { wire : Wire.Target.t
    ; data : Chart_resource.t option
    }
  [@@deriving equal, sexp_of]

  let stable wire = { wire; data = None }

  let cartesian ~series ~datum =
    stable
      (Wire.Target.Cartesian (D.Series_id.to_int64 series, D.Datum_id.to_int64 datum))
  ;;

  let slice id = stable (Wire.Target.Slice (D.Datum_id.to_int64 id))

  let radar ~series ~axis =
    stable (Wire.Target.Radar (D.Series_id.to_int64 series, D.Datum_id.to_int64 axis))
  ;;

  let candlestick id = stable (Wire.Target.Candlestick (D.Datum_id.to_int64 id))
  let node id = stable (Wire.Target.Node (D.Node_id.to_int64 id))
  let edge id = stable (Wire.Target.Edge (D.Edge_id.to_int64 id))

  module Expert = struct
    let to_wire t = t.wire

    let of_wire wire ~data =
      if not (Wire.Target.valid wire)
      then Or_error.error_string "invalid chart inspection content target"
      else (
        match wire with
        | Aggregate { source; _ } ->
          if
            Gpuio_protocol.Resource_id.equal source (Chart_resource.Expert.native_id data)
          then Ok { wire; data = Some data }
          else
            Or_error.error_string
              "inspection content publication belongs to another resource"
        | Cartesian _ | Slice _ | Radar _ | Candlestick _ | Node _ | Edge _ ->
          Ok (stable wire))
    ;;
  end

  let of_selection selection ~data ~data_revision ~data_generation =
    match
      Wire.Target.of_selection
        (Chart_selection.Expert.to_wire selection)
        ~source:(Chart_resource.Expert.native_id data)
        ~data_revision
        ~data_generation
    with
    | None ->
      Or_error.error_string "inspection content requires a valid positive publication"
    | Some wire -> Expert.of_wire wire ~data
  ;;

  let for_data t data =
    Option.some_if (Option.for_all t.data ~f:(Chart_resource.equal data)) t.wire
  ;;
end

module Container = Wire.Container

module Entry = struct
  type 'view t =
    { target : Target.t
    ; container : Container.t
    ; content : 'view
    }

  let create ~target ?(container = Container.Card) content =
    { target; container; content }
  ;;

  let target t = t.target
  let container t = t.container
  let content t = t.content
end

type 'view t = 'view Entry.t list

let create entries =
  if List.length entries > 128
  then Or_error.error_string "inspection content requires at most 128 entries"
  else if
    List.contains_dup
      (List.map entries ~f:(fun entry -> Target.Expert.to_wire (Entry.target entry)))
      ~compare:Wire.Target.compare
  then Or_error.error_string "inspection content targets must be unique"
  else Ok entries
;;

let empty = []

module Expert = struct
  let entries t = t

  let metadata t ~data =
    List.map t ~f:(fun entry ->
      { Wire.Entry.target = Target.for_data (Entry.target entry) data
      ; container = Entry.container entry
      })
  ;;
end
