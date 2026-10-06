open Core

type t =
  | Default
  | Rounded
  | Muted
  | Minimum
  | Spaced
[@@deriving equal]

val all : t list
val label : t -> string
val options : t -> Gpuio.Chart_options.Sankey.t

(** Main, tiny and zero parallel flows. Phase must be finite in [0,1000]. *)
val data_exn : float -> Gpuio.Chart_data.t
