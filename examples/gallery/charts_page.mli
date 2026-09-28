(** All seven families and mixed layers, native keyboard/selection/data access.
    Each page visit owns one chart registration in a fresh child scope; native
    preparation and selection delivery never call OCaml synchronously. *)
val component
  :  Gpuio_eio.App.t
  -> Gpuio_eio.App.Window.t
  -> Palette.t Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> Gpuio_bonsai.View.t Bonsai.Cont.t
