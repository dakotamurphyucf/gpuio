open Core
module N = Gpuio.Number_input

module Error : sig
  type t =
    | Work_failed of Error.t
    | Resolution_failed of N.Command_error.t
  [@@deriving sexp_of]
end

type t

(** Cancels work/queued completion and declines the original intent once.
    A proposal already dispatched to native code cannot be undone. *)
val cancel : t -> unit

val is_finished : t -> bool

(** Internal adapter. [decline] must be nonblocking, nonraising on the owning
    domain, generation-checked and independent of ordinary command capacity.
    Start failures also decline. Cancellation suppresses [on_result]; normal
    completion releases scopes/cleanup before invoking it. [f] runs in Eio and
    must not use Bonsai from another domain. *)
val start
  :  scope:Scope.t
  -> resolve:
       (N.Step_resolution.t -> (N.Snapshot.t, N.Command_error.t) Result.t Bonsai.Effect.t)
  -> decline:(unit -> unit)
  -> f:(unit -> N.Step_resolution.t)
  -> on_result:((N.Snapshot.t, Error.t) Result.t -> unit Bonsai.Effect.t)
  -> t Or_error.t
