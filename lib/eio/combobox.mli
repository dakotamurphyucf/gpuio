open Core

(** One native editable choice placement. Rust owns query/caret/composition;
    Bonsai owns the selected application ID and available choices. *)
type t

val create
  :  App.Window.t
  -> config:Gpuio.Combobox.Config.t Bonsai.Cont.t
  -> ?initial_text:string
  -> on_select:(Gpuio.Combobox.Selection.t -> unit Bonsai.Effect.t) Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> t Bonsai.Cont.t

val view
  :  ?style:Gpuio.Style.t
  -> ?appearance:Gpuio.Choice.Appearance.t
  -> t
  -> Gpuio_bonsai.View.t

(** Last native observation, absent before first mount. It includes selection,
    focus and composition changes as well as text changes. Derive query-dependent
    computations from it; an observation never implicitly replaces text. *)
val snapshot : t -> Gpuio.Text_input.Snapshot.t option

val command
  :  t
  -> Gpuio.Text_input.Command.t
  -> (Gpuio.Text_input.Snapshot.t, Gpuio.Text_input.Command_error.t) Result.t
       Bonsai.Effect.t

val focus
  :  t
  -> (Gpuio.Text_input.Snapshot.t, Gpuio.Text_input.Command_error.t) Result.t
       Bonsai.Effect.t

val replace
  :  t
  -> ?if_revision:Gpuio.Text_input.Revision.t
  -> selection:Gpuio.Text_input.Selection_policy.t
  -> undo:Gpuio.Text_input.Undo_policy.t
  -> string
  -> (Gpuio.Text_input.Snapshot.t, Gpuio.Text_input.Command_error.t) Result.t
       Bonsai.Effect.t

(** Replace the query after accepting a selection, only if its exact editor lease
    and revision are still current. Later typing returns [Stale_revision], a
    remount returns [Stale_editor], and composition returns [Composing]. Native
    undo records the replacement; the selected application value is unaffected. *)
val replace_if_unchanged
  :  t
  -> Gpuio.Combobox.Selection.t
  -> string
  -> (Gpuio.Text_input.Snapshot.t, Gpuio.Text_input.Command_error.t) Result.t
       Bonsai.Effect.t
