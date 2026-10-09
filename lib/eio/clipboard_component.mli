open Core
module Text = Gpuio.Clipboard.Text
module Error = Gpuio.Clipboard.Error

type t

val create
  :  (Text.t -> (unit, Error.t) Result.t Bonsai.Effect.t)
  -> text:Text.t Bonsai.Cont.t
  -> ?disabled:bool Bonsai.Cont.t
  -> ?on_copied:(Text.t -> unit Bonsai.Effect.t) Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> t Bonsai.Cont.t

val is_busy : t -> bool
val is_copied : t -> bool
val error : t -> Error.t option
val copy : t -> unit Bonsai.Effect.t

val view
  :  t
  -> ?label:string
  -> ?copied_label:string
  -> ?style:Gpuio.Style.t
  -> unit
  -> unit Bonsai.Effect.t Gpuio.View.t
