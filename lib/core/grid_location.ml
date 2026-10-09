open Core
module Wire = Gpuio_protocol.Grid_location_wire

module Line = struct
  type t = int [@@deriving equal, sexp_of]

  let of_int value =
    if value <> 0 && value >= -1025 && value <= 1025
    then Ok value
    else Or_error.error_string "Grid line must be nonzero and in -1025..1025"
  ;;

  let of_int_exn value = of_int value |> Or_error.ok_exn
  let to_int t = t
end

module Span = struct
  type t = int [@@deriving equal, sexp_of]

  let of_int value =
    if value >= 1 && value <= 1024
    then Ok value
    else Or_error.error_string "Grid span must be in 1..1024"
  ;;

  let of_int_exn value = of_int value |> Or_error.ok_exn
  let to_int t = t
end

module Edge = struct
  type t =
    | Auto
    | Line of Line.t
    | Span of Span.t
  [@@deriving equal, sexp_of]

  let to_wire = function
    | Auto -> Wire.Edge.Auto
    | Line line -> Wire.Edge.Line (Line.to_int line |> Int64.of_int)
    | Span span -> Wire.Edge.Span (Span.to_int span |> Int64.of_int)
  ;;
end

module Axis = struct
  type t =
    { start : Edge.t
    ; end_ : Edge.t
    }
  [@@deriving equal, sexp_of]

  let create ~start ~end_ = { start; end_ }
  let auto = create ~start:Auto ~end_:Auto
  let full = create ~start:(Line (Line.of_int_exn 1)) ~end_:(Line (Line.of_int_exn (-1)))
  let span count = create ~start:(Span count) ~end_:(Span count)
  let start t = t.start
  let end_ t = t.end_

  let to_wire { start; end_ } =
    { Wire.Axis.start = Edge.to_wire start; end_ = Edge.to_wire end_ }
  ;;
end

type t =
  { column : Axis.t
  ; row : Axis.t
  }
[@@deriving equal, sexp_of]

let create ?(column = Axis.auto) ?(row = Axis.auto) () = { column; row }
let column t = t.column
let row t = t.row

module Expert = struct
  let to_wire { column; row } =
    { Wire.column = Axis.to_wire column; row = Axis.to_wire row }
  ;;
end
