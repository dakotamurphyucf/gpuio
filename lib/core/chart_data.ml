open Core

let max_points = 100_000
let max_series = 32
let max_text_bytes = 8 * 1024 * 1024
let valid_number value = Float.is_finite value && Float.(abs value <= 1e100)
let valid_nonnegative value = valid_number value && Float.(value >= 0.)
let require condition message = if condition then Ok () else Or_error.error_string message

let text ~name ~limit ~nonblank value =
  require
    (String.length value <= limit
     && Stdlib.String.is_valid_utf_8 value
     && (not
           (String.exists value ~f:(fun c ->
              Char.equal c '\000' || Char.equal c '\r' || Char.equal c '\n')))
     && ((not nonblank) || not (String.is_empty (String.strip value))))
    (name ^ " must be bounded UTF-8 text without NUL/CR/LF")
;;

module type Id = sig
  type t [@@deriving equal, compare, sexp_of]

  val of_int64 : int64 -> t Or_error.t
  val to_int64 : t -> int64
end

module Make_id () : Id = struct
  type t = int64 [@@deriving equal, compare, sexp_of]

  let of_int64 t =
    let%map.Or_error () = require Int64.(t > 0L) "chart ID must be positive" in
    t
  ;;

  let to_int64 t = t
end

module Datum_id = Make_id ()
module Series_id = Make_id ()
module Node_id = Make_id ()
module Edge_id = Make_id ()

let unique values ~key =
  let seen = Hash_set.create (module Int64) in
  require
    (List.for_all values ~f:(fun value ->
       let key = key value in
       if Hash_set.mem seen key
       then false
       else (
         Hash_set.add seen key;
         true)))
    "chart IDs must be unique within their scope"
;;

let ordered values ~x =
  let rec loop previous = function
    | [] -> true
    | first :: rest ->
      let value = x first in
      Option.for_all previous ~f:(fun previous -> Float.(previous < value))
      && loop (Some value) rest
  in
  require (loop None values) "chart x coordinates must be strictly increasing"
;;

module Point = struct
  type t =
    { id : Datum_id.t
    ; x : float
    ; y : float option
    ; label : string
    }
  [@@deriving equal, sexp_of]

  let create ~id ~x ~y ?(label = "") () =
    let%bind.Or_error () =
      require
        (valid_number x && Option.for_all y ~f:valid_number)
        "chart coordinates must be finite and within +/-1e100"
    in
    let%map.Or_error () = text ~name:"point label" ~limit:256 ~nonblank:false label in
    { id; x; y; label }
  ;;

  let id t = t.id
  let x t = t.x
  let y t = t.y
  let label t = t.label
end

module Series = struct
  type t =
    { id : Series_id.t
    ; name : string
    ; points : Point.t list
    }
  [@@deriving equal, sexp_of]

  let create ~id ~name points =
    let%bind.Or_error () = text ~name:"series name" ~limit:128 ~nonblank:true name in
    let%bind.Or_error () =
      require (List.length points <= max_points) "chart point limit exceeded"
    in
    let%bind.Or_error () =
      unique points ~key:(fun point -> Datum_id.to_int64 (Point.id point))
    in
    let%map.Or_error () = ordered points ~x:Point.x in
    { id; name; points }
  ;;

  let id t = t.id
  let name t = t.name
  let points t = t.points
end

module Layer = struct
  type t =
    | Line of Series.t
    | Area of Series.t
    | Bar of Series.t
  [@@deriving equal, sexp_of]

  let series = function
    | Line t | Area t | Bar t -> t
  ;;
end

module Slice = struct
  type t =
    { id : Datum_id.t
    ; label : string
    ; value : float
    }
  [@@deriving equal, sexp_of]

  let create ~id ~label ~value =
    let%bind.Or_error () = text ~name:"slice label" ~limit:256 ~nonblank:true label in
    let%map.Or_error () =
      require (valid_nonnegative value) "slice value must be finite and in [0,1e100]"
    in
    { id; label; value }
  ;;

  let id t = t.id
  let label t = t.label
  let value t = t.value
end

module Radar_axis = struct
  type t =
    { id : Datum_id.t
    ; label : string
    ; maximum : float
    }
  [@@deriving equal, sexp_of]

  let create ~id ~label ~maximum =
    let%bind.Or_error () =
      text ~name:"radar axis label" ~limit:256 ~nonblank:true label
    in
    let%map.Or_error () =
      require
        (valid_number maximum && Float.(maximum > 0.))
        "radar maximum must be finite and in (0,1e100]"
    in
    { id; label; maximum }
  ;;

  let id t = t.id
  let label t = t.label
  let maximum t = t.maximum
end

