open Core

(** Qualification-only native snapshots. Begin settles for two seconds before
    capturing; await Begun rather than Command_completed. Finish captures before
    its command redraw. Fetch bounded bucket pages only after Finished. *)
module Command : sig
  type t =
    | Begin
    | Finish
    | Buckets of
        { metric : int
        ; offset : int
        }
end

module Event : sig
  type t =
    | Begun of int64
    | Finished of
        { elapsed_ns : int64
        ; capture_ns : int64
        ; dropped_inputs : int64
        ; counts : int64 list
        }
    | Buckets of
        { metric : int
        ; offset : int
        ; total : int
        ; values : (int64 * int64) list
        }
  [@@deriving sexp]
end

val instance
  :  sequence:int64
  -> Command.t
  -> Event.t Gpuio.Extension.Instance.t Or_error.t
