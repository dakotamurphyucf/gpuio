(** Search presentation for one opted-in multiline editor. Rust owns the document
    and matching; Bonsai owns the query/replacement fields and controls. *)
type t

(** The supplied editor must use [Text_input.Config ~searchable:true]. Keep its
    controller outside this component and pass the same window used to create it. Each open/reopen gives the query controls
    a fresh lifetime; the document editor retains its draft, selection and undo.
    Query composition is not sent to search until committed. The bar owns its
    query draft during an opening; raw source search commands do not replace that
    draft. Reopen to seed it from native search state. The committed replacement
    draft persists for this component's lifetime. *)
val create
  :  App.Window.t
  -> editor:Text_input.t Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> t Bonsai.Cont.t

(** Explicit user gesture. Ordinary rendering must not call this effect. *)
val open_ : t -> ?replace:bool -> unit -> unit Bonsai.Effect.t

(** Wrap exactly one editor placement and its related content. Supplies a local
    Find/Replace/F3/Shift-F3/Escape command scope. Enter/Shift-Enter navigate only
    inside the query field, preserving document newlines. Shortcuts defer during
    composition. The query takes focus and selects its unchanged seed once per
    opening. Close restores document focus atomically for that opening only,
    skipping focus when the document is hidden or disabled.

    Field names, count/error text and controls are English. Styles inherit from
    the application; [bar_style] refines the bar container. Mount this wrapper
    once for the component's active lifetime. No OS/input acceptance is implied
    by constructing a view. *)
val wrap
  :  ?style:Gpuio.Style.t
  -> ?bar_style:Gpuio.Style.t
  -> t
  -> Gpuio_bonsai.View.t
  -> Gpuio_bonsai.View.t
