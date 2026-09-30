open Core

exception Invalid_wire_observation

let max_specs = 16
let max_ranges = 4096
let max_query_bytes = 4096
let max_config_bytes = 262144

module Query = struct
  type t =
    { text : string
    ; case_sensitive : bool
    ; whole_word : bool
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    (not (String.is_empty t.text))
    && String.length t.text <= max_query_bytes
    && Stdlib.String.is_valid_utf_8 t.text
    && not (String.contains t.text '\000')
  ;;
end

module Range = struct
  type t =
    { start_byte : int64
    ; end_byte : int64
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t = Int64.(t.start_byte >= 0L && t.end_byte > t.start_byte)
end

module Appearance = struct
  type t =
    { color : int64
    ; active_color : int64
    ; radius : float
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    let color c = Int64.(c >= 0L && c <= 0xffff_ffffL) in
    color t.color
    && color t.active_color
    && Float.is_finite t.radius
    && Float.(t.radius >= 0. && t.radius <= 64.)
  ;;
end

module Spec = struct
  type t =
    { query : Query.t option
    ; ranges : Range.t list
    ; appearance : Appearance.t
    ; active_index : int64 option
    ; match_index_offset : int64
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    (Option.is_some t.query || not (List.is_empty t.ranges))
    && Option.for_all t.query ~f:Query.valid
    && List.length t.ranges <= max_ranges
    && List.for_all t.ranges ~f:Range.valid
    && Appearance.valid t.appearance
    && Option.for_all t.active_index ~f:(fun n -> Int64.(n >= 0L))
    && Int64.(t.match_index_offset >= 0L)
  ;;
end

module Config = struct
  type t = Spec.t list [@@deriving bin_io, equal, sexp_of]

  let valid t =
    List.length t <= max_specs
    && List.for_all t ~f:Spec.valid
    && List.sum (module Int) t ~f:(fun spec -> List.length spec.ranges) <= max_ranges
    && bin_size_t t <= max_config_bytes
  ;;
end

module Count = struct
  type t =
    { total : int64
    ; stored : int64
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Int64.(t.total >= 0L && t.stored >= 0L && t.stored <= t.total && t.stored <= 16384L)
  ;;
end

module Counts = struct
  type t = Count.t list [@@deriving bin_io, equal, sexp_of]

  let bin_read_t buffer ~pos_ref =
    let count = (Bin_prot.Read.bin_read_nat0 buffer ~pos_ref :> int) in
    if count < 0 || count > max_specs then raise Invalid_wire_observation;
    if count > (Bigstring.length buffer - !pos_ref) / 2
    then raise Bin_prot.Common.Buffer_short;
    (* Core.List.init evaluates in descending index order; this cursor must be
       consumed in wire order. *)
    let rec read remaining reversed =
      if remaining = 0
      then List.rev reversed
      else (
        let value = Count.bin_read_t buffer ~pos_ref in
        read (remaining - 1) (value :: reversed))
    in
    read count []
  ;;

  let bin_reader_t = { bin_reader_t with read = bin_read_t }
  let bin_t = { bin_t with reader = bin_reader_t }
end

module Range_error = struct
  type t =
    | Out_of_bounds
    | Scalar_boundary
  [@@deriving bin_io, equal, sexp_of]
end

module Invalid_range = struct
  type t =
    { spec_index : int64
    ; range_index : int64
    ; reason : Range_error.t
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Int64.(
      t.spec_index >= 0L
      && t.spec_index < 16L
      && t.range_index >= 0L
      && t.range_index < 4096L)
  ;;
end

module Limit = struct
  type t =
    | Source
    | Work
    | Admission
  [@@deriving bin_io, equal, sexp_of]
end

module Failure = struct
  type t =
    | Source_unavailable
    | Worker_failed
    | Epoch_exhausted
  [@@deriving bin_io, equal, sexp_of]
end

module State = struct
  type t =
    | Pending
    | Ready of Counts.t
    | Invalid_range of Invalid_range.t
    | Capacity of Limit.t
    | Failed of Failure.t
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Pending | Capacity _ | Failed _ -> true
    | Invalid_range range -> Invalid_range.valid range
    | Ready counts ->
      List.length counts <= max_specs
      && List.for_all counts ~f:Count.valid
      && Int64.(List.sum (module Int64) counts ~f:(fun c -> c.stored) <= 16384L)
  ;;
end

module Observation = struct
  type t =
    { epoch : int64
    ; state : State.t
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t = Int64.(t.epoch > 0L) && State.valid t.state

  let valid_for t (config : Config.t) =
    valid t
    &&
    match t.state with
    | Pending | Capacity _ | Failed _ -> true
    | Ready counts -> List.length counts = List.length config
    | Invalid_range range ->
      Option.exists
        (List.nth config (Int64.to_int_exn range.spec_index))
        ~f:(fun spec -> Int64.(range.range_index < of_int (List.length spec.ranges)))
  ;;

  let bin_read_t buffer ~pos_ref =
    let t = bin_read_t buffer ~pos_ref in
    if valid t then t else raise Invalid_wire_observation
  ;;

  let bin_reader_t = { bin_reader_t with read = bin_read_t }
  let bin_t = { bin_t with reader = bin_reader_t }
end
