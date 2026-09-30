(** Native width/height rules with independently retained branch drafts. Hidden
    branches stay mounted; leaving the page disposes all native editors. Logical
    breakpoint dimensions are deliberately independent of preview text size. *)
val component
  :  Gpuio_eio.App.Window.t
  -> Palette.t Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> Gpuio_bonsai.View.t Bonsai.Cont.t
