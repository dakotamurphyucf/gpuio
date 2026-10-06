open Core
open Gpuio
module I = Chart_inspection

type t =
  | Default
  | Vertical
  | Band
  | Anchored
  | Cursor
  | Marker_only
[@@deriving equal]

let all = [ Default; Vertical; Band; Anchored; Cursor; Marker_only ]

let label = function
  | Default -> "Default inspection"
  | Vertical -> "Vertical crosshair"
  | Band -> "Horizontal band"
  | Anchored -> "Anchored details"
  | Cursor -> "Cursor details"
  | Marker_only -> "Marker only"
;;

let config t =
  let ok = Or_error.ok_exn in
  let accent = Color.rgb_exn 0xfa315e in
  match t with
  | Default -> I.default
  | Vertical ->
    I.create ~crosshair:(I.Crosshair.create ~axis:Vertical ~color:accent () |> ok) ()
  | Band ->
    I.create
      ~crosshair:
        (I.Crosshair.create
           ~axis:Horizontal
           ~pattern:Solid
           ~thickness:12.
           ~color:(Color.rgba ~red:250 ~green:49 ~blue:94 ~alpha:80 |> ok)
           ()
         |> ok)
      ()
  | Anchored | Cursor ->
    I.create
      ~card:
        (I.Card.create
           ~placement:(if equal t Cursor then Cursor else Anchor)
           ~width:180.
           ~gap:12.
           ~padding:10.
           ~radius:10.
           ~border_width:1.
           ~border_color:accent
           ~background:(Color.rgb_exn 0x0f172a)
           ~text_color:(Color.rgb_exn 0xf8fafc)
           ()
         |> ok)
      ~crosshair:(I.Crosshair.create ~axis:Both ~pattern:Solid ~color:accent () |> ok)
      ~marker:
        (I.Marker.create
           ~size:24.
           ~stroke_width:3.
           ~fill:(Color.rgb_exn 0x0f172a)
           ~stroke:accent
           ()
         |> ok)
      ()
  | Marker_only ->
    I.create
      ~card:(I.Card.create ~visible:false () |> ok)
      ~marker:(I.Marker.create ~size:20. ~status:false ~fill:accent () |> ok)
      ()
;;
