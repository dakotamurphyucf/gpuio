module Page : sig
  type t =
    | Sources
    | Results
    | Diagram
    | Feedback
end

(** A window-owned, bounded gallery of links to real workspace destinations.
    Auto-advance is opt-in and timed by the native carousel. Inactive routes
    invalidate its pending proposals; returning preserves manual selection. *)
val component
  :  active:bool Bonsai.Cont.t
  -> dark:bool Bonsai.Cont.t
  -> on_open:(Page.t -> unit Bonsai.Effect.t)
  -> Bonsai.Cont.graph
  -> Gpuio_bonsai.View.t Bonsai.Cont.t
