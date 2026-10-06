open Core

(** Shared deterministic example data, built exclusively with the public chart
    API. No native resources or application state live in this module. *)
module Family : sig
  type t =
    | Line
    | Area
    | Bar
    | Pie
    | Radar
    | Candlestick
    | Sankey
  [@@deriving equal, sexp_of]

  val all : t list
  val of_index : int -> t Or_error.t
  val label : t -> string
  val description : t -> string
end

module Preset : sig
  type t =
    | Standard
    | Mixed
    | Horizontal
    | Dense_legend
  [@@deriving equal]
end

(** Phases must be finite and nonnegative. Invalid phases or generated values
    raise; these are checked example fixtures. IDs stay stable across phases.
    Standard pie/radar/Sankey data is intentionally constant. *)
val data_exn : Family.t -> float -> Gpuio.Chart_data.t

val preset_data_exn : Preset.t -> Family.t -> float -> Gpuio.Chart_data.t
val edge_data : Family.t -> Gpuio.Chart_data.t
val description : Preset.t -> Family.t -> string

(** Resolve a selection only against its published source revision. This checks
    IDs at source-span endpoints but does not infer publication identity. *)
val describe_selection : Gpuio.Chart_data.t -> Gpuio.Chart_selection.t -> string option

module Categorical : sig
  val data_exn : float -> Gpuio.Chart_data.t
  val describe_selection : Gpuio.Chart_data.t -> Gpuio.Chart_selection.t -> string option
end

module Stacked : sig
  val data_exn : area:bool -> float -> Gpuio.Chart_data.t
end

module Ordinal_colors : sig
  val data_exn : float -> Gpuio.Chart_data.t
  val mapping : unknown:bool -> Gpuio.Chart_style.Ordinal.t
end
