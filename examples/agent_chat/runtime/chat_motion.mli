(** Native presentation only: these helpers own no clock, task or Bonsai state.
    Stable keys preserve retargeting. Hidden views and reduced motion are handled
    by the public animation host. *)
val stage_context : expanded:bool -> Gpuio_bonsai.View.t list -> Gpuio_bonsai.View.t

(** Two ordered stages, with a small initial delay based on destination order. *)
val destination : index:int -> Gpuio_bonsai.View.t -> Gpuio_bonsai.View.t

(** Mount only while the associated conversation is busy. Members with the same
    application-scoped [group] share phase, including in independent windows. *)
val activity : key:string -> group:string -> Gpuio_bonsai.View.t -> Gpuio_bonsai.View.t
