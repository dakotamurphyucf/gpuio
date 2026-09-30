open Core
open Gpuio
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View
module P = Presentation
module Registered = Gpuio_eio.Asset

type kind =
  | Count of int
  | Dot
  | Icon
  | Decorative_icon
[@@deriving equal]

let ok = Or_error.ok_exn
let px = Length.px_exn
let style = Style.create_exn

let check_svg =
  {|<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path d="M5 12l4 4L19 6" fill="none" stroke="white" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"/></svg>|}
;;

let component app window palette graph =
  let resources =
    Preview_scope.acquire
      window
      ~name:"gallery badge icon"
      ~create:(fun scope ->
        E.map
          (Registered.register
             app
             ~scope
             (Asset.Source.of_bytes ~format:Svg check_svg |> ok))
          ~f:(fun result ->
            Result.map result ~f:Registered.handle
            |> Result.map_error ~f:(fun error ->
              Error.create_s [%sexp (error : Registered.Error.t)])))
      graph
  in
  let kind, set_kind = B.state (Count 7) graph in
  let size, set_size = B.state P.Size.Medium graph in
  let cap, toggle_cap = B.toggle ~default_model:true graph in
  let clicks, click =
    B.state_machine0 ~default_model:0 ~apply_action:(fun _ count () -> count + 1) graph
  in
  let open B.Let_syntax in
  let%arr p = palette
  and resources = resources
  and kind = kind
  and set_kind = set_kind
  and size = size
  and set_size = set_size
  and cap = cap
  and toggle_cap = toggle_cap
  and clicks = clicks
  and click = click in
  match resources with
  | Preview_scope.Loading -> Palette.text p "Preparing badge preview…"
  | Failed error ->
    Palette.text p ("Badge preview unavailable: " ^ Error.to_string_hum error)
  | Ready asset ->
    let icon description =
      P.Overlay_badge.icon (Icon.Config.create ~asset ~description () |> ok)
    in
    let badge =
      match kind with
      | Count value ->
        P.Overlay_badge.count
          ~max:(if cap then 99 else 9)
          ~label:(sprintf "%d unread messages" value)
          value
        |> ok
      | Dot -> P.Overlay_badge.dot ~label:"Inbox has new activity" |> ok
      | Icon -> icon (Image.Description.label "Verified inbox" |> ok)
      | Decorative_icon -> icon Image.Description.decorative
    in
    let size_label, next_size =
      match size with
      | P.Size.Small -> "Badge size: small", P.Size.Medium
      | Medium -> "Badge size: medium", P.Size.Large
      | Large -> "Badge size: large", P.Size.Small
    in
    let tone =
      match kind with
      | Count _ -> P.Tone.Danger
      | Dot -> Accent
      | Icon | Decorative_icon -> Success
    in
    V.column
      ~style:(style [ Gap (px 10.) ])
      [ V.row
          ~style:(style [ Gap (px 6.); Wrap Wrap ])
          (List.map
             [ "Unread: 0", Count 0
             ; "Unread: 7", Count 7
             ; "Unread: 150", Count 150
             ; "Activity dot", Dot
             ; "Verified icon", Icon
             ; "Decorative icon", Decorative_icon
             ]
             ~f:(fun (label, value) ->
               Palette.button p ~selected:(equal_kind kind value) label (set_kind value)))
      ; V.row
          ~style:(style [ Gap (px 18.); Align_items Center ])
          [ (P.overlay_badge
               (Palette.appearance p)
               ~size
               ~tone
               ~badge
               ~style:(style [ Width (px 170.) ])
               (V.button
                  ~style:
                    (style
                       [ Height (px 48.)
                       ; Width (Length.percent_exn 100.)
                       ; Background (Background.solid (Palette.surface p))
                       ; Foreground (Palette.foreground p)
                       ; Radius 8.
                       ])
                  "Open inbox"
                  ~on_click:(click ()))
             |> fun view ->
             V.with_accessibility
               view
               (Accessibility.create ~role:Group ~label:"Badged inbox" () |> ok)
             |> ok)
          ; Palette.button p size_label (set_size next_size)
          ; Palette.button p (if cap then "Badge cap: 99" else "Badge cap: 9") toggle_cap
          ; Palette.text p ~muted:true (sprintf "Inbox opens: %d" clicks)
          ]
      ]
;;
