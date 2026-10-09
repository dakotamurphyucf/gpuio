open Core

(** Bounded Eio read of the example profile. I/O and parse failures are values;
    Eio cancellation propagates. The caller owns task scope and UI publication. *)
val load : _ Eio.Path.t -> Gpuio_gallery_model.Theme_profile.t Or_error.t
