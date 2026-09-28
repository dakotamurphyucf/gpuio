open Core
module Wire = Gpuio_protocol.Chart_selection_wire
module D = Chart_data

module Span = struct
  type t =
    { start_index : int
    ; length : int
    ; first : D.Datum_id.t
    ; last : D.Datum_id.t
    }
  [@@deriving equal, sexp_of]

  let to_wire t : Wire.Span.t =
    { start_index = Int64.of_int t.start_index
    ; length = Int64.of_int t.length
    ; first = D.Datum_id.to_int64 t.first
    ; last = D.Datum_id.to_int64 t.last
    }
  ;;

  let create ~start_index ~length ~first ~last =
    let t = { start_index; length; first; last } in
    if Wire.Span.valid (to_wire t)
    then Ok t
    else Or_error.error_string "invalid chart source span"
  ;;

  let of_wire (t : Wire.Span.t) =
    let open Or_error.Let_syntax in
    let%bind first = D.Datum_id.of_int64 t.first in
    let%bind last = D.Datum_id.of_int64 t.last in
    create
      ~start_index:(Int64.to_int_exn t.start_index)
      ~length:(Int64.to_int_exn t.length)
      ~first
      ~last
  ;;
end

module Aggregation = Wire.Aggregation

type t =
  | Cartesian of
      { series : D.Series_id.t
      ; span : Span.t
      ; aggregation : Aggregation.t
      }
  | Slice of D.Datum_id.t
  | Radar of
      { series : D.Series_id.t
      ; axis : D.Datum_id.t
      }
  | Candlestick of
      { span : Span.t
      ; aggregated : bool
      }
  | Node of D.Node_id.t
  | Edge of D.Edge_id.t
[@@deriving equal, sexp_of]

let cartesian ~series ~span ~aggregation =
  if Aggregation.equal aggregation Exact && span.Span.length <> 1
  then Or_error.error_string "an exact chart selection must contain one datum"
  else Ok (Cartesian { series; span; aggregation })
;;

let slice id = Slice id
let radar ~series ~axis = Radar { series; axis }

let candlestick ~span ~aggregated =
  if (not aggregated) && span.Span.length <> 1
  then Or_error.error_string "an exact candle selection must contain one datum"
  else Ok (Candlestick { span; aggregated })
;;

let node id = Node id
let edge id = Edge id

module Expert = struct
  let to_wire = function
    | Cartesian { series; span; aggregation } ->
      Wire.Cartesian
        { series = D.Series_id.to_int64 series; span = Span.to_wire span; aggregation }
    | Slice id -> Wire.Slice (D.Datum_id.to_int64 id)
    | Radar { series; axis } ->
      Wire.Radar { series = D.Series_id.to_int64 series; axis = D.Datum_id.to_int64 axis }
    | Candlestick { span; aggregated } ->
      Wire.Candlestick { span = Span.to_wire span; aggregated }
    | Node id -> Wire.Node (D.Node_id.to_int64 id)
    | Edge id -> Wire.Edge (D.Edge_id.to_int64 id)
  ;;

  let of_wire t =
    if not (Wire.valid t)
    then Or_error.error_string "invalid chart selection"
    else
      let open Or_error.Let_syntax in
      match t with
      | Wire.Cartesian { series; span; aggregation } ->
        let%bind series = D.Series_id.of_int64 series in
        let%bind span = Span.of_wire span in
        cartesian ~series ~span ~aggregation
      | Slice id -> D.Datum_id.of_int64 id >>| slice
      | Radar { series; axis } ->
        let%bind series = D.Series_id.of_int64 series in
        let%map axis = D.Datum_id.of_int64 axis in
        radar ~series ~axis
      | Candlestick { span; aggregated } ->
        let%bind span = Span.of_wire span in
        candlestick ~span ~aggregated
      | Node id -> D.Node_id.of_int64 id >>| node
      | Edge id -> D.Edge_id.of_int64 id >>| edge
  ;;
end
