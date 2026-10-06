(** Explicitly ordered categories, equal labels with distinct IDs, a missing bar,
    and a mixed line layer. All data is constructed through public OCaml APIs. *)
val data_exn : float -> Gpuio.Chart_data.t

val describe_selection : Gpuio.Chart_data.t -> Gpuio.Chart_selection.t -> string option
