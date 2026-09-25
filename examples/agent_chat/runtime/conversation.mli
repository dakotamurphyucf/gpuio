open Core
module Backend = Gpuio_agent_chat_model.Fake_backend
module Pager = Gpuio_eio.List_paging

module Phase : sig
  type t =
    | Idle
    | Accepting
    | Streaming
    | Complete
    | Cancelled
    | Failed of string
  [@@deriving equal, sexp_of]
end

module Message : sig
  type body =
    | Plain of string
    | Rich of Gpuio_eio.Document.t * Gpuio.Document.Mode.t

  type t =
    { author : string
    ; body : body
    ; detail : string
    }
end

(** Application-owned, UI-domain-only conversation. Visibility does not own
    producers. At most 64 responses and 200 saved history rows per conversation.
    Native resource and task disposal follows the supplied parent scope. *)
type t

val create
  :  Gpuio_eio.App.t
  -> scope:Gpuio_eio.Scope.t
  -> id:int
  -> title:string
  -> sleep:(float -> unit)
  -> t Or_error.t

val id : t -> int
val title : t -> string
val scope : t -> Gpuio_eio.Scope.t
val pager : t -> (int, Message.t, Int.comparator_witness) Pager.t
val phase : t -> Phase.t
val phase_value : t -> Phase.t Bonsai.Cont.t
val initialize : t -> unit Bonsai.Effect.t

(** Admission is synchronous; acceptance is a cancellable window-owned task.
    After acceptance, generation belongs to the conversation and survives tab,
    row and window visibility. [on_accept] may conditionally clear the submitting
    editor. Cancelling the originating window before acceptance suppresses it. *)
val submit
  :  t
  -> window_scope:Gpuio_eio.Scope.t
  -> config:Backend.Config.t
  -> prompt:string
  -> on_accept:unit Bonsai.Effect.t
  -> unit Or_error.t

val cancel : t -> unit
val retry : t -> config:Backend.Config.t -> unit Or_error.t
val last_document : t -> Gpuio_eio.Document.t option
val response_count : t -> int
val is_busy : t -> bool

(** Register a bounded UTF-8 text attachment as an inspectable transcript artifact.
    At most eight attachments, each at most 64 KiB, per conversation. Call after
    reading through the originating window's Eio task scope. *)
val attach : t -> name:string -> text:string -> unit Or_error.t Bonsai.Effect.t
