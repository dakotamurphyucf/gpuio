(** Typed cursor vocabulary and clipped/end/start-truncated native text. The
    complete source remains the accessible label. Controls preserve the preview
    choice across page visits and require no per-frame OCaml work. *)
val component
  :  Palette.t Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> Gpuio_bonsai.View.t Bonsai.Cont.t
