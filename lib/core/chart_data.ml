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
     && ((not nonblank)
         || String.exists value ~f:(fun c -> not (String.contains " \t\r\n\011\012" c))))
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
module Category_id = Make_id ()

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

module Category = struct
  type t =
    { id : Category_id.t
    ; label : string
    }
  [@@deriving equal, sexp_of]

  let create ~id ~label =
    let%map.Or_error () = text ~name:"category label" ~limit:256 ~nonblank:true label in
    { id; label }
  ;;

  let id t = t.id
  let label t = t.label
end

module Categorical_point = struct
  type t =
    { id : Datum_id.t
    ; category : Category_id.t
    ; value : float option
    ; label : string
    }
  [@@deriving equal, sexp_of]

  let create ~id ~category ~value ?(label = "") () =
    let%bind.Or_error () =
      require
        (Option.for_all value ~f:valid_number)
        "category values must be finite and within +/-1e100"
    in
    let%map.Or_error () =
      text ~name:"categorical point label" ~limit:256 ~nonblank:false label
    in
    { id; category; value; label }
  ;;

  let id t = t.id
  let category t = t.category
  let value t = t.value
  let label t = t.label
end

module Categorical_series = struct
  type t =
    { id : Series_id.t
    ; name : string
    ; points : Categorical_point.t list
    }
  [@@deriving equal, sexp_of]

  let create ~id ~name points =
    let%bind.Or_error () =
      text ~name:"categorical series name" ~limit:128 ~nonblank:true name
    in
    let%bind.Or_error () =
      require (List.length points <= max_points) "chart point limit exceeded"
    in
    let%bind.Or_error () =
      unique points ~key:(fun p -> Datum_id.to_int64 (Categorical_point.id p))
    in
    let%map.Or_error () =
      unique points ~key:(fun p -> Category_id.to_int64 (Categorical_point.category p))
    in
    { id; name; points }
  ;;

  let id t = t.id
  let name t = t.name
  let points t = t.points
end

module Categorical_layer = struct
  type t =
    | Line of Categorical_series.t
    | Area of Categorical_series.t
    | Bar of Categorical_series.t
  [@@deriving equal, sexp_of]

  let series = function
    | Line s | Area s | Bar s -> s
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
  | Categorical of Category.t list * Categorical_layer.t list
[@@deriving equal, sexp_of]

type t =
  { contents : contents
  ; value_count : int
  ; text_bytes : int
  ; bar_backgrounds : Gpuio_protocol.Chart_data_wire.Bar_background.t list
  ; bar_baselines : Gpuio_protocol.Chart_data_wire.Bar_baseline.t list
  }
[@@deriving equal, sexp_of]

let sum_bytes values ~text =
  List.sum (module Int) values ~f:(fun value -> String.length (text value))
;;

let finish contents ~value_count ~text_bytes =
  let%map.Or_error () =
    require (text_bytes <= max_text_bytes) "chart text budget exceeded"
  in
  { contents; value_count; text_bytes; bar_backgrounds = []; bar_baselines = [] }
;;

let categorical ~categories layers =
  let%bind.Or_error () =
    require
      (List.length categories <= max_points && List.length layers <= max_series)
      "categorical chart domain/series limit exceeded"
  in
  let%bind.Or_error () =
    unique categories ~key:(fun c -> Category_id.to_int64 (Category.id c))
  in
  let series = List.map layers ~f:Categorical_layer.series in
  let%bind.Or_error () =
    unique series ~key:(fun s -> Series_id.to_int64 (Categorical_series.id s))
  in
  let value_count =
    List.sum (module Int) series ~f:(fun s -> List.length (Categorical_series.points s))
  in
  let%bind.Or_error () =
    require (value_count <= max_points) "chart point limit exceeded"
  in
  let%bind.Or_error () =
    require
      (List.for_all series ~f:(fun s ->
         match
           List.for_all2 categories (Categorical_series.points s) ~f:(fun c p ->
             Category_id.equal (Category.id c) (Categorical_point.category p))
         with
         | Ok valid -> valid
         | Unequal_lengths -> false))
      "categorical series must cover the domain exactly in its declared order"
  in
  let text_bytes =
    sum_bytes categories ~text:Category.label
    + List.sum
        (module Int)
        series
        ~f:(fun s ->
          String.length (Categorical_series.name s)
          + sum_bytes (Categorical_series.points s) ~text:Categorical_point.label)
  in
  finish (Categorical (categories, layers)) ~value_count ~text_bytes
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

