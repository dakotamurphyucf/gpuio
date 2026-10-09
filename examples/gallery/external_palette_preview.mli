(** An Eio-backed search example. [search] runs in a page-owned task and may use
    explicit I/O capabilities captured at application startup. Result definitions
    are staged through View; their accepted Bonsai lifecycle publishes native
    order against the observed query. *)
val component
  :  search:(string -> string list)
  -> Gpuio_eio.App.Window.t
  -> Palette.t Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> Gpuio_bonsai.View.t Bonsai.Cont.t
