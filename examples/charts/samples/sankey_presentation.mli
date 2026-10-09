open Core

type t =
  | Default
  | Rounded
  | Muted
  | Minimum
  | Spaced
  | Rich_labels
  | Hidden_target
  | Gradient
  | Target
[@@deriving equal]

val all : t list
val label : t -> string

val options
  :  ?label_placement:Gpuio.Chart_options.Sankey.Label_placement.t
  -> ?labels:bool
  -> t
  -> Gpuio.Chart_options.Sankey.t

(** [middle] includes the three-column sample's processing node. [long] replaces
    captions with deliberately wide Unicode text; it preserves explicit hiding. *)
val node_labels : ?middle:bool -> ?long:bool -> t -> Gpuio.Chart_node_labels.t

(** Main, tiny and zero parallel flows. Phase must be finite in [0,1000]. *)
val data_exn : float -> Gpuio.Chart_data.t

(** Three columns with source nodes deliberately stored out of topology order.
    The incoming main/tiny/zero flows retain their IDs; the outgoing flow is
    their sum. Phase must be finite in [0,1000]. *)
val placement_data_exn : float -> Gpuio.Chart_data.t
