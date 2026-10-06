open Core

type t =
  | Default
  | Rounded
  | Muted
  | Minimum
  | Spaced
  | Rich_labels
  | Hidden_target
[@@deriving equal]

val all : t list
val label : t -> string
val options : t -> Gpuio.Chart_options.Sankey.t
val node_labels : t -> Gpuio.Chart_node_labels.t

(** Main, tiny and zero parallel flows. Phase must be finite in [0,1000]. *)
val data_exn : float -> Gpuio.Chart_data.t
