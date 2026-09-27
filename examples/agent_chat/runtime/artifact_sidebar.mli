module Destination : sig
  type t =
    | Overview
    | Diagram
    | Review
    | Feedback
    | Tour
    | Sources
    | Results
end

(** Window-local navigation preferences. Selection follows the inspector route;
    expansion and collapse do not change that route or own page tasks. *)
val component
  :  current:Destination.t Bonsai.Cont.t
  -> icons:(Icons.Name.t * Gpuio.Asset.Handle.t) list Bonsai.Cont.t
  -> dark:bool Bonsai.Cont.t
  -> on_select:(Destination.t -> unit Bonsai.Effect.t)
  -> Bonsai.Cont.graph
  -> Gpuio_bonsai.View.t Bonsai.Cont.t
