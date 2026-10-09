open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View
module P = Presentation
module Empty = P.Empty_state
module Registered = Gpuio_eio.Asset

let style = Style.create_exn
let px = Length.px_exn
let ok = Or_error.ok_exn

let named name view =
  V.with_accessibility view (Accessibility.create ~role:Group ~label:name () |> ok) |> ok
;;

let component app window palette graph =
  let resources =
    Preview_scope.acquire
      window
      ~name:"gallery empty media"
      ~create:(fun scope ->
        Bonsai.Effect.map
          (Registered.register
             app
             ~scope
             (Asset.Source.of_bytes ~format:Pnm Image_samples.gradient_pnm |> ok))
          ~f:(fun result ->
            Result.map result ~f:Registered.handle
            |> Result.map_error ~f:(fun error ->
              Error.create_s [%sexp (error : Registered.Error.t)])))
      graph
  in
  let image, toggle_image = B.toggle ~default_model:false graph in
  let image_state, set_image_state = B.state Image.State.Loading graph in
  let media, toggle_media = B.toggle ~default_model:true graph in
  let title, toggle_title = B.toggle ~default_model:true graph in
  let description, toggle_description = B.toggle ~default_model:true graph in
  let content, toggle_content = B.toggle ~default_model:true graph in
  let extra, toggle_extra = B.toggle ~default_model:true graph in
  let framed, toggle_framed = B.toggle ~default_model:false graph in
  let leading, toggle_leading = B.toggle ~default_model:false graph in
  let narrow, toggle_narrow = B.toggle ~default_model:false graph in
  let bordered, toggle_border = B.toggle ~default_model:false graph in
  let solid_border, toggle_solid_border = B.toggle ~default_model:false graph in
  let large_description, toggle_large_description = B.toggle ~default_model:false graph in
  let compact_spacing, toggle_compact_spacing = B.toggle ~default_model:false graph in
  let checked, toggle_checked = B.toggle ~default_model:false graph in
  let clicks, click =
    B.state_machine0 ~default_model:0 ~apply_action:(fun _ count () -> count + 1) graph
  in
  let open B.Let_syntax in
  let%arr p = palette
  and resources = resources
  and image = image
  and toggle_image = toggle_image
  and image_state = image_state
  and set_image_state = set_image_state
  and media = media
  and toggle_media = toggle_media
  and title = title
  and toggle_title = toggle_title
  and description = description
  and toggle_description = toggle_description
  and content = content
  and toggle_content = toggle_content
  and extra = extra
  and toggle_extra = toggle_extra
  and framed = framed
  and toggle_framed = toggle_framed
  and leading = leading
  and toggle_leading = toggle_leading
  and narrow = narrow
  and toggle_narrow = toggle_narrow
  and bordered = bordered
  and toggle_border = toggle_border
  and solid_border = solid_border
  and toggle_solid_border = toggle_solid_border
  and large_description = large_description
  and toggle_large_description = toggle_large_description
  and compact_spacing = compact_spacing
  and toggle_compact_spacing = toggle_compact_spacing
  and checked = checked
  and toggle_checked = toggle_checked
  and clicks = clicks
  and click = click in
  match resources with
  | Preview_scope.Loading -> Palette.text p "Preparing empty-state media…"
  | Failed error ->
    Palette.text p ("Empty-state media unavailable: " ^ Error.to_string_hum error)
  | Ready asset ->
    let appearance = Palette.appearance p in
    let checkbox label enabled toggle =
      V.checkbox ~state:(if enabled then Checked else Unchecked) ~on_toggle:toggle label
    in
    let alignment = style [ Align_items (if leading then Start else Center) ] in
    let avatar name initials =
      V.avatar
        ~style:(style [ Width (px 32.); Height (px 32.); Foreground (Palette.accent p) ])
        (Avatar.Config.create
           ~fallback:(Avatar.Fallback.create initials |> ok)
           ~description:(Image.Description.label name |> ok)
           ())
    in
    let media_view =
      if framed
      then V.text "✦"
      else if image
      then
        V.image
          ~key:(Key.of_string_exn "empty-image")
          ~on_change:set_image_state
          ~style:(style [ Width (px 96.); Height (px 48.); Radius 8.; Shrink 0. ])
          (Image.Config.create
             ~asset
             ~description:(Image.Description.label "Empty media image" |> ok)
             ())
      else
        V.row
          ~style:(style [ Gap (px 6.); Shrink 0. ])
          [ avatar "Empty preview Alex" "AL"; avatar "Empty preview Sam" "SA" ]
    in
    V.column
      ~style:(style [ Gap (px 12.) ])
      [ Palette.text p "Make space for a fresh start."
      ; V.row
          ~style:(style [ Gap (px 12.); Wrap Wrap ])
          [ checkbox "Empty media" media toggle_media
          ; checkbox "Empty title" title toggle_title
          ; checkbox "Empty description" description toggle_description
          ; checkbox "Empty content" content toggle_content
          ; checkbox "Empty extras" extra toggle_extra
          ]
      ; V.row
          ~style:(style [ Gap (px 12.); Wrap Wrap ])
          [ checkbox "Frame empty media" framed toggle_framed
          ; checkbox "Use image empty media" image toggle_image
          ; checkbox "Align empty slots to start" leading toggle_leading
          ; checkbox "Narrow empty preview" narrow toggle_narrow
          ]
      ; V.row
          ~style:(style [ Gap (px 12.); Wrap Wrap ])
          [ checkbox "Show empty border" bordered toggle_border
          ; checkbox "Use solid empty border" solid_border toggle_solid_border
          ]
      ; V.row
          ~style:(style [ Gap (px 12.); Wrap Wrap ])
          [ checkbox "Large empty description" large_description toggle_large_description
          ; checkbox "Compact empty line spacing" compact_spacing toggle_compact_spacing
          ]
      ; Empty.create
          appearance
          ~key:(Key.of_string_exn "empty-preview")
          ~style:
            (Style.merge
               [ style
                   [ Width (px (if narrow then 240. else 440.))
                   ; Align_items (if leading then Start else Center)
                   ; Text_align (if leading then Left else Center)
                   ; Background (Background.solid (Palette.background p))
                   ]
               ; (if bordered
                  then style [ Border_width 2.; Border_color (Palette.accent p) ]
                  else Style.empty)
               ; (if solid_border then style [ Border_style Solid ] else Style.empty)
               ])
          ~children_style:alignment
          ?header:
            (Option.some_if
               (media || title || description)
               (Empty.header
                  ~style:alignment
                  ?media:
                    (Option.some_if
                       media
                       (Empty.media
                          appearance
                          ~variant:(if framed then Icon else Unframed)
                          [ media_view ]
                        |> named "Empty rich media"))
                  ?title:
                    (Option.some_if
                       title
                       (Empty.title
                          [ V.row
                              ~style:
                                (style [ Gap (px 6.); Wrap Wrap; Align_items Center ])
                              [ V.text "Room for what is next"
                              ; P.badge appearance ~tone:Accent "New"
                              ]
                          ]
                        |> named "Empty rich title"))
                  ?description:
                    (Option.some_if
                       description
                       (Empty.description
                          appearance
                          ~style:
                            (Style.merge
                               [ (if large_description
                                  then style [ Font_size 20. ]
                                  else Style.empty)
                               ; (if compact_spacing
                                  then style [ Line_height (px 24.) ]
                                  else Style.empty)
                               ])
                          [ V.text
                              "Bring your ideas together in a collection. Add notes, \
                               conversations and discoveries as your work grows."
                            |> named "Empty description text"
                          ; Palette.button p "Read the empty-state guide" (click ())
                          ]
                        |> named "Empty rich description"))
                  ()))
          ?content:
            (Option.some_if
               content
               (Empty.content
                  ~style:alignment
                  [ checkbox "Keep empty-state updates" checked toggle_checked
                  ; Palette.button p "Create first item" (click ())
                  ]
                |> named "Empty rich content"))
          (Option.to_list
             (Option.some_if extra (Palette.button p "Import instead" (click ()))))
        |> named "Rich empty state"
      ; Palette.text p ~muted:true (sprintf "Empty actions: %d" clicks)
      ; Palette.text
          p
          ~muted:true
          (sprintf
             "Empty description: %s · %s"
             (if large_description then "20 px" else "14 px")
             (if compact_spacing then "24 px spacing" else "relative spacing"))
      ; Palette.text
          p
          ~muted:true
          (sprintf
             "Empty border: %s · %s"
             (if bordered then "visible" else "hidden")
             (if solid_border then "solid override" else "default dashed"))
      ; Palette.text
          p
          ~muted:true
          (if framed
           then "Empty media: icon frame"
           else if not image
           then "Empty media: avatar row"
           else (
             match image_state with
             | Image.State.Loading -> "Empty image: awaiting decode"
             | Failed error ->
               "Empty image: " ^ Sexp.to_string_hum (Image.Error.sexp_of_t error)
             | Ready metadata ->
               sprintf
                 "Empty image decoded: %d × %d"
                 (Image.Metadata.width_px metadata)
                 (Image.Metadata.height_px metadata)))
      ; Palette.text
          p
          ~muted:true
          (sprintf
             "Empty layout: %s · %s · %s"
             (if framed then "icon" else "unframed")
             (if leading then "start" else "center")
             (if narrow then "narrow" else "wide"))
      ]
;;
