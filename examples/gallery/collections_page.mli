(** Loaded sample data stays outside transient row computations. Lists, trees and
    tables mount bounded native viewports using their public managed adapters. *)
val component
  :  Palette.t Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> Gpuio_bonsai.View.t Bonsai.Cont.t
