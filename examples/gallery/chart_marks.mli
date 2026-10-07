open Core

(** Pure chart appearance examples. Charts_page owns Bonsai state and resources. *)
type t =
  | Default
  | Paths
  | Markers
  | Signed_bars
  | Domain_bars
  | Value_bars
  | Uniform_buckets
  | Slash_pattern
  | Checkerboard
  | Raised_area
[@@deriving equal]

val all : t list
val label : t -> string
val configuration : t -> Palette.t -> Gpuio.Chart_data.t -> Gpuio.Chart_appearance.t
val sampling : t -> Gpuio.Chart_sampling.t
val dots : t -> bool
