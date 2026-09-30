module B = Bonsai.Cont

(** Mount a native binding observer and build its children from the latest sample.
    [None] means the current configuration is waiting for its first native sample.
    A configuration change or component reactivation clears the old observation;
    effects captured during an earlier configuration visit cannot restore it.
    Child computations keep their own state across configuration changes.

    Mount the returned view for this component's active lifetime. The observer
    adds no focus stop and does not register shortcuts or invoke actions. *)
val component
  :  ?key:Gpuio.Key.t
  -> ?style:Gpuio.Style.t B.t
  -> config:Gpuio.Command_binding.Config.t B.t
  -> f:
       (Gpuio.Command_binding.Observation.t option B.t
        -> B.graph
        -> unit Bonsai.Effect.t Gpuio.View.t list B.t)
  -> B.graph
  -> unit Bonsai.Effect.t Gpuio.View.t B.t
