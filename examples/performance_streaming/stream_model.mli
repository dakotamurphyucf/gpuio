open Core

(** Qualification data only: four independent streams, with sixteen 128-byte
    fragments per history block. A point update preserves the collection order;
    only starting a new block appends a row. No native view is retained here. *)
module Row : sig
  type t =
    { stream : int
    ; block : int
    ; text : string
    }
end

module Progress : sig
  type t =
    { updates : int
    ; bytes : int
    }
  [@@deriving sexp_of]
end

type t

val stream_count : int
val fragment_bytes : int
val fragments_per_block : int
val create : unit -> t
val rows : t -> (int, Row.t, Int.comparator_witness) Gpuio.List_collection.t
val progress : t -> Progress.t list

(** Stream is in [0,4); sequence is positive and must be exactly the next
    sequence for that stream. Invalid updates return an error without mutation. *)
val append : t -> stream:int -> sequence:int -> t Or_error.t

(** Verify every retained block against the deterministic source and exact
    per-stream counts, including Unicode and the final partial block. *)
val validate : t -> updates_per_stream:int -> unit Or_error.t
