open Core

(** Qualification-only native snapshots. Begin settles for two seconds before
    capturing; await Begun rather than Command_completed. Finish captures before
    its command redraw. Fetch bounded bucket pages only after Finished.
    [Document_preparation] reads application-wide cumulative worker elapsed
    microseconds, including discarded jobs. It is separate from frame snapshots;
    compare phase boundaries and retain peak fields as absolute maxima. *)
module Command : sig
  type t =
    | Begin
    | Finish
    | Document_preparation
    | Buckets of
        { metric : int
        ; offset : int
        }
end

module Event : sig
  type t =
    | Begun of int64
    | Document_preparation of
        { queue_us : int64
        ; configure_us : int64
        ; parse_us : int64
        ; highlight_us : int64
        ; search_us : int64
        ; source_bytes : int64
        ; completed : int64
        ; discarded : int64
        ; peak_workers : int64
        ; peak_reserved_bytes : int64
        }
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
