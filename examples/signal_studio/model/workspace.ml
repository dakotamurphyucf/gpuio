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
  let latency t = (G.Point.x (position t) -. 64.) *. 1000. /. 580.
  let quality t = (404. -. G.Point.y (position t)) *. 100. /. 340.
end

type t =
  { samples : Sample.t list
  ; run : int
  ; selected : S.Item_id.t option
  }

let create () =
  { run = 0
  ; selected = None
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

let run t = t.run

let set_run t run =
  if run < 0 || run > 100
  then Or_error.error_string "Run must be 0..100"
  else Ok { t with run }
;;

let selected t = Option.bind t.selected ~f:(find t)

let select t selected =
  match selected with
  | Some id when Option.is_none (find t id) -> Or_error.error_string "Unknown sample"
  | None | Some _ -> Ok { t with selected }
;;

let move t id position =
  match find t id with
  | None -> Or_error.error_string "Unknown sample"
  | Some _ ->
    let transform =
      G.Transform.translate
        ~x:(Float.clamp_exn (G.Point.x position) ~min:74. ~max:634.)
        ~y:(Float.clamp_exn (G.Point.y position) ~min:74. ~max:394.)
      |> Or_error.ok_exn
    in
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
    ; text 35 354. 427. 12. "500 ms"
    ]
  in
  let cloud =
    List.init 160 ~f:(fun index ->
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
  let names =
    List.mapi t.samples ~f:(fun index sample ->
      let p = Sample.position sample in
      text (30_000 + index) (G.Point.x p +. 15.) (G.Point.y p -. 7.) 12. sample.name)
  in
  S.create
    ~description:
      "Simulated model evaluation: horizontal latency, vertical quality. Four labelled \
       samples can be selected and dragged. Background points are decorative fixtures."
    (axes @ labels @ cloud @ nodes @ names)
  |> Or_error.ok_exn
;;

let chart t =
  let module D = Gpuio.Chart_data in
  D.line
    (List.mapi t.samples ~f:(fun index sample ->
       D.Series.create
         ~id:(D.Series_id.of_int64 (Int64.of_int (index + 1)) |> Or_error.ok_exn)
         ~name:sample.name
         (List.init 24 ~f:(fun i ->
            let phase = Float.of_int (i + t.run) /. 4. in
            let latency =
              Sample.latency sample +. (15. *. Float.sin (phase +. Float.of_int index))
            in
            D.Point.create
              ~id:(D.Datum_id.of_int64 (Int64.of_int (i + 1)) |> Or_error.ok_exn)
              ~x:(Float.of_int i)
              ~y:(Some latency)
              ()
            |> Or_error.ok_exn))
       |> Or_error.ok_exn))
  |> Or_error.ok_exn
;;

module Document = struct
  module Position = struct
    type t =
      { id : int64
      ; x : float
      ; y : float
      }
    [@@deriving sexp]
  end

  type t =
    { version : int
    ; run : int
    ; selected : int64 option
    ; positions : Position.t list
    }
  [@@deriving sexp]
end

let encode t =
  let document : Document.t =
    { version = 1
    ; run = t.run
    ; selected = Option.map t.selected ~f:S.Item_id.to_int64
    ; positions =
        List.map t.samples ~f:(fun sample ->
          let position = Sample.position sample in
          { Document.Position.id = S.Item_id.to_int64 sample.id
          ; x = G.Point.x position
          ; y = G.Point.y position
          })
    }
  in
  Sexp.to_string_hum (Document.sexp_of_t document)
;;

let decode text =
  let open Or_error.Let_syntax in
  if String.length text > 16 * 1024
  then Or_error.error_string "Document exceeds 16 KiB"
  else (
    let%bind document =
      Or_error.try_with (fun () -> Document.t_of_sexp (Sexp.of_string text))
    in
    if document.version <> 1
    then Or_error.error_string "Unsupported document version"
    else (
      let%bind initial = set_run (create ()) document.run in
      let ids =
        List.map document.positions ~f:(fun position -> position.Document.Position.id)
        |> List.sort ~compare:Int64.compare
      in
      if not (List.equal Int64.equal ids [ 1L; 2L; 3L; 4L ])
      then Or_error.error_string "Document must contain each sample exactly once"
      else (
        let%bind t =
          List.fold_result document.positions ~init:initial ~f:(fun t position ->
            let%bind point = G.Point.create ~x:position.x ~y:position.y in
            if
              Float.(
                position.x < 74.
                || position.x > 634.
                || position.y < 74.
                || position.y > 394.)
            then Or_error.error_string "Document sample is outside the plot"
            else (
              let%bind id = S.Item_id.of_int64 position.id in
              move t id point))
        in
        let%bind selected =
          Option.value_map document.selected ~default:(Ok None) ~f:(fun id ->
            S.Item_id.of_int64 id |> Or_error.map ~f:Option.some)
        in
        select t selected)))
;;

let scheme = Gpuio.Deep_link.Scheme.of_string "gpuio-signal" |> Or_error.ok_exn

let route t link =
  if
    (not (Gpuio.Deep_link.Scheme.equal (Gpuio.Deep_link.scheme link) scheme))
    || (not (String.equal (Gpuio.Deep_link.route link) "sample"))
    || Option.is_some (Gpuio.Deep_link.query link)
    || Option.is_some (Gpuio.Deep_link.fragment link)
  then Or_error.error_string "Unsupported workspace route"
  else (
    let path = Gpuio.Deep_link.path link in
    match
      List.find t.samples ~f:(fun sample ->
        String.equal path ("/" ^ Int64.to_string (S.Item_id.to_int64 sample.id)))
    with
    | None -> Or_error.error_string "Unknown sample route"
    | Some sample -> select t (Some sample.id))
;;
