open Core

(** Internal theme resolution shared by chart presentation and data-owned
    brushes. This module depends on neither chart identities nor resources.
    Resolve every referenced token, including colors on currently hidden marks;
    retain no theme or callback in the returned wire value. *)
val resolve
  :  Background.t
  -> theme:Theme.t
  -> Gpuio_protocol.Chart_appearance_wire.Brush.t Or_error.t
