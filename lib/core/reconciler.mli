open Core

(** Runtime adapter; ordinary application code uses View and the application
    runner. All operations belong to the same OCaml UI domain. *)
type 'action t

type 'action update

val create
  :  ?asset_owner:Asset.Expert.Owner.t
  -> ?document_owner:Text_source.Expert.Owner.t
  -> Gpuio_protocol.Window_id.t
  -> 'action t

(** Prepare against acknowledged state. Preparation has no published effects and
    may be discarded if submission fails. Only one update may be in flight per
    window. Physical sharing of unchanged View subtrees is preserved. *)
val prepare
  :  'action t
  -> theme:Theme.t
  -> 'action View.t option
  -> 'action update Or_error.t

val message : _ update -> Gpuio_protocol.Wire.Message.t option

(** Publish only after native Accepted, or immediately when message is None.
    Even an empty native diff refreshes callbacks. Stale/foreign updates fail. *)
val accept : 'action t -> 'action update -> unit Or_error.t

(** Map a correlated native retry to callbacks of the last accepted list. *)
val retain_list_rows
  :  'action t
  -> Gpuio_protocol.List_wire.Retained.t list
  -> 'action list Or_error.t

val dispatch : 'action t -> Gpuio_protocol.Wire.Event.t -> 'action option
val revision : _ t -> int64
val close : _ t -> unit
