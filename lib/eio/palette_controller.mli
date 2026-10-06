open Core

(** Native-owned palette query/highlight with asynchronous typed commands.
    Use one controller with one mounted command palette. Pass [key] and [observe]
    to the view's [key] and [on_change] arguments. *)
type t

val create : App.Window.t -> Bonsai.Cont.graph -> t Bonsai.Cont.t
val key : t -> Gpuio.Key.t
val snapshot : t -> Gpuio.Command_palette.Snapshot.t option
val observe : t -> Gpuio.Command_palette.Snapshot.t -> unit Bonsai.Effect.t

(** Clear the local observation when removing the palette. Native retirement
    still fences commands even if this bookkeeping effect has not run. *)
val reset : t -> unit Bonsai.Effect.t

(** Captures the observed window/node/subscription, never a replacement mount.
    At most 64 requests may be pending in the application. Query/focus/highlight
    mutations reject IME composition and unavailable native interaction. [Read_snapshot] does not
    require focus; [Focus] requires the query field to be visible.

    [Set_query] accepts at most 4096 UTF-8 bytes without NUL/CR/LF, places the
    caret at the end and clears query undo history. [Highlight None] clears the
    highlight until query editing, native navigation or an explicit selection.
    [Highlight (Some id)] requires a currently matching, enabled command. No
    command activates an item. Successful replies reflect application, not paint.

    [Set_loading] controls the native busy indicator and suppresses empty content.
    It preserves existing rows, selection, query, undo and composition, and may
    run while another overlay owns interaction. Use the query-checked command
    below for asynchronous search completion. Loading does not start a search or
    atomically publish application results. *)
val command
  :  t
  -> Gpuio.Command_palette.Command.t
  -> (Gpuio.Command_palette.Snapshot.t, Gpuio.Command_palette.Command_error.t) Result.t
       Bonsai.Effect.t

(** Additionally compare [expected]'s query identity against freshly read native
    state immediately before execution. Selection-only changes do not invalidate
    the query. This guards this command, not a subsequent View/config publication. *)
val command_if_query_unchanged
  :  t
  -> expected:Gpuio.Command_palette.Snapshot.t
  -> Gpuio.Command_palette.Command.t
  -> (Gpuio.Command_palette.Snapshot.t, Gpuio.Command_palette.Command_error.t) Result.t
       Bonsai.Effect.t
