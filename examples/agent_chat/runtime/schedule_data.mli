open Core
module C = Gpuio.Calendar

(** Reproducible civil-date fixtures. No ambient clock, time zone or scheduling
    service. Only available weekdays in October 2026 have sample reviews. *)
module Review : sig
  type t

  val date : t -> Date.t
  val label : t -> string
end

val month : C.Month.t
val constraints : C.Constraints.t
val config : mode:C.Mode.t -> label:string -> C.Config.t
val describe : C.Selection.t -> string
val reviews : C.Selection.t -> Review.t list Or_error.t
