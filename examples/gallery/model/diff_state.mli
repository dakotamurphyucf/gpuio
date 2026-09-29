(** The gallery's optional application-owned expansion policy. Native-managed
    mode keeps configuration seeds stable while observing local changes. *)
type t

module Action : sig
  type t =
    | Toggle_controlled
    | Toggle_words
    | Reset
    | Observe of Gpuio.Document.Diff.Event.t
end

val initial : t
val apply : t -> Action.t -> t
val config : t -> Gpuio.Document.Diff.Config.t
val controlled : t -> bool
val word_diff : t -> bool
val notice : t -> string