module Bar_background = struct
  type t =
    { series : Series_id.t
    ; datum : Datum_id.t
    ; background : Background.t
    }
  [@@deriving equal, sexp_of]

  let create ~series ~datum background = { series; datum; background }
end

module Bar_baseline = struct
  type t =
    { series : Series_id.t
    ; datum : Datum_id.t
    ; baseline : float
    }
  [@@deriving equal, sexp_of]

  let create ~series ~datum baseline =
    let%map.Or_error () =
      require (valid_number baseline) "bar baseline must be finite within +/-1e100"
    in
    { series; datum; baseline }
  ;;
end

module Bar_key = struct
  module T = struct
    type t = int64 * int64 [@@deriving compare, sexp]
  end

  include T
  include Comparator.Make (T)
end

let background_key (b : Gpuio_protocol.Chart_data_wire.Bar_background.t) =
  b.series, b.datum
;;

let validate_bar_pairs t entries ~name ~key ~valid =
  let%bind.Or_error () =
    require (List.length entries <= max_points) (name ^ " limit exceeded")
  in
  let%bind.Or_error () =
    require
      (let rec strictly_ordered = function
         | [] | [ _ ] -> true
         | first :: (second :: _ as rest) ->
           Bar_key.compare (key first) (key second) < 0 && strictly_ordered rest
       in
       strictly_ordered entries)
      (name ^ " pairs must be strictly ordered and unique")
  in
  if List.is_empty entries
  then Ok ()
  else (
    let keys =
      match t.contents with
      | Cartesian layers ->
        List.concat_map layers ~f:(function
          | Layer.Line _ | Area _ -> []
          | Bar s ->
            List.map s.points ~f:(fun p ->
              Series_id.to_int64 s.id, Datum_id.to_int64 p.Point.id))
      | Categorical (_, layers) ->
        List.concat_map layers ~f:(function
          | Categorical_layer.Line _ | Area _ -> []
          | Bar s ->
            List.map s.points ~f:(fun p ->
              Series_id.to_int64 s.id, Datum_id.to_int64 p.Categorical_point.id))
      | Pie _ | Radar _ | Candlestick _ | Sankey _ -> []
    in
    let keys = Set.of_list (module Bar_key) keys in
    require
      (List.for_all entries ~f:(fun b -> valid b && Set.mem keys (key b)))
      (name ^ " entries must be valid and refer to existing bar observations"))
;;

let validate_backgrounds t entries =
  validate_bar_pairs
    t
    entries
    ~name:"chart background"
    ~key:background_key
    ~valid:(fun b -> Gpuio_protocol.Chart_appearance_wire.Brush.valid b.brush)
;;

let baseline_key (b : Gpuio_protocol.Chart_data_wire.Bar_baseline.t) = b.series, b.datum

let validate_baselines t entries =
  validate_bar_pairs t entries ~name:"chart baseline" ~key:baseline_key ~valid:(fun b ->
    valid_number b.baseline)
;;

let bar_baseline t ~series ~datum =
  let key = Series_id.to_int64 series, Datum_id.to_int64 datum in
  List.find_map t.bar_baselines ~f:(fun b ->
    if Bar_key.compare (baseline_key b) key = 0 then Some b.baseline else None)
;;

