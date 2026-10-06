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

let options = function
  | Default | Rich_labels | Hidden_target -> O.Sankey.default
  | Rounded -> O.Sankey.create ~node_corner_radius:8. () |> ok
  | Muted -> O.Sankey.create ~link_opacity:0.12 () |> ok
  | Minimum -> O.Sankey.create ~min_link_width:12. () |> ok
  | Spaced -> O.Sankey.create ~label_gap:32. () |> ok
  | Gradient -> O.Sankey.create ~link_color:Gradient () |> ok
  | Target -> O.Sankey.create ~link_color:Target () |> ok
;;

let node_labels = function
  | Default | Rounded | Muted | Minimum | Spaced | Gradient | Target ->
    Gpuio.Chart_node_labels.empty
  | (Rich_labels | Hidden_target) as preset ->
    let module L = Gpuio.Chart_node_labels in
    let line ?color ?font_size text = L.Line.create ?color ?font_size text |> ok in
    let entry id lines = L.Node.create ~node:(node_id id) lines |> ok in
    L.create
      [ entry
          10L
          [ line ~color:(Gpuio.Color.rgb_exn 0x2dd4bf) ~font_size:22. "Intake"
          ; line "Recorded activity"
          ]
      ; entry
          20L
          (match preset with
           | Hidden_target -> []
           | _ ->
             [ line ~color:(Gpuio.Color.rgb_exn 0xfbbf24) ~font_size:18. "Outcome"
             ; line "Delivered locally"
             ])
      ]
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
