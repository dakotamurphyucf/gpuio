(** Typed asynchronous input observations around a retained native editor. Only
    fixed-size counts/latest samples are retained; page departure drops previews. *)
val component
  :  Gpuio_eio.App.Window.t
  -> Palette.t Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> Gpuio_bonsai.View.t Bonsai.Cont.t