module Expert = struct
  type nonrec contents = contents =
    | Cartesian of Layer.t list
    | Pie of Slice.t list
    | Radar of Radar_axis.t list * Radar_series.t list
    | Candlestick of Candle.t list
    | Sankey of Node.t list * Edge.t list
    | Categorical of Category.t list * Categorical_layer.t list
  [@@deriving equal, sexp_of]

  let contents t = t.contents

  let retained_bytes t =
    let categories =
      match t.contents with
      | Categorical (categories, _) -> List.length categories
      | _ -> 0
    in
    65_536
    + (512 * List.length t.bar_backgrounds)
    + (128 * List.length t.bar_baselines)
    + (512 * t.value_count)
    + (128 * categories)
    + (4 * t.text_bytes)
  ;;

  module Wire = Gpuio_protocol.Chart_data_wire

  let convert values ~f =
    let%map.Or_error values =
      List.fold_result values ~init:[] ~f:(fun acc value ->
        let%map.Or_error value = f value in
        value :: acc)
    in
    List.rev values
  ;;

  let point_to_wire (t : Point.t) : Wire.Point.t =
    { id = Datum_id.to_int64 (Point.id t)
    ; x = Point.x t
    ; y = Point.y t
    ; label = Point.label t
    }
  ;;

  let point_of_wire (wire : Wire.Point.t) =
    let%bind.Or_error id = Datum_id.of_int64 wire.id in
    Point.create ~id ~x:wire.x ~y:wire.y ~label:wire.label ()
  ;;

  let series_to_wire (t : Series.t) : Wire.Series.t =
    { id = Series_id.to_int64 (Series.id t)
    ; name = Series.name t
    ; points = List.map (Series.points t) ~f:point_to_wire
    }
  ;;

  let series_of_wire (wire : Wire.Series.t) =
    let%bind.Or_error id = Series_id.of_int64 wire.id in
    let%bind.Or_error points = convert wire.points ~f:point_of_wire in
    Series.create ~id ~name:wire.name points
  ;;

  let slice_to_wire (t : Slice.t) : Wire.Slice.t =
    { id = Datum_id.to_int64 (Slice.id t); label = Slice.label t; value = Slice.value t }
  ;;

  let slice_of_wire (wire : Wire.Slice.t) =
    let%bind.Or_error id = Datum_id.of_int64 wire.id in
    Slice.create ~id ~label:wire.label ~value:wire.value
  ;;

  let radar_axis_to_wire (t : Radar_axis.t) : Wire.Radar_axis.t =
    { id = Datum_id.to_int64 (Radar_axis.id t)
    ; label = Radar_axis.label t
    ; maximum = Radar_axis.maximum t
    }
  ;;

  let radar_axis_of_wire (wire : Wire.Radar_axis.t) =
    let%bind.Or_error id = Datum_id.of_int64 wire.id in
    Radar_axis.create ~id ~label:wire.label ~maximum:wire.maximum
  ;;

  let radar_series_to_wire (t : Radar_series.t) : Wire.Radar_series.t =
    { id = Series_id.to_int64 (Radar_series.id t)
    ; name = Radar_series.name t
    ; values =
        List.map (Radar_series.values t) ~f:(fun (id, value) ->
          Datum_id.to_int64 id, value)
    }
  ;;

  let radar_series_of_wire (wire : Wire.Radar_series.t) =
    let%bind.Or_error id = Series_id.of_int64 wire.id in
    let%bind.Or_error values =
      convert wire.values ~f:(fun (id, value) ->
        let%map.Or_error id = Datum_id.of_int64 id in
        id, value)
    in
    Radar_series.create ~id ~name:wire.name values
  ;;

  let candle_to_wire (t : Candle.t) : Wire.Candle.t =
    { id = Datum_id.to_int64 (Candle.id t)
    ; x = Candle.x t
    ; label = Candle.label t
    ; open_ = Candle.open_ t
    ; high = Candle.high t
    ; low = Candle.low t
    ; close = Candle.close t
    }
  ;;

  let candle_of_wire (wire : Wire.Candle.t) =
    let%bind.Or_error id = Datum_id.of_int64 wire.id in
    Candle.create
      ~id
      ~x:wire.x
      ~label:wire.label
      ~open_:wire.open_
      ~high:wire.high
      ~low:wire.low
      ~close:wire.close
  ;;

  let node_to_wire (t : Node.t) : Wire.Node.t =
    { id = Node_id.to_int64 (Node.id t); label = Node.label t }
  ;;

  let node_of_wire (wire : Wire.Node.t) =
    let%bind.Or_error id = Node_id.of_int64 wire.id in
    Node.create ~id ~label:wire.label
  ;;

  let edge_to_wire (t : Edge.t) : Wire.Edge.t =
    { id = Edge_id.to_int64 (Edge.id t)
    ; source = Node_id.to_int64 (Edge.source t)
    ; target = Node_id.to_int64 (Edge.target t)
    ; value = Edge.value t
    }
  ;;

  let edge_of_wire (wire : Wire.Edge.t) =
    let%bind.Or_error id = Edge_id.of_int64 wire.id in
    let%bind.Or_error source = Node_id.of_int64 wire.source in
    let%bind.Or_error target = Node_id.of_int64 wire.target in
    Edge.create ~id ~source ~target ~value:wire.value
  ;;

  let layer_to_wire = function
    | Layer.Line series -> Wire.Layer.Line (series_to_wire series)
    | Layer.Area series -> Wire.Layer.Area (series_to_wire series)
    | Layer.Bar series -> Wire.Layer.Bar (series_to_wire series)
  ;;

  let layer_of_wire = function
    | Wire.Layer.Line series ->
      Or_error.map (series_of_wire series) ~f:(fun s -> Layer.Line s)
    | Wire.Layer.Area series ->
      Or_error.map (series_of_wire series) ~f:(fun s -> Layer.Area s)
    | Wire.Layer.Bar series ->
      Or_error.map (series_of_wire series) ~f:(fun s -> Layer.Bar s)
  ;;

  let category_to_wire (t : Category.t) : Wire.Category.t =
    { id = Category_id.to_int64 (Category.id t); label = Category.label t }
  ;;

  let category_of_wire (w : Wire.Category.t) =
    let%bind.Or_error id = Category_id.of_int64 w.id in
    Category.create ~id ~label:w.label
  ;;

  let categorical_point_to_wire (t : Categorical_point.t) : Wire.Categorical_point.t =
    { id = Datum_id.to_int64 (Categorical_point.id t)
    ; category = Category_id.to_int64 (Categorical_point.category t)
    ; value = Categorical_point.value t
    ; label = Categorical_point.label t
    }
  ;;

  let categorical_point_of_wire (w : Wire.Categorical_point.t) =
    let%bind.Or_error id = Datum_id.of_int64 w.id in
    let%bind.Or_error category = Category_id.of_int64 w.category in
    Categorical_point.create ~id ~category ~value:w.value ~label:w.label ()
  ;;

  let categorical_series_to_wire (t : Categorical_series.t) : Wire.Categorical_series.t =
    { id = Series_id.to_int64 (Categorical_series.id t)
    ; name = Categorical_series.name t
    ; points = List.map (Categorical_series.points t) ~f:categorical_point_to_wire
    }
  ;;

  let categorical_series_of_wire (w : Wire.Categorical_series.t) =
    let%bind.Or_error id = Series_id.of_int64 w.id in
    let%bind.Or_error points = convert w.points ~f:categorical_point_of_wire in
    Categorical_series.create ~id ~name:w.name points
  ;;

  let categorical_layer_to_wire = function
    | Categorical_layer.Line s ->
      Wire.Categorical_layer.Line (categorical_series_to_wire s)
    | Area s -> Wire.Categorical_layer.Area (categorical_series_to_wire s)
    | Bar s -> Wire.Categorical_layer.Bar (categorical_series_to_wire s)
  ;;

  let categorical_layer_of_wire = function
    | Wire.Categorical_layer.Line s ->
      Or_error.map (categorical_series_of_wire s) ~f:(fun s -> Categorical_layer.Line s)
    | Area s ->
      Or_error.map (categorical_series_of_wire s) ~f:(fun s -> Categorical_layer.Area s)
    | Bar s ->
      Or_error.map (categorical_series_of_wire s) ~f:(fun s -> Categorical_layer.Bar s)
  ;;

  let to_wire t =
    let contents =
      match t.contents with
      | Cartesian layers -> Wire.Contents.Cartesian (List.map layers ~f:layer_to_wire)
      | Categorical (categories, layers) ->
        Wire.Contents.Categorical
          ( List.map categories ~f:category_to_wire
          , List.map layers ~f:categorical_layer_to_wire )
      | Pie slices -> Wire.Contents.Pie (List.map slices ~f:slice_to_wire)
      | Radar (axes, series) ->
        Wire.Contents.Radar
          (List.map axes ~f:radar_axis_to_wire, List.map series ~f:radar_series_to_wire)
      | Candlestick candles ->
        Wire.Contents.Candlestick (List.map candles ~f:candle_to_wire)
      | Sankey (nodes, edges) ->
        Wire.Contents.Sankey
          (List.map nodes ~f:node_to_wire, List.map edges ~f:edge_to_wire)
    in
    { Wire.version = 3L
    ; contents
    ; bar_backgrounds = t.bar_backgrounds
    ; bar_baselines = t.bar_baselines
    }
  ;;

  let of_wire (wire : Wire.t) =
    let%bind.Or_error () =
      require (Int64.equal wire.version 3L) "unsupported chart data version"
    in
    let%bind.Or_error () =
      require (Wire.within_bounds wire) "chart wire envelope exceeds its resource bounds"
    in
    let%bind.Or_error t =
      match wire.contents with
      | Cartesian layers ->
        let%bind.Or_error layers = convert layers ~f:layer_of_wire in
        cartesian layers
      | Categorical (categories, layers) ->
        let%bind.Or_error categories = convert categories ~f:category_of_wire in
        let%bind.Or_error layers = convert layers ~f:categorical_layer_of_wire in
        categorical ~categories layers
      | Pie slices ->
        let%bind.Or_error slices = convert slices ~f:slice_of_wire in
        pie slices
      | Radar (axes, series) ->
        let%bind.Or_error axes = convert axes ~f:radar_axis_of_wire in
        let%bind.Or_error series = convert series ~f:radar_series_of_wire in
        radar ~axes series
      | Candlestick candles ->
        let%bind.Or_error candles = convert candles ~f:candle_of_wire in
        candlestick candles
      | Sankey (nodes, edges) ->
        let%bind.Or_error nodes = convert nodes ~f:node_of_wire in
        let%bind.Or_error edges = convert edges ~f:edge_of_wire in
        sankey ~nodes ~edges
    in
    let%bind.Or_error () = validate_backgrounds t wire.bar_backgrounds in
    let%map.Or_error () = validate_baselines t wire.bar_baselines in
    { t with bar_backgrounds = wire.bar_backgrounds; bar_baselines = wire.bar_baselines }
  ;;

  let encode t =
    let wire = to_wire t in
    let%map.Or_error () =
      require (Wire.bin_size_t wire <= Wire.max_bytes) "chart data exceeds 16 MiB"
    in
    Bin_prot.Utils.bin_dump Wire.bin_writer_t wire |> Bigstring.to_string
  ;;

  let decode bytes =
    let%bind.Or_error wire = Wire.decode bytes in
    of_wire wire
  ;;
