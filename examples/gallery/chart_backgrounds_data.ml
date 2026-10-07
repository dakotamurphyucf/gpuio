open Core
open Gpuio
module D = Chart_data

let ok = Or_error.ok_exn

module Baselines = struct
  type t =
    | Zero
    | Shared
    | Individual
  [@@deriving equal]

  let all = [ Zero; Shared; Individual ]

  let label = function
    | Zero -> "Zero baselines"
    | Shared -> "Shared baseline 40"
    | Individual -> "Individual baselines"
  ;;
end

type t =
  { phase : int
  ; reordered : bool
  ; patterns : bool
  ; baselines : Baselines.t
  }
[@@deriving equal]

let initial =
  { phase = 0; reordered = false; patterns = false; baselines = Baselines.Zero }
;;

let advance t = { t with phase = (t.phase + 1) % 13 }
let reorder t = { t with reordered = not t.reordered }
let toggle_patterns t = { t with patterns = not t.patterns }
let phase t = t.phase
let is_reordered t = t.reordered
let has_patterns t = t.patterns
let baselines t = t.baselines
let with_baselines t baselines = { t with baselines }
let series_id = D.Series_id.of_int64 7L |> ok
let datum_id index = D.Datum_id.of_int64 (Int64.of_int (240 - (index * 7))) |> ok
let first_id = datum_id 0
let category_id index = D.Category_id.of_int64 (Int64.of_int (101 + index)) |> ok

let background t index =
  let color =
    Color.token_exn (List.nth_exn [ "accent"; "muted"; "foreground" ] (index % 3))
  in
  if not t.patterns
  then Background.solid color
  else (
    match index % 3 with
    | 0 -> Background.solid color
    | 1 -> Background.pattern_slash color ~width:2. ~interval:4. |> ok
    | _ -> Background.checkerboard color ~size:5. |> ok)
;;

let data_exn t ~theme =
  let indices = List.init 24 ~f:Fn.id in
  let order = if t.reordered then List.rev indices else indices in
  let categories =
    List.map order ~f:(fun index ->
      D.Category.create ~id:(category_id index) ~label:(sprintf "Batch %02d" (index + 1))
      |> ok)
  in
  let points =
    List.map order ~f:(fun index ->
      D.Categorical_point.create
        ~id:(datum_id index)
        ~category:(category_id index)
        ~value:(Some (Float.of_int (30 + (3 * t.phase) + (index * 11 % 47))))
        ()
      |> ok)
  in
  let series =
    D.Categorical_series.create ~id:series_id ~name:"Evaluations" points |> ok
  in
  let data = D.categorical ~categories [ Bar series ] |> ok in
  (* Deliberately keep descriptor order independent of the current category order.
     The constructor associates and canonicalizes by typed identity. *)
  let backgrounds =
    List.map indices ~f:(fun index ->
      D.Bar_background.create
        ~series:series_id
        ~datum:(datum_id index)
        (background t index))
  in
  let data = D.with_bar_backgrounds data ~theme backgrounds |> ok in
  let baselines =
    List.filter_map indices ~f:(fun index ->
      let baseline =
        match t.baselines with
        | Baselines.Zero -> None
        | Shared -> Some 40.
        | Individual -> Some (Float.of_int (20 + (20 * (index % 3))))
      in
      Option.map baseline ~f:(fun base ->
        D.Bar_baseline.create ~series:series_id ~datum:(datum_id index) base |> ok))
  in
  D.with_bar_baselines data baselines |> ok
;;
