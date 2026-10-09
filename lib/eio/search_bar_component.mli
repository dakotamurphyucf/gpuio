open Core
module B = Bonsai.Cont
module S = Gpuio.Text_input.Search

type target =
  { search : S.Snapshot.t option
  ; command :
      S.Command.t
      -> (S.Response.t, Gpuio.Text_input.Command_error.t) Result.t Bonsai.Effect.t
  }

type t

val create : Editor_controller.command -> target:target B.t -> B.graph -> t B.t
val open_ : t -> ?replace:bool -> unit -> unit Bonsai.Effect.t

val wrap
  :  ?style:Gpuio.Style.t
  -> ?bar_style:Gpuio.Style.t
  -> t
  -> Gpuio_bonsai.View.t
  -> Gpuio_bonsai.View.t
