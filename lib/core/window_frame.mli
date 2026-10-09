open Core

(** Immutable geometry for a custom window's native client frame. macOS and
    server-decorated windows use their OS frame. Metrics are logical pixels;
    changing the application tree never resets the platform inset. Frame color
    follows the root view's resolved [Border_color], with a neutral fallback. *)
type t [@@deriving equal, sexp_of]

(** Shadow margin is finite in [0,128]; resize half-band in [0.5,32]. The frame
    keeps a one-pixel border on untiled sides, proportionally compressed only
    for exceptionally small viewports. Default margins are 20 and 4. *)
val create : ?shadow_size:float -> ?resize_hit_size:float -> unit -> t Or_error.t

val default : t

module Expert : sig
  val to_wire : t -> Gpuio_protocol.Window_wire.Frame.t
end
