open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View
module Rows = Presentation.Chart_inspection
module Content = Chart_inspection_content

let ok = Or_error.ok_exn
let key = Key.of_string_exn
let px = Length.px_exn
let style = Style.create_exn

module Mode = struct
  type t =
    | Native
    | Rows
    | Interactive
    | Overlay
  [@@deriving equal]

  let all = [ Native; Rows; Interactive; Overlay ]

  let label = function
    | Native -> "Native inspection"
    | Rows -> "Inspection rows"
    | Interactive -> "Interactive inspection"
    | Overlay -> "Inspection overlay"
  ;;
end

type t =
  { palette : Palette.t
  ; mode : Mode.t
  ; set_mode : Mode.t -> unit Bonsai.Effect.t
  ; activations : int
  ; activate : unit Bonsai.Effect.t
  ; editor : Gpuio_eio.Text_input.t
  }

let component window palette graph =
  let mode, set_mode = B.state Mode.Native graph in
  let activations, set_activations = B.state 0 graph in
  let editor =
    Gpuio_eio.Text_input.create
      window
      ~config:
        (B.return
           (Text_input.Config.create
              ~mode:Single_line
              ~label:"Inspection note"
              ~placeholder:"Add context"
              ()
            |> ok))
      graph
  in
  let open B.Let_syntax in
  let%arr palette = palette
  and mode = mode
  and set_mode = set_mode
  and activations = activations
  and set_activations = set_activations
  and editor = editor in
  { palette
  ; mode
  ; set_mode
  ; activations
  ; activate = set_activations (activations + 1)
  ; editor
  }
;;

let controls t =
  V.column
    ~style:(style [ Gap (px 6.) ])
    [ V.row
        ~style:(style [ Gap (px 6.); Wrap Wrap ])
        (List.map Mode.all ~f:(fun candidate ->
           Palette.button
             t.palette
             ~selected:(Mode.equal candidate t.mode)
             (Mode.label candidate)
             (t.set_mode candidate)))
    ; Palette.text
        t.palette
        ~size:12.
        ~muted:true
        (sprintf "Inspection activations: %d" t.activations)
    ; Palette.text
        t.palette
        ~size:12.
        ~muted:true
        "Focus the pie and press Home, then Tab to enter its details. The first slice \
         has an action and native note in interactive modes."
    ]
;;

let slice_content t ~total slice =
  let p = t.palette in
  let id = Chart_data.Slice.id slice in
  let interactive =
    (Mode.equal t.mode Interactive || Mode.equal t.mode Overlay)
    && Chart_data.Datum_id.equal id (Chart_data.Datum_id.of_int64 1L |> ok)
  in
  let value = Chart_data.Slice.value slice in
  let share = if Float.(total > 0.) then value /. total *. 100. else 0. in
  let rows =
    Rows.view
      (Palette.appearance p)
      ~key:(key "rows")
      ~style:(style [ Font_size (Palette.size p 12.) ])
      ~title:(Palette.text p ~size:13. (Chart_data.Slice.label slice))
      [ Rows.Row.text
          ~key:(key "weight")
          ~color:(Palette.accent p)
          ~label:"Weight"
          ~value:(sprintf "%.0f" value)
      ; Rows.Row.text
          ~key:(key "share")
          ~color:(Palette.muted p)
          ~label:"Share"
          ~value:(sprintf "%.1f%%" share)
      ]
    |> ok
  in
  let children =
    if not interactive
    then [ rows ]
    else
      [ rows
      ; V.button
          ~key:(key "action")
          ~accessible_name:"Inspect chart reasoning"
          ~on_click:t.activate
          ~style:
            (style
               [ Padding (px 6.)
               ; Radius 6.
               ; Background (Background.solid (Palette.accent p))
               ; Foreground (Palette.background p)
               ])
          (sprintf "Inspect reasoning · %d" t.activations)
      ; V.column
          ~key:(key "note")
          [ Gpuio_eio.Text_input.view
              t.editor
              ~style:
                (style
                   [ Width (Length.percent_exn 100.)
                   ; Height (px (Palette.size p 30.))
                   ; Font_size (Palette.size p 12.)
                   ; Padding (px 4.)
                   ; Radius 6.
                   ; Background (Background.solid (Palette.background p))
                   ; Foreground (Palette.foreground p)
                   ; Border_width 1.
                   ; Border_color (Palette.border p)
                   ])
          ]
      ]
  in
  V.column
    ~key:(key "inspection-placement")
    ~style:(style [ Width (Length.percent_exn 100.); Align_items End ])
    [ V.column
        ~key:(key "inspection-content")
        ~style:
          (style
             [ Width (px (Palette.size p 240.))
             ; Max_width (Length.percent_exn 100.)
             ; Gap (px 6.)
             ; Padding (px 8.)
             ; Radius 8.
             ; Background (Background.solid (Palette.surface p))
             ; Foreground (Palette.foreground p)
             ; Border_width 1.
             ; Border_color (Palette.border p)
             ])
        children
    ]
;;

let content t data =
  match t.mode, Option.map data ~f:Chart_data.Expert.contents with
  | Native, _ | _, None -> Content.empty
  | (Rows | Interactive | Overlay), Some (Pie slices) ->
    let total = List.sum (module Float) slices ~f:Chart_data.Slice.value in
    List.map slices ~f:(fun slice ->
      Content.Entry.create
        ~target:(Content.Target.slice (Chart_data.Slice.id slice))
        ~container:(if Mode.equal t.mode Overlay then Overlay else Card)
        (slice_content t ~total slice))
    |> Content.create
    |> ok
  | ( (Rows | Interactive | Overlay)
    , Some (Cartesian _ | Radar _ | Candlestick _ | Sankey _ | Categorical _) ) ->
    Content.empty
;;
