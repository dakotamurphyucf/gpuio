open Core
module V = Gpuio_bonsai.View
module P = Gpuio.Presentation

module Kind = struct
  type t =
    | User
    | Assistant
    | Artifact
end

let view ~(kind : Kind.t) ~dark ~icons ~content =
  let p = Palette.of_dark dark in
  let appearance = if dark then P.Appearance.dark else P.Appearance.light in
  let px = Gpuio.Length.px_exn in
  let style = Gpuio.Style.create_exn in
  let full = Gpuio.Length.percent_exn 100. in
  let avatar =
    V.column
      ~style:
        (style
           [ Width (px 27.)
           ; Height (px 27.)
           ; Shrink 0.
           ; Radius 8.
           ; Align_items Center
           ; Justify_content Center
           ; Background (Gpuio.Background.solid p.accent_surface)
           ; Foreground p.accent
           ; Font_size 11.
           ; Font_weight 600
           ])
      [ (match kind with
         | User -> V.text "Y"
         | Assistant | Artifact -> Icons.view icons Spark)
      ]
  in
  let common =
    style
      [ Width full
      ; Max_width (px 760.)
      ; Foreground p.text
      ; Border_color p.line
      ; Radius 12.
      ]
  in
  let message ~author ~detail content =
    P.message
      appearance
      ~style:
        (Gpuio.Style.merge
           [ common; style [ Background (Gpuio.Background.solid p.canvas) ] ])
      ~avatar
      ~author
      ~detail
      content
  in
  let card =
    match kind with
    | Artifact ->
      P.tool_result
        appearance
        ~style:
          (Gpuio.Style.merge
             [ common; style [ Background (Gpuio.Background.solid p.surface) ] ])
        ~title:"Workspace · artifact"
        ~status:(P.marker appearance ~tone:Success "Ready")
        content
    | User ->
      message
        ~author:"You"
        ~detail:"Just now"
        (P.bubble
           appearance
           ~style:
             (style [ Background (Gpuio.Background.solid p.surface); Foreground p.text ])
           content)
    | Assistant -> message ~author:"GPUIO" ~detail:"Local assistant" content
  in
  V.column
    ~style:
      (style
         [ Width full
         ; Align_items Center
         ; Padding_left (px 28.)
         ; Padding_right (px 28.)
         ; Padding_top (px 8.)
         ; Padding_bottom (px 8.)
         ])
    [ card ]
;;
