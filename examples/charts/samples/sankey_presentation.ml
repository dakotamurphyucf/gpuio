open Core
module D = Gpuio.Chart_data
module O = Gpuio.Chart_options

let ok = Or_error.ok_exn
let node_id n = D.Node_id.of_int64 n |> ok
let edge_id n = D.Edge_id.of_int64 n |> ok

type t =
  | Default
  | Rounded
  | Muted
  | Minimum
  | Spaced
  | Rich_labels
  | Hidden_target
  | Gradient
  | Target
[@@deriving equal]

let all =
  [ Default
  ; Rounded
  ; Muted
  ; Minimum
  ; Spaced
  ; Rich_labels
  ; Hidden_target
  ; Gradient
  ; Target
  ]
;;

let label = function
  | Default -> "Default flows"
  | Rounded -> "Rounded nodes"
  | Muted -> "Muted ribbons"
  | Minimum -> "Visible small flows"
  | Spaced -> "Spaced flow labels"
  | Rich_labels -> "Rich flow labels"
  | Hidden_target -> "Hide target label"
  | Gradient -> "Gradient flows"
  | Target -> "Target-colored flows"
;;

let options ?(label_placement = O.Sankey.Label_placement.Inside) ?(labels = true) preset =
  let create = O.Sankey.create ~label_placement ~labels in
  match preset with
  | Default | Rich_labels | Hidden_target -> create () |> ok
  | Rounded -> create ~node_corner_radius:8. () |> ok
  | Muted -> create ~link_opacity:0.12 () |> ok
  | Minimum -> create ~min_link_width:12. () |> ok
  | Spaced -> create ~label_gap:32. () |> ok
  | Gradient -> create ~link_color:Gradient () |> ok
  | Target -> create ~link_color:Target () |> ok
;;

let node_labels ?(middle = false) ?(long = false) preset =
  let rich = equal preset Rich_labels || equal preset Hidden_target in
  if not (rich || long)
  then Gpuio.Chart_node_labels.empty
  else
    let module L = Gpuio.Chart_node_labels in
    let line ?color ?font_size text = L.Line.create ?color ?font_size text |> ok in
    let entry id lines = L.Node.create ~node:(node_id id) lines |> ok in
    let caption short =
      if long then short ^ " · 日本語 λ · a deliberately long caption" else short
    in
    let lines ~color ~size title detail =
      if rich
      then
        [ line ~color:(Gpuio.Color.rgb_exn color) ~font_size:size (caption title)
        ; line detail
        ]
      else [ line (caption title) ]
    in
    L.create
      ([ entry 10L (lines ~color:0x2dd4bf ~size:22. "Intake" "Recorded activity")
       ; entry
           20L
           (if equal preset Hidden_target
            then []
            else lines ~color:0xfbbf24 ~size:18. "Outcome" "Delivered locally")
       ]
       @
       if middle
       then [ entry 30L (lines ~color:0xc4b5fd ~size:20. "Processing" "Step λ") ]
       else [])
    |> ok
;;

let data_exn phase =
  if (not (Float.is_finite phase)) || Float.(phase < 0. || phase > 1000.)
  then invalid_arg "Sankey presentation sample phase must be finite and in [0,1000]";
  let nodes =
    [ D.Node.create ~id:(node_id 10L) ~label:"Input" |> ok
    ; D.Node.create ~id:(node_id 20L) ~label:"Output" |> ok
    ]
  in
  let edges =
    List.map
      [ 1L, 100. +. phase; 2L, 0.01; 3L, 0. ]
      ~f:(fun (key, value) ->
        D.Edge.create ~id:(edge_id key) ~source:(node_id 10L) ~target:(node_id 20L) ~value
        |> ok)
  in
  D.sankey ~nodes ~edges |> ok
;;

let placement_data_exn phase =
  if (not (Float.is_finite phase)) || Float.(phase < 0. || phase > 1000.)
  then invalid_arg "Sankey placement sample phase must be finite and in [0,1000]";
  let nodes =
    List.map
      [ 30L, "Processing"; 20L, "Output"; 10L, "Input" ]
      ~f:(fun (id, label) -> D.Node.create ~id:(node_id id) ~label |> ok)
  in
  let edges =
    List.map
      [ 1L, 10L, 30L, 100. +. phase
      ; 2L, 10L, 30L, 0.01
      ; 3L, 10L, 30L, 0.
      ; 4L, 30L, 20L, 100.01 +. phase
      ]
      ~f:(fun (id, source, target, value) ->
        D.Edge.create
          ~id:(edge_id id)
          ~source:(node_id source)
          ~target:(node_id target)
          ~value
        |> ok)
  in
  D.sankey ~nodes ~edges |> ok
;;
