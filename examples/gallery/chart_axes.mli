open Core

(** Pure presentation presets; Bonsai state and source ownership live in Charts_page. *)
type t =
  | Default
  | Styled
  | Floating
  | Labels_only
  | Lines_only
[@@deriving equal]

val all : t list
val label : t -> string

val configuration
  :  t
  -> Palette.t
  -> Gpuio.Chart_axis.t * Gpuio.Chart_axis.t * Gpuio.Chart_grid.t
