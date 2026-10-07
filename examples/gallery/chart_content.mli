(** Page-owned Bonsai controls and one native editor for pie inspection content.
    The chart page owns its source; this component never registers or publishes
    data. Its editor is placed at most once, on stable slice ID 1. *)
type t

val component
  :  Gpuio_eio.App.Window.t
  -> Palette.t Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> t Bonsai.Cont.t

val controls : t -> Gpuio_bonsai.View.t

(** Native mode and non-pie data return no custom entries. Rows mode attaches
    formatted values to each slice; interactive modes attach one editor/button
    to ID 1, with ordinary rows for other slices. Removing the interactive
    content destroys the native draft; the page-owned activation count persists. *)
val content
  :  t
  -> Gpuio.Chart_data.t option
  -> Gpuio_bonsai.View.t Gpuio.Chart_inspection_content.t
