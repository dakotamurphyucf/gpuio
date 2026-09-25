open Core

module Name : sig
  type t =
    | Spark
    | Search
    | Message
    | Plus
    | Arrow_up
    | Paperclip
    | External
    | Sun
    | Moon
    | Command
    | Close
    | Code
    | Check
    | Stop
    | Retry
    | Sliders
    | History
    | Arrow_down
  [@@deriving equal]
end

(** Original monochrome SVGs, registered once in the application scope. *)
type t

val create : unit -> t
val initialize : t -> Gpuio_eio.App.t -> unit Bonsai.Effect.t
val value : t -> (Name.t * Gpuio.Asset.Handle.t) list Bonsai.Cont.t

val decoration
  :  (Name.t * Gpuio.Asset.Handle.t) list
  -> Name.t
  -> Gpuio.Icon.Decoration.t option

val view : (Name.t * Gpuio.Asset.Handle.t) list -> Name.t -> Gpuio_bonsai.View.t
