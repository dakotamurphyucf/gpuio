(** Loaded sample data stays outside transient row computations. Lists, trees and
    tables mount bounded native viewports using their public managed adapters. *)
val component
  :  Gpuio_eio.App.t
  -> Selectable_preview.t
  -> Gpuio_eio.App.Window.t
  -> Palette.t Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> Gpuio_bonsai.View.t Bonsai.Cont.t