module Radar_series = struct
  type t =
    { id : Series_id.t
    ; name : string
    ; values : (Datum_id.t * float) list
    }
  [@@deriving equal, sexp_of]

  let create ~id ~name values =
    let%bind.Or_error () =
      text ~name:"radar series name" ~limit:128 ~nonblank:true name
    in
    let%bind.Or_error () =
      require (List.length values <= 64) "radar axis limit exceeded"
    in
    let%bind.Or_error () = unique values ~key:(fun (id, _) -> Datum_id.to_int64 id) in
    let%map.Or_error () =
      require
        (List.for_all values ~f:(fun (_, value) -> valid_nonnegative value))
        "radar values must be finite and in [0,1e100]"
    in
    { id; name; values }
  ;;

  let id t = t.id
  let name t = t.name
  let values t = t.values
end

module Candle = struct
  type t =
    { id : Datum_id.t
    ; x : float
    ; label : string
    ; open_ : float
    ; high : float
    ; low : float
    ; close : float
    }
  [@@deriving equal, sexp_of]

  let create ~id ~x ~label ~open_ ~high ~low ~close =
    let%bind.Or_error () = text ~name:"candle label" ~limit:256 ~nonblank:false label in
    let%map.Or_error () =
      require
        (List.for_all [ x; open_; high; low; close ] ~f:valid_number
         && Float.(low <= open_ && open_ <= high && low <= close && close <= high))
        "candle requires bounded finite values and low <= open,close <= high"
    in
    { id; x; label; open_; high; low; close }
  ;;

  let id t = t.id
  let x t = t.x
  let label t = t.label
  let open_ t = t.open_
  let high t = t.high
  let low t = t.low
  let close t = t.close
end

module Node = struct
  type t =
    { id : Node_id.t
    ; label : string
    }
  [@@deriving equal, sexp_of]

  let create ~id ~label =
    let%map.Or_error () = text ~name:"flow node label" ~limit:256 ~nonblank:true label in
    { id; label }
  ;;

  let id t = t.id
  let label t = t.label
end

module Edge = struct
  type t =
    { id : Edge_id.t
    ; source : Node_id.t
    ; target : Node_id.t
    ; value : float
    }
  [@@deriving equal, sexp_of]

  let create ~id ~source ~target ~value =
    let%map.Or_error () =
      require
        ((not (Node_id.equal source target)) && valid_nonnegative value)
        "flow edge requires distinct nodes and a finite value in [0,1e100]"
    in
    { id; source; target; value }
  ;;

  let id t = t.id
  let source t = t.source
  let target t = t.target
  let value t = t.value
end

type contents =
  | Cartesian of Layer.t list
  | Pie of Slice.t list
  | Radar of Radar_axis.t list * Radar_series.t list
  | Candlestick of Candle.t list
  | Sankey of Node.t list * Edge.t list
[@@deriving equal, sexp_of]

type t =
  { contents : contents
  ; value_count : int
  ; text_bytes : int
  }
[@@deriving equal, sexp_of]

let sum_bytes values ~text =
  List.sum (module Int) values ~f:(fun value -> String.length (text value))
;;

let finish contents ~value_count ~text_bytes =
  let%map.Or_error () =
    require (text_bytes <= max_text_bytes) "chart text budget exceeded"
  in
  { contents; value_count; text_bytes }
;;

let value_count t = t.value_count
let text_bytes t = t.text_bytes

let cartesian layers =
  let%bind.Or_error () =
    require (List.length layers <= max_series) "chart series limit exceeded"
  in
  let series = List.map layers ~f:Layer.series in
  let%bind.Or_error () =
    unique series ~key:(fun series -> Series_id.to_int64 (Series.id series))
  in
  let value_count =
    List.sum (module Int) series ~f:(fun series -> List.length (Series.points series))
  in
  let%bind.Or_error () =
    require (value_count <= max_points) "chart point limit exceeded"
  in
  let%bind.Or_error () =
    require
      (List.for_all layers ~f:(function
         | Layer.Line _ | Area _ -> true
         | Bar series ->
           List.for_all (Series.points series) ~f:(fun point ->
             Option.is_some (Point.y point))))
      "bars cannot contain missing values"
  in
  let text_bytes =
    List.sum
      (module Int)
      series
      ~f:(fun series ->
        String.length (Series.name series)
        + sum_bytes (Series.points series) ~text:Point.label)
  in
  finish (Cartesian layers) ~value_count ~text_bytes
;;

let line series = cartesian (List.map series ~f:(fun series -> Layer.Line series))
let area series = cartesian (List.map series ~f:(fun series -> Layer.Area series))
let bar series = cartesian (List.map series ~f:(fun series -> Layer.Bar series))

let pie slices =
  let%bind.Or_error () = require (List.length slices <= 256) "pie slice limit exceeded" in
  let%bind.Or_error () =
    unique slices ~key:(fun slice -> Datum_id.to_int64 (Slice.id slice))
  in
  finish
    (Pie slices)
    ~value_count:(List.length slices)
    ~text_bytes:(sum_bytes slices ~text:Slice.label)
