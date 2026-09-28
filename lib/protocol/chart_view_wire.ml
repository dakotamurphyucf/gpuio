open Core

module Config = struct
  type t =
    { source : Resource_id.t option
    ; label : string
    ; options : Chart_options_wire.t
    ; sampling : Chart_sampling_wire.t
    ; style : Chart_style_wire.t
    ; legend : bool
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    String.length t.label <= 1024
    && Stdlib.String.is_valid_utf_8 t.label
    && String.exists t.label ~f:(fun c ->
      let n = Char.to_int c in
      not ((n >= 9 && n <= 13) || n = 32))
    && (not
          (String.exists t.label ~f:(fun c ->
             Char.equal c '\000' || Char.equal c '\r' || Char.equal c '\n')))
    && Chart_options_wire.valid t.options
    && Chart_sampling_wire.valid t.sampling
    && Chart_style_wire.valid t.style
  ;;
end

module Error = struct
  type t =
    | Wrong_application
    | Unavailable_data
    | Render_limit
    | Native_failure
  [@@deriving bin_io, equal, sexp_of]
end

module Metrics = struct
  type t =
    { source_values : int64
    ; retained_values : int64
    ; mesh_vertices : int64
    ; quads : int64
    ; bytes : int64
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Int64.(
      t.source_values >= 0L
      && t.source_values <= 100_000L
      && t.retained_values >= 0L
      && t.retained_values <= t.source_values
      && t.mesh_vertices >= 0L
      && t.mesh_vertices <= 1_000_000L
      && t.quads >= 0L
      && t.quads <= 300_000L
      && t.bytes >= 0L
      && t.bytes <= 67_108_864L)
  ;;
end

module Observation = struct
  type t =
    | Ready of Metrics.t
    | Failed of Error.t
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Ready metrics -> Metrics.valid metrics
    | Failed _ -> true
  ;;
end
