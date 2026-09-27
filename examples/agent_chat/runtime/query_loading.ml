open Core
module V = Gpuio_bonsai.View

let px = Gpuio.Length.px_exn
let style = Gpuio.Style.create_exn

let indicator ~dark ~kind ~label properties =
  let open Gpuio.Style.Property in
  let p = Palette.of_dark dark in
  V.loading
    ~style:
      (style
         ([ Foreground p.accent; Background (Gpuio.Background.solid p.raised); Radius 6. ]
          @ properties))
    ~config:(Gpuio.Loading.Config.create ~kind ~label () |> Or_error.ok_exn)
    ()
;;

let spinner ~dark ~label =
  indicator ~dark ~kind:Spinner ~label [ Width (px 18.); Height (px 18.) ]
;;

let results ~dark =
  let p = Palette.of_dark dark in
  V.column
    ~style:
      (style
         [ Height (px 300.)
         ; Shrink 0.
         ; Padding (px 18.)
         ; Gap (px 14.)
         ; Background (Gpuio.Background.solid p.surface)
         ; Border_width 1.
         ; Border_color p.line
         ; Radius 10.
         ])
    [ indicator
        ~dark
        ~kind:Skeleton
        ~label:"Preparing result columns"
        [ Width (px 180.); Height (px 14.) ]
    ; indicator
        ~dark
        ~kind:Shimmer
        ~label:"Preparing result rows"
        [ Width (Gpuio.Length.percent_exn 100.); Height (px 98.) ]
    ; V.text
        ~style:(style [ Foreground p.muted; Font_size 12. ])
        "Waiting for the local query…"
    ]
;;
