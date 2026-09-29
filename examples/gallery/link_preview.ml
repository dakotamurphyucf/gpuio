open Core
open Gpuio
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View
module P = Presentation
module Registered = Gpuio_eio.Asset

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn

let arrow_svg =
  {|<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path d="M6 18L18 6M6 6h12v12" fill="none" stroke="white" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>|}
;;

let component app window palette graph =
  let resources =
    Preview_scope.acquire
      window
      ~name:"gallery composed links"
      ~create:(fun scope ->
        E.map
          (Registered.register
             app
             ~scope
             (Asset.Source.of_bytes ~format:Svg arrow_svg |> ok))
          ~f:(fun result ->
            Result.map result ~f:Registered.handle
            |> Result.map_error ~f:(fun error ->
              Error.create_s [%sexp (error : Registered.Error.t)])))
      graph
  in
  let disabled, toggle_disabled = B.toggle ~default_model:false graph in
  let tab_stop, toggle_tab_stop = B.toggle ~default_model:true graph in
  let reverse, toggle_reverse = B.toggle ~default_model:false graph in
  let detailed, toggle_detail = B.toggle ~default_model:true graph in
  let decorated, toggle_icon = B.toggle ~default_model:true graph in
  let clicks, click =
    B.state_machine0 ~default_model:0 ~apply_action:(fun _ n () -> n + 1) graph
  in
  let open B.Let_syntax in
  let%arr p = palette
  and resources = resources
  and disabled = disabled
  and toggle_disabled = toggle_disabled
  and tab_stop = tab_stop
  and toggle_tab_stop = toggle_tab_stop
  and reverse = reverse
  and toggle_reverse = toggle_reverse
  and detailed = detailed
  and toggle_detail = toggle_detail
  and decorated = decorated
  and toggle_icon = toggle_icon
  and clicks = clicks
  and click = click in
  match resources with
  | Preview_scope.Loading -> Palette.text p "Preparing link preview…"
  | Failed error ->
    Palette.text p ("Link preview unavailable: " ^ Error.to_string_hum error)
  | Ready asset ->
    let checkbox label enabled toggle =
      V.checkbox ~state:(if enabled then Checked else Unchecked) ~on_toggle:toggle label
    in
    let link name subtitle order =
      let index = if reverse then 40 - order else order in
      let config =
        Link.Config.create ~label:("Open " ^ name) ~disabled ~tab_stop ~tab_index:index ()
        |> ok
      in
      let icon =
        V.icon
          ~style:(style [ Width (px 24.); Height (px 24.) ])
          (Icon.Config.create ~asset ~description:Image.Description.decorative () |> ok)
      in
      let content =
        V.column
          ~style:(style [ Gap (px 4.) ])
          ([ Palette.text p ~size:16. name ]
           @ if detailed then [ Palette.text p ~muted:true subtitle ] else [])
      in
      P.composed_link
        (Palette.appearance p)
        ~key:(Key.of_string_exn name)
        ~style:
          (style
             [ Width (px 440.)
             ; Padding (px 16.)
             ; Gap (px 16.)
             ; Radius 12.
             ; Background (Background.solid (Palette.background p))
             ; Border_width 1.
             ; Border_color (Palette.border p)
             ])
        config
        ~on_click:(fun () -> click ())
        [ V.row
            ~key:(Key.of_string_exn "content")
            ~style:(style [ Gap (px 16.); Align_items Center ])
            ((if decorated then [ icon ] else []) @ [ content ])
        ]
      |> ok
    in
    V.column
      ~style:(style [ Gap (px 12.) ])
      [ Palette.text p "One destination. A richer invitation."
      ; Palette.text
          p
          ~muted:true
          "Composed content with one accessible action. These previews stay inside the \
           gallery."
      ; V.row
          ~style:(style [ Gap (px 12.); Wrap Wrap ])
          [ checkbox "Disable composed links" disabled toggle_disabled
          ; checkbox "Links in Tab order" tab_stop toggle_tab_stop
          ; checkbox "Reverse link order" reverse toggle_reverse
          ]
      ; V.row
          ~style:(style [ Gap (px 12.); Wrap Wrap ])
          [ checkbox "Link descriptions" detailed toggle_detail
          ; checkbox "Link icons" decorated toggle_icon
          ]
      ; link "Design guide" "Principles, patterns, and a little inspiration · 世界" 10
      ; link "API reference" "Small interfaces. Native possibilities." 30
      ; link "Release notes" "What changed, and what you can build next." 20
      ; Palette.text p ~muted:true (sprintf "Link opens: %d" clicks)
      ; Palette.text
          p
          ~muted:true
          (if reverse
           then "Link order: API · Release · Design"
           else "Link order: Design · Release · API")
      ]
;;
