(** Updating the phase rotates slice order while preserving IDs. *)
val data_exn : float -> Gpuio.Chart_data.t

(** Build/Research keep their explicit domain colors; Review demonstrates an
    unknown key, with either an explicit color or the ordinary palette fallback. *)
val mapping : unknown:bool -> Gpuio.Chart_style.Ordinal.t
