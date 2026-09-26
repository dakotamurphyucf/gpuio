open Core

val item
  :  _ Gpuio.Tree_rows.Item.t
  -> config:Gpuio.Virtual_list.Config.t
  -> content:unit Bonsai.Effect.t Gpuio.View.t
  -> on_toggle:unit Bonsai.Effect.t
  -> unit Bonsai.Effect.t Gpuio.View.t

val boundary
  :  Gpuio.Tree_rows.Boundary.t
  -> config:Gpuio.Virtual_list.Config.t
  -> can_load:bool
  -> request:unit Bonsai.Effect.t
  -> retry:unit Bonsai.Effect.t
  -> unit Bonsai.Effect.t Gpuio.View.t

val label : _ Gpuio.Tree_rows.Item.t -> unit Bonsai.Effect.t Gpuio.View.t
