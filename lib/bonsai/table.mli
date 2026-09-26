open Core
module B = Bonsai.Cont
module Row = Gpuio.Table_data.Row_ref
module Config = Gpuio.Table.Config
module Selection = Gpuio.Table.Selection
module Request = Gpuio.Table.Request
module Target = Gpuio.Table.Target

module Cell : sig
  (** The retained [copy_text] is also the native cell's accessibility value.
      Provide a readable textual equivalent for custom visual content. Native
      controls inside the cell retain their own accessibility semantics. *)
  type t

  (** Copy text and the child View are independent. Column identity must match
      the column passed to [render_cell]. Copy text uses the Core cell bounds. *)
  val create
    :  column:Gpuio.Table_column.Id.t
    -> copy_text:string
    -> unit Bonsai.Effect.t Gpuio.View.t
    -> t Or_error.t

  val text : column:Gpuio.Table_column.Id.t -> string -> t Or_error.t
end

module Controller : sig
  (** One mounted source and query, without a payload-bearing source snapshot.
      Retired mounts/queries and absent/reincarnated targets ignore delayed work. *)
  type t

  (** An explicit batch of at most 64 commands. A newer batch supersedes one
      that has not yet been displayed. Put selection and reveal in one batch
      when both must execute. A newer native selection supersedes a pending batch.
      Invalid current targets discard the whole batch;
      malformed offsets and oversized batches return Error at construction. *)
  val batch : t -> Row.t Target.t list -> unit Bonsai.Effect.t Or_error.t

  val select : t -> Row.t Selection.t -> unit Bonsai.Effect.t
  val reveal : t -> ?column:Gpuio.Table_column.Id.t -> Row.t -> unit Bonsai.Effect.t
  val scroll_to : t -> ?offset:float -> Row.t -> unit Bonsai.Effect.t Or_error.t
  val scroll_to_column : t -> Gpuio.Table_column.Id.t -> unit Bonsai.Effect.t
  val scroll_to_end : t -> unit Bonsai.Effect.t
  val reset_columns : t -> unit Bonsai.Effect.t
end

module Output : sig
  type 'data t

  val view : _ t -> unit Bonsai.Effect.t Gpuio.View.t
  val controller : _ t -> Controller.t
  val selection : _ t -> Row.t Selection.t
  val target : _ t -> Gpuio.Table_data.Id.t -> Row.t Or_error.t

  (** Latest native observation for the current query, order and configuration.
      Changes can temporarily return [None] until a matching observation arrives.
      A rendered-frame acknowledgement does not guarantee that observation has
      already been delivered to Bonsai. *)
  val viewport : _ t -> Gpuio.Virtual_list.Viewport.t option

  val active_rows : _ t -> int
  val active_cells : _ t -> int
  val budget_exhausted : _ t -> bool
end

(** A bounded read-only native table. The application owns [source], accepted
    columns/sort in [config], and data loading. Native selection is optimistic;
    observe [Output.selection] or issue an explicit controller command. Removed
    rows/columns and disallowed modes repair selection to Empty. Sort, resize,
    reorder, activation, context and copy proposals reach [on_request]; apply
    accepted column/query changes in application state. This component never
    sorts a partial remote dataset or synchronously calls OCaml from Rust.

    A fresh Table_data lineage resets the entire widget. Increasing
    [query_generation] retires old effects and transient cell models while keeping
    the native table and surviving row anchors/selection. Source/query changes
    do not cancel application I/O; reset the Eio pager to cancel its producers.
    Generations must be nonnegative and cannot decrease within a native mount.

    Only requested/pinned rows allocate cells. All columns of each active row
    count against [config]'s cell limit. Cells use Managed_rows' default-reset
    lifecycle contract and receive their own lifetime guard. Preferences and
    application jobs belong outside transient cell computations. Source-order
    metadata is O(logical rows); point updates share it without rebuilding keys.
    Give the table bounded geometry through [style] or its parent. The caller's
    key and style belong to the native table root: padding, border and surface
    decoration are applied once. Style updates preserve selection and anchors;
    source-lineage resets remain independent of the caller's sibling key. *)
val component
  :  'data Gpuio.Table_data.t B.t
  -> config:Config.t B.t
  -> ?key:Gpuio.Key.t
  -> ?style:Gpuio.Style.t B.t
  -> ?query_generation:int64 B.t
  -> ?on_request:(Row.t Request.t -> unit Bonsai.Effect.t) B.t
  -> render_cell:
       (row:Row.t B.t
        -> data:'data B.t
        -> column:Gpuio.Table_column.t B.t
        -> lifetime:Managed_rows.Lifetime.t B.t
        -> B.graph
        -> Cell.t Or_error.t B.t)
  -> B.graph
  -> 'data Output.t Or_error.t B.t

module Paging = Virtual_list.Paging

(** Requests Ready boundaries after native layout, including empty/short sources.
    Failed boundaries require [Paging.retry]. Leaving the viewport does not cancel
    application loads. [auto_load=false] suspends new automatic requests. Controls
    must recheck generation at execution; Gpuio_eio.Table_paging.controls does so. *)
val paged
  :  ('query, 'data) Gpuio.Table_paging.Snapshot.t B.t
  -> paging:Paging.t B.t
  -> config:Config.t B.t
  -> ?key:Gpuio.Key.t
  -> ?style:Gpuio.Style.t B.t
  -> ?auto_load:bool B.t
  -> ?on_request:(Row.t Request.t -> unit Bonsai.Effect.t) B.t
  -> render_cell:
       (row:Row.t B.t
        -> data:'data B.t
        -> column:Gpuio.Table_column.t B.t
        -> lifetime:Managed_rows.Lifetime.t B.t
        -> B.graph
        -> Cell.t Or_error.t B.t)
  -> B.graph
  -> 'data Output.t Or_error.t B.t
