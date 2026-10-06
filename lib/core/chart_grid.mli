open Core

(** Grid presentation resolved by [Chart_style.create]. [Axes.grid] remains
    the master visibility flag, independently of axis-line/label visibility. *)
type t [@@deriving equal, sexp_of]

(** Omitted x/y positions follow the final axis ticks; explicit lists (at most
    64 each) replace them, including empty lists. Position semantics are those
    of [Chart_axis.Tick_position]. Radar ignores positions but uses appearance.
    Width [0.5,8]; dashes are empty for solid or 1..16 lengths in [0.5,128].
    Odd patterns repeat over two cycles; every line starts with a painted span.
    Excessive dash geometry fails native RenderLimit instead of approximating. *)
val create
  :  ?x:Chart_axis.Tick_position.t list
  -> ?y:Chart_axis.Tick_position.t list
  -> ?dashes:float list
  -> ?width:float
  -> ?color:Color.t
  -> unit
  -> t Or_error.t

val default : t

module Expert : sig
  val to_wire : t -> theme:Theme.t -> Gpuio_protocol.Chart_grid_wire.t Or_error.t
end
