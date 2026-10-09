(** Native targets, spring sequences and shared clocks. The application-wide
    preference is shared by all gallery windows. Leaving this page unmounts the
    animated wrappers, pauses the sequence and stops the repeating preview. *)
val component
  :  Gpuio_eio.App.t
  -> motion:Gpuio.Animation.Preference.t Bonsai.Cont.Expert.Var.t
  -> Palette.t Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> Gpuio_bonsai.View.t Bonsai.Cont.t
