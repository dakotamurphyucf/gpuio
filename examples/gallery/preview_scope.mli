open Core

type 'a t =
  | Loading
  | Ready of 'a
  | Failed of Error.t

(** Acquire on each Bonsai activation in a fresh child of the window scope.
    Departure cancels producers and registrations, suppresses late results and
    clears the view state. Partial acquisition failure also closes the scope.
    [create] must use this scope for every resource it acquires. *)
val acquire
  :  Gpuio_eio.App.Window.t
  -> name:string
  -> create:(Gpuio_eio.Scope.t -> 'a Or_error.t Bonsai.Effect.t)
  -> Bonsai.Cont.graph
  -> 'a t Bonsai.Cont.t
