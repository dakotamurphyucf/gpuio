open Core

(** Qualification only. Register one sequential window with the separate audit
    backend. After close, that backend samples live GPUI entities against a
    post-warmup baseline and writes process records for the external collector.
    [metal_memory] additionally requires macOS renderer-device allocation samples.
    This does not preserve a retired component's event route. *)
val instance
  :  warmups:int
  -> measurements:int
  -> cycle:int
  -> metal_memory:bool
  -> unit Gpuio.Extension.Instance.t Or_error.t