end

let with_bar_backgrounds t ?(theme = Theme.default) backgrounds =
  let module W = Gpuio_protocol.Chart_data_wire in
  let%bind.Or_error () =
    require (List.length backgrounds <= max_points) "chart background limit exceeded"
  in
  let%bind.Or_error bar_backgrounds =
    List.map backgrounds ~f:(fun (b : Bar_background.t) ->
      let%map.Or_error brush = Chart_brush.resolve b.background ~theme in
      { W.Bar_background.series = Series_id.to_int64 b.series
      ; datum = Datum_id.to_int64 b.datum
      ; brush
      })
    |> Or_error.all
  in
  let bar_backgrounds =
    List.sort bar_backgrounds ~compare:(fun a b ->
      Bar_key.compare (background_key a) (background_key b))
  in
  let%bind.Or_error () = validate_backgrounds t bar_backgrounds in
  let result = { t with bar_backgrounds } in
  let%map.Or_error () =
    require
      (W.bin_size_t (Expert.to_wire result) <= W.max_bytes)
      "chart data exceeds 16 MiB"
  in
  result
;;

let with_bar_baselines t entries =
  let module W = Gpuio_protocol.Chart_data_wire in
  let%bind.Or_error () =
    require (List.length entries <= max_points) "chart baseline limit exceeded"
  in
  let bar_baselines =
    List.map entries ~f:(fun (b : Bar_baseline.t) ->
      { W.Bar_baseline.series = Series_id.to_int64 b.series
      ; datum = Datum_id.to_int64 b.datum
      ; baseline = b.baseline
      })
    |> List.sort ~compare:(fun a b -> Bar_key.compare (baseline_key a) (baseline_key b))
  in
  let%bind.Or_error () = validate_baselines t bar_baselines in
  let result = { t with bar_baselines } in
  let%map.Or_error () =
    require
      (W.bin_size_t (Expert.to_wire result) <= W.max_bytes)
      "chart data exceeds 16 MiB"
  in
  result
;;