;;

let radar ~axes series =
  let axis_count = List.length axes in
  let%bind.Or_error () =
    require
      (axis_count <= 64
       && (axis_count = 0 || axis_count >= 3)
       && List.length series <= max_series
       && (axis_count > 0 || List.is_empty series))
      "radar requires 3..64 axes, <=32 series, or wholly empty input"
  in
  let%bind.Or_error () =
    unique axes ~key:(fun axis -> Datum_id.to_int64 (Radar_axis.id axis))
  in
  let%bind.Or_error () =
    unique series ~key:(fun series -> Series_id.to_int64 (Radar_series.id series))
  in
  let maxima = Int64.Table.create () in
  List.iter axes ~f:(fun axis ->
    Hashtbl.set
      maxima
      ~key:(Datum_id.to_int64 (Radar_axis.id axis))
      ~data:(Radar_axis.maximum axis));
  let%bind.Or_error () =
    require
      (List.for_all series ~f:(fun series ->
         List.length (Radar_series.values series) = axis_count
         && List.for_all (Radar_series.values series) ~f:(fun (id, value) ->
           match Hashtbl.find maxima (Datum_id.to_int64 id) with
           | None -> false
           | Some maximum -> Float.(value <= maximum))))
      "radar series must cover each axis exactly and remain within its maximum"
  in
  finish
    (Radar (axes, series))
    ~value_count:(axis_count * List.length series)
    ~text_bytes:
      (sum_bytes axes ~text:Radar_axis.label + sum_bytes series ~text:Radar_series.name)
;;

let candlestick candles =
  let%bind.Or_error () =
    require (List.length candles <= max_points) "chart candle limit exceeded"
  in
  let%bind.Or_error () =
    unique candles ~key:(fun candle -> Datum_id.to_int64 (Candle.id candle))
  in
  let%bind.Or_error () = ordered candles ~x:Candle.x in
  finish
    (Candlestick candles)
    ~value_count:(List.length candles)
    ~text_bytes:(sum_bytes candles ~text:Candle.label)
;;

let acyclic nodes edges =
  let degrees = Int64.Table.create () in
  let outgoing = Int64.Table.create () in
  List.iter nodes ~f:(fun node ->
    Hashtbl.set degrees ~key:(Node_id.to_int64 (Node.id node)) ~data:0);
  let%bind.Or_error () =
    require
      (List.for_all edges ~f:(fun edge ->
         Hashtbl.mem degrees (Node_id.to_int64 (Edge.source edge))
         && Hashtbl.mem degrees (Node_id.to_int64 (Edge.target edge))))
      "flow edge refers to a missing node"
  in
  List.iter edges ~f:(fun edge ->
    let source = Node_id.to_int64 (Edge.source edge) in
    let target = Node_id.to_int64 (Edge.target edge) in
    Hashtbl.update degrees target ~f:(fun count -> Option.value_exn count + 1);
    Hashtbl.add_multi outgoing ~key:source ~data:target);
  let ready = Queue.create () in
  Hashtbl.iteri degrees ~f:(fun ~key ~data -> if data = 0 then Queue.enqueue ready key);
  let visited = ref 0 in
  while not (Queue.is_empty ready) do
    let source = Queue.dequeue_exn ready in
    Int.incr visited;
    List.iter (Hashtbl.find_multi outgoing source) ~f:(fun target ->
      let remaining = Hashtbl.find_exn degrees target - 1 in
      Hashtbl.set degrees ~key:target ~data:remaining;
      if remaining = 0 then Queue.enqueue ready target)
  done;
  require (!visited = List.length nodes) "flow graph must be acyclic"
;;

let sankey ~nodes ~edges =
  let%bind.Or_error () =
    require
      (List.length nodes <= 256 && List.length edges <= 2048)
      "flow graph resource limit exceeded"
  in
  let%bind.Or_error () =
    unique nodes ~key:(fun node -> Node_id.to_int64 (Node.id node))
  in
  let%bind.Or_error () =
    unique edges ~key:(fun edge -> Edge_id.to_int64 (Edge.id edge))
  in
  let%bind.Or_error () = acyclic nodes edges in
  finish
    (Sankey (nodes, edges))
    ~value_count:(List.length edges)
    ~text_bytes:(sum_bytes nodes ~text:Node.label)
;;

module Expert = struct
  type nonrec contents = contents =
    | Cartesian of Layer.t list
    | Pie of Slice.t list
    | Radar of Radar_axis.t list * Radar_series.t list
    | Candlestick of Candle.t list
    | Sankey of Node.t list * Edge.t list
  [@@deriving equal, sexp_of]

  let contents t = t.contents
end
