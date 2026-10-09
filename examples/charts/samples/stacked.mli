(** Aligned categorical layers with signed values, gaps and stable source IDs.
    Both forms use the same original observations and a separate line overlay.
    Select [Chart_options.Stacking.Stacked] to accumulate the two value layers. *)
val data_exn : area:bool -> float -> Gpuio.Chart_data.t
