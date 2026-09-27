(** Fixed-height, native width selection for small presentation alternatives.
    Keep editors, tasks and document owners outside these branches. No OCaml
    callback observes sizes or chooses the branch during resizing. *)
val at_width
  :  key:string
  -> height:float
  -> breakpoint:float
  -> compact:Gpuio_bonsai.View.t
  -> wide:Gpuio_bonsai.View.t
  -> Gpuio_bonsai.View.t
