(** Render only while the caller's actual request is pending. Native visibility
    and motion preferences govern frames; this module owns no clock or task. *)
val spinner : dark:bool -> label:string -> Gpuio_bonsai.View.t

val results : dark:bool -> Gpuio_bonsai.View.t
