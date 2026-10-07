open Core

(* Owned raw schema. Bounded decoding validates structure before allocation;
   domain semantics must additionally pass Chart_data.Expert.of_wire. *)
let max_bytes = 16 * 1024 * 1024
let max_text_bytes = 8 * 1024 * 1024
let max_points = 100_000
let max_series = 32

module Point = struct
  type t =
    { id : int64
    ; x : float
    ; y : float option
    ; label : string
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Series = struct
  type t =
    { id : int64
    ; name : string
    ; points : Point.t list
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Slice = struct
  type t =
    { id : int64
    ; label : string
    ; value : float
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Radar_axis = struct
  type t =
    { id : int64
    ; label : string
    ; maximum : float
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Radar_series = struct
  type t =
    { id : int64
    ; name : string
    ; values : (int64 * float) list
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Candle = struct
  type t =
    { id : int64
    ; x : float
    ; label : string
    ; open_ : float
    ; high : float
    ; low : float
    ; close : float
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Node = struct
  type t =
    { id : int64
    ; label : string
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Edge = struct
  type t =
    { id : int64
    ; source : int64
    ; target : int64
    ; value : float
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Layer = struct
  type t =
    | Line of Series.t
    | Area of Series.t
    | Bar of Series.t
  [@@deriving bin_io, equal, sexp_of]
end

module Category = struct
  type t =
    { id : int64
    ; label : string
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Categorical_point = struct
  type t =
    { id : int64
    ; category : int64
    ; value : float option
    ; label : string
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Categorical_series = struct
  type t =
    { id : int64
    ; name : string
    ; points : Categorical_point.t list
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Categorical_layer = struct
  type t =
    | Line of Categorical_series.t
    | Area of Categorical_series.t
    | Bar of Categorical_series.t
  [@@deriving bin_io, equal, sexp_of]
end

module Contents = struct
  type t =
    | Cartesian of Layer.t list
    | Pie of Slice.t list
    | Radar of Radar_axis.t list * Radar_series.t list
    | Candlestick of Candle.t list
    | Sankey of Node.t list * Edge.t list
    | Categorical of Category.t list * Categorical_layer.t list
  [@@deriving bin_io, equal, sexp_of]
end

module Bar_background = struct
  type t =
    { series : int64
    ; datum : int64
    ; brush : Chart_appearance_wire.Brush.t
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Bar_baseline = struct
  type t =
    { series : int64
    ; datum : int64
    ; baseline : float
    }
  [@@deriving bin_io, equal, sexp_of]
end

type t =
  { version : int64
  ; contents : Contents.t
  ; bar_backgrounds : Bar_background.t list
  ; bar_baselines : Bar_baseline.t list
  }
[@@deriving bin_io, equal, sexp_of]

(* Do not use generated list readers for an untrusted dataset. This reader
   enforces aggregate budgets before list allocation and text copying. *)
let bin_read_t buffer ~pos_ref =
  let fail () = invalid_arg "invalid or oversized chart data" in
  let remaining () = Bigstring.length buffer - !pos_ref in
  if remaining () > max_bytes then fail ();
  let text_remaining = ref max_text_bytes in
  let points_remaining = ref max_points in
  let int () = Bin_prot.Read.bin_read_int64 buffer ~pos_ref in
  let float () = Bin_prot.Read.bin_read_float buffer ~pos_ref in
  let tag () = Bin_prot.Read.bin_read_int_8bit buffer ~pos_ref in
  let count maximum =
    let count = (Bin_prot.Read.bin_read_nat0 buffer ~pos_ref :> int) in
    if count > maximum || count > remaining () then fail ();
    count
  in
  let read_count count read =
    let rec loop remaining acc =
      if remaining = 0
      then List.rev acc
      else (
        let value = read () in
        loop (remaining - 1) (value :: acc))
    in
    loop count []
  in
  let list maximum read = read_count (count maximum) read in
  let text maximum =
    let start = !pos_ref in
    let length = count (Int.min maximum !text_remaining) in
    text_remaining := !text_remaining - length;
    pos_ref := start;
    let value = Bin_prot.Read.bin_read_string buffer ~pos_ref in
    if not (Stdlib.String.is_valid_utf_8 value) then fail ();
    value
  in
  let option read =
    match tag () with
    | 0 -> None
    | 1 -> Some (read ())
    | _ -> fail ()
  in
  let point () =
    let id = int () in
    let x = float () in
    let y = option float in
    let label = text 256 in
    { Point.id; x; y; label }
  in
  let series () =
    let id = int () in
    let name = text 128 in
    let count = count !points_remaining in
    points_remaining := !points_remaining - count;
    let points = read_count count point in
    { Series.id; name; points }
  in
  let slice () =
    let id = int () in
    let label = text 256 in
    let value = float () in
    { Slice.id; label; value }
  in
  let radar_axis () =
    let id = int () in
    let label = text 256 in
    let maximum = float () in
    { Radar_axis.id; label; maximum }
  in
  let radar_series () =
    let id = int () in
    let name = text 128 in
    let values =
      list 64 (fun () ->
        let id = int () in
        let value = float () in
        id, value)
    in
    { Radar_series.id; name; values }
  in
  let candle () =
    let id = int () in
    let x = float () in
    let label = text 256 in
    let open_ = float () in
    let high = float () in
    let low = float () in
    let close = float () in
    { Candle.id; x; label; open_; high; low; close }
  in
  let node () =
    let id = int () in
    let label = text 256 in
    { Node.id; label }
  in
  let edge () =
    let id = int () in
    let source = int () in
    let target = int () in
    let value = float () in
    { Edge.id; source; target; value }
  in
  let layer () =
    match tag () with
    | 0 -> Layer.Line (series ())
    | 1 -> Layer.Area (series ())
    | 2 -> Layer.Bar (series ())
    | _ -> fail ()
  in
  let categorical_layer () =
    let kind = tag () in
    if kind > 2 then fail ();
    let id = int () in
    let name = text 128 in
    let count = count !points_remaining in
    points_remaining := !points_remaining - count;
    let points =
      read_count count (fun () ->
        let id = int () in
        let category = int () in
        let value = option float in
        let label = text 256 in
        { Categorical_point.id; category; value; label })
    in
    let series = { Categorical_series.id; name; points } in
    match kind with
    | 0 -> Categorical_layer.Line series
    | 1 -> Categorical_layer.Area series
    | 2 -> Categorical_layer.Bar series
    | _ -> fail ()
  in
  let version = int () in
  if not (Int64.equal version 3L) then fail ();
  let contents =
    match tag () with
    | 0 -> Contents.Cartesian (list max_series layer)
    | 1 -> Contents.Pie (list 256 slice)
    | 2 ->
      let axes = list 64 radar_axis in
      let series = list max_series radar_series in
      Contents.Radar (axes, series)
    | 3 -> Contents.Candlestick (list max_points candle)
    | 4 ->
      let nodes = list 256 node in
      let edges = list 2048 edge in
      Contents.Sankey (nodes, edges)
    | 5 ->
      let categories =
        list max_points (fun () ->
          let id = int () in
          let label = text 256 in
          { Category.id; label })
      in
      let layers = list max_series categorical_layer in
      Contents.Categorical (categories, layers)
    | _ -> fail ()
  in
  let bar_backgrounds =
    list max_points (fun () ->
      let series = int () in
      let datum = int () in
      let brush = Chart_appearance_wire.Brush.bin_read_t buffer ~pos_ref in
      if not (Chart_appearance_wire.Brush.valid brush) then fail ();
      { Bar_background.series; datum; brush })
  in
  let bar_baselines =
    list max_points (fun () ->
      let series = int () in
      let datum = int () in
      let baseline = float () in
      if not (Float.is_finite baseline && Float.(abs baseline <= 1e100)) then fail ();
      { Bar_baseline.series; datum; baseline })
  in
  { version; contents; bar_backgrounds; bar_baselines }
;;

let bin_reader_t = { bin_reader_t with read = bin_read_t }
let bin_t = { bin_t with reader = bin_reader_t }

let decode bytes =
  if String.length bytes > max_bytes
  then Or_error.error_string "chart data exceeds 16 MiB"
  else
    Or_error.try_with (fun () ->
      let buffer = Bigstring.of_string bytes in
      let pos_ref = ref 0 in
      let t = bin_read_t buffer ~pos_ref in
      if !pos_ref <> String.length bytes then invalid_arg "trailing chart data";
      t)
;;

(* In-memory raw callers also pass the aggregate envelope before the Core
   adapter constructs a second set of domain records. This is structural only. *)
let within_bounds t =
  let text_remaining = ref max_text_bytes in
  let points_remaining = ref max_points in
  let text maximum value =
    let length = String.length value in
    if length > maximum || length > !text_remaining
    then false
    else (
      text_remaining := !text_remaining - length;
      true)
  in
  let points values =
    let count = List.length values in
    if count > !points_remaining
    then false
    else (
      points_remaining := !points_remaining - count;
      true)
  in
  List.length t.bar_baselines <= max_points
  && List.for_all t.bar_baselines ~f:(fun b ->
    Float.is_finite b.Bar_baseline.baseline && Float.(abs b.baseline <= 1e100))
  && List.length t.bar_backgrounds <= max_points
  && List.for_all t.bar_backgrounds ~f:(fun b ->
    Chart_appearance_wire.Brush.valid b.Bar_background.brush)
  && (match t.contents with
      | Contents.Cartesian layers ->
        List.length layers <= max_series
        && List.for_all layers ~f:(fun layer ->
          let series =
            match layer with
            | Layer.Line s | Area s | Bar s -> s
          in
          points series.points
          && text 128 series.name
          && List.for_all series.points ~f:(fun p -> text 256 p.Point.label))
      | Categorical (categories, layers) ->
        List.length categories <= max_points
        && List.length layers <= max_series
        && List.for_all categories ~f:(fun c -> text 256 c.Category.label)
        && List.for_all layers ~f:(fun layer ->
          let series =
            match layer with
            | Categorical_layer.Line s | Area s | Bar s -> s
          in
          points series.points
          && text 128 series.name
          && List.for_all series.points ~f:(fun p -> text 256 p.Categorical_point.label))
      | Pie slices ->
        List.length slices <= 256
        && List.for_all slices ~f:(fun s -> text 256 s.Slice.label)
      | Radar (axes, series) ->
        List.length axes <= 64
        && List.length series <= max_series
        && List.for_all axes ~f:(fun a -> text 256 a.Radar_axis.label)
        && List.for_all series ~f:(fun s ->
          List.length s.Radar_series.values <= 64 && text 128 s.name)
      | Candlestick candles ->
        List.length candles <= max_points
        && List.for_all candles ~f:(fun c -> text 256 c.Candle.label)
      | Sankey (nodes, edges) ->
        List.length nodes <= 256
        && List.length edges <= 2048
        && List.for_all nodes ~f:(fun n -> text 256 n.Node.label))
  && bin_size_t t <= max_bytes
;;
