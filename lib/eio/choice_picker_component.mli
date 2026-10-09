open Core

(** One retained picker placement. Bonsai owns selection and catalog; Rust owns
    search text, caret/composition, scrolling and popup focus. *)
type t

val create
  :  Editor_controller.command
  -> config:Gpuio.Choice_picker.Config.t Bonsai.Cont.t
  -> ?initial_text:string
  -> on_event:(Gpuio.Choice_picker.Event.t -> unit Bonsai.Effect.t) Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> t Bonsai.Cont.t

(** The default root key is the controller's stable Bonsai placement key.
    The query is mounted exactly while the configuration enables search. Closing
    the popup retains it; disabling search retires its lease. Content uses the
    same passive-slot and aggregate-budget rules as [Gpuio.View.choice_picker]. *)
val view
  :  ?key:Gpuio.Key.t
  -> ?style:Gpuio.Style.t
  -> ?appearance:Gpuio.Choice_picker.Appearance.t
  -> ?trigger:Gpuio_bonsai.View.t
  -> ?empty:Gpuio_bonsai.View.t
  -> ?footer:Gpuio_bonsai.View.t
  -> ?groups:(Gpuio.Choice_picker.Group.Id.t * Gpuio_bonsai.View.t) list
  -> ?options:
       (Gpuio.Choice.Id.t * Gpuio_bonsai.View.t Gpuio.Choice_picker.Option_content.t) list
  -> t
  -> Gpuio_bonsai.View.t Or_error.t

(** Observations update the editor controller before invoking the application
    callback. This is not an optimistic selected-value cache. *)
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

(** Use the exact query captured by a selection request. Later typing returns
    [Stale_revision], remounting [Stale_editor], and composition [Composing]. A
    request with no query or a controller with search disabled returns
    [Not_mounted] without issuing a native command. Selection itself is unaffected. *)
val replace_if_unchanged
  :  t
  -> Gpuio.Choice_picker.Selection_request.t
  -> string
  -> (Gpuio.Text_input.Snapshot.t, Gpuio.Text_input.Command_error.t) Result.t
       Bonsai.Effect.t
