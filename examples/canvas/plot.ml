open Core
module G = Gpuio.Canvas_geometry
module S = Gpuio.Canvas_scene
module R = Gpuio.Canvas_resource

let point x y = G.Point.create ~x ~y |> Or_error.ok_exn
let rect x y width height = G.Rect.create ~x ~y ~width ~height |> Or_error.ok_exn
let color = Gpuio.Color.rgb_exn
let item_id n = S.Item_id.of_int64 (Int64.of_int n) |> Or_error.ok_exn
let resource_id n = R.Id.of_int64 (Int64.of_int n) |> Or_error.ok_exn
let paint value = S.Paint.create ~fill:(color value) () |> Or_error.ok_exn

module Sample = struct
  type t =
    { id : S.Item_id.t
    ; name : string
    ; color : Gpuio.Color.t
    ; transform : G.Transform.t
    }

  let id t = t.id
  let name t = t.name
  let color t = t.color
  let position t = G.Transform.apply t.transform (point 0. 0.) |> Or_error.ok_exn
end

type t =
  { samples : Sample.t list
  ; large : bool
  }

let create ~large =
  { large
  ; samples =
      List.mapi
        [ "Swift", 180., 320., 0x65dec1
        ; "Sage", 370., 210., 0xa797ff
        ; "Atlas", 510., 100., 0xf9c36b
        ; "Orbit", 250., 160., 0xf493b6
        ]
        ~f:(fun index (name, x, y, rgb) ->
          { Sample.id = item_id (index + 1)
          ; name
          ; color = color rgb
          ; transform = G.Transform.translate ~x ~y |> Or_error.ok_exn
          })
  }
;;

let samples t = t.samples

let find t id =
  List.find t.samples ~f:(fun sample -> S.Item_id.equal (Sample.id sample) id)
;;

let move t id transform =
  match find t id with
  | None -> Or_error.error_string "The selected sample is no longer present"
  | Some _ ->
    Ok
      { t with
        samples =
          List.map t.samples ~f:(fun sample ->
            if S.Item_id.equal (Sample.id sample) id
            then { sample with transform }
            else sample)
      }
;;

let scene t =
  let clipping = rect 64. 64. 580. 340. in
  let shape id shape = S.Item.create ~id:(item_id id) shape |> Or_error.ok_exn in
  let axes =
    List.init 7 ~f:(fun index ->
      let x = 64. +. (Float.of_int index *. (580. /. 6.)) in
      let y = 64. +. (Float.of_int index *. (340. /. 6.)) in
      [ shape
          (10 + index)
          (S.Drawing.rectangle (rect x 64. 0.7 340.) ~paint:(paint 0x26303f))
      ; shape
          (20 + index)
          (S.Drawing.rectangle (rect 64. y 580. 0.7) ~paint:(paint 0x26303f))
      ])
    |> List.concat
  in
  let text id x y size value =
    let resource = R.text ~id:(resource_id id) ~font_size:size value |> Or_error.ok_exn in
    S.Item.create
      ~id:(item_id id)
      (S.Drawing.text resource ~origin:(point x y) ~color:(color 0x99a9bf))
    |> Or_error.ok_exn
  in
  let labels =
    [ text 30 64. 18. 12. "QUALITY SCORE"
    ; text 31 535. 427. 12. "LATENCY →"
    ; text 32 20. 64. 12. "100"
    ; text 33 32. 386. 12. "0"
    ; text 34 64. 427. 12. "0 ms"
    ; text 35 301. 427. 12. "500 ms"
    ]
  in
  let cloud =
    List.init
      (if t.large then 19_000 else 160)
      ~f:(fun index ->
        let x = 72. +. Float.of_int (((index * 131) + 7) mod 565) in
        let y = 72. +. Float.of_int (((index * 71) + 11) mod 324) in
        let transform = G.Transform.translate ~x ~y |> Or_error.ok_exn in
        S.Item.create
          ~id:(item_id (100 + index))
          ~transform
          ~clips:[ clipping ]
          (S.Drawing.ellipse (rect (-1.5) (-1.5) 3. 3.) ~paint:(paint 0x344355))
        |> Or_error.ok_exn)
  in
  let nodes =
    List.map t.samples ~f:(fun sample ->
      let bounds = rect (-10.) (-10.) 20. 20. in
      let interaction =
        S.Interaction.create
          ~label:sample.name
          ~hit_region:(G.Hit_region.ellipse bounds)
          ~draggable:true
          ~activatable:true
          ()
        |> Or_error.ok_exn
      in
      let stroke =
        S.Stroke.create ~color:(color 0xe6edf7) ~width:1.5 |> Or_error.ok_exn
      in
      let paint = S.Paint.create ~fill:sample.color ~stroke () |> Or_error.ok_exn in
      S.Item.create
        ~id:sample.id
        ~transform:sample.transform
        ~clips:[ clipping ]
        ~interaction
        (S.Drawing.ellipse bounds ~paint)
      |> Or_error.ok_exn)
  in
  S.create
    ~description:
      ("Simulated model evaluation: horizontal latency, vertical quality. Four labelled \
        samples can be selected and dragged. Background points are decorative fixtures. \
        Sample positions: "
       ^ String.concat
           ~sep:"; "
           (List.map t.samples ~f:(fun sample ->
              let p = Sample.position sample in
              sprintf "%s at x %.1f, y %.1f" sample.name (G.Point.x p) (G.Point.y p))))
    (axes @ labels @ cloud @ nodes)
  |> Or_error.ok_exn
;;
