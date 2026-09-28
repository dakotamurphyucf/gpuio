open Core
module Wire = Gpuio_protocol.Chart_view_wire
module Error = Wire.Error

module Metrics = struct
  type t =
    { source_values : int
    ; retained_values : int
    ; mesh_vertices : int
    ; quads : int
    ; bytes : int
    }
  [@@deriving equal, sexp_of]

  let of_wire (t : Wire.Metrics.t) =
    { source_values = Int64.to_int_exn t.source_values
    ; retained_values = Int64.to_int_exn t.retained_values
    ; mesh_vertices = Int64.to_int_exn t.mesh_vertices
    ; quads = Int64.to_int_exn t.quads
    ; bytes = Int64.to_int_exn t.bytes
    }
  ;;
end

module Selection = Chart_selection

module Observation = struct
  type t =
    | Ready of Metrics.t
    | Failed of Error.t
    | Selection_changed of Selection.t option
  [@@deriving equal, sexp_of]
end

module Event = struct
  type t =
    { data_revision : int64
    ; data_generation : int64
    ; observation : Observation.t
    }
  [@@deriving equal, sexp_of]
end

module Config = struct
  type t =
    { data : Chart_resource.t
    ; wire : Wire.Config.t
    }
  [@@deriving equal, sexp_of]

  let create
        ~data
        ?(label = "Chart")
        ?(legend = true)
        ?(options = Chart_options.default)
        ?(sampling = Chart_sampling.default)
        ?(style = Chart_style.default)
        ()
    =
    let wire =
      { Wire.Config.source = None
      ; label
      ; options = Chart_options.Expert.to_wire options
      ; sampling = Chart_sampling.Expert.to_wire sampling
      ; style = Chart_style.Expert.to_wire style
      ; legend
      }
    in
    if Wire.Config.valid wire
    then Ok { data; wire }
    else Or_error.error_string "invalid chart label"
  ;;

  let data t = t.data
end

module Expert = struct
  let to_wire (t : Config.t) ~owner =
    let source =
      Option.bind owner ~f:(fun owner ->
        if Chart_resource.Expert.belongs_to t.data ~owner
        then Some (Chart_resource.Expert.native_id t.data)
        else None)
    in
    { t.wire with source }
  ;;

  let event ~data_revision ~data_generation observation =
    let valid_identity =
      Int64.(data_revision > 0L && data_generation > 0L)
      || (Int64.(data_revision = 0L && data_generation = 0L)
          &&
          match observation with
          | Wire.Observation.Failed _ -> true
          | Ready _ | Selection_changed _ -> false)
    in
    if not (valid_identity && Wire.Observation.valid observation)
    then Or_error.error_string "invalid chart observation"
    else (
      let%map.Or_error observation =
        match observation with
        | Wire.Observation.Ready metrics ->
          Ok (Observation.Ready (Metrics.of_wire metrics))
        | Failed error -> Ok (Observation.Failed error)
        | Selection_changed selection ->
          let%map.Or_error selection =
            match selection with
            | None -> Ok None
            | Some selection ->
              Selection.Expert.of_wire selection |> Or_error.map ~f:Option.some
          in
          Observation.Selection_changed selection
      in
      { Event.data_revision; data_generation; observation })
  ;;
end
