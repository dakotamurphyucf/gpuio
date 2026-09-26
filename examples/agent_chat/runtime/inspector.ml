open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View

type t = bool B.Expert.Var.t

let create () = B.Expert.Var.create false
let toggle t = B.Expert.Var.set t (not (B.Expert.Var.get t))
let px = Gpuio.Length.px_exn
let style = Gpuio.Style.create_exn

let component t ~dark graph =
  let review = Review.component ~dark graph in
  let open B.Let_syntax in
  let%arr opened = B.Expert.Var.value t
  and dark = dark
  and review = review in
  let palette = Palette.of_dark dark in
  V.panel
    ~key:(Gpuio.Key.of_string_exn "artifact-inspector")
    ~label:"Artifact workspace"
    ~active:opened
    ~hidden:Unmount
    ~style:
      (style
         [ Width (px 380.)
         ; Height (Gpuio.Length.percent_exn 100.)
         ; Shrink 0.
         ; Min_height (px 0.)
         ; Background (Gpuio.Background.solid palette.sidebar)
         ; Foreground palette.text
         ; Border_left_width 1.
         ; Border_color palette.line
         ])
    [ V.row
        ~style:
          (style
             [ Height (px 45.)
             ; Shrink 0.
             ; Align_items Center
             ; Justify_content Space_between
             ; Padding_left (px 20.)
             ; Padding_right (px 12.)
             ; Border_bottom_width 1.
             ; Border_color palette.line
             ])
        [ V.text ~style:(style [ Font_size 12.; Font_weight 600 ]) "ARTIFACT WORKSPACE"
        ; V.button
            ~accessible_name:"Close workspace inspector"
            ~style:
              (style
                 [ Foreground palette.text
                 ; Background (Gpuio.Background.solid palette.raised)
                 ; Border_color palette.line
                 ; Border_width 1.
                 ; Padding (px 8.)
                 ; Radius 8.
                 ])
            ~on_click:(E.of_thunk (fun () -> B.Expert.Var.set t false))
            "Close"
        ]
    ; V.column
        ~style:
          (style [ Grow 1.; Min_height (px 0.); Padding (px 22.); Overflow_y Scroll ])
        [ review ]
    ]
;;
