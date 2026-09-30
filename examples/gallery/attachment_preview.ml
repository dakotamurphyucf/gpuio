open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View
module A = Presentation.Attachment
module Registered = Gpuio_eio.Asset

let ok = Or_error.ok_exn
let key = Key.of_string_exn
let style = Style.create_exn
let px = Length.px_exn

let named name view =
  V.with_accessibility view (Accessibility.create ~role:Group ~label:name () |> ok) |> ok
;;

let component app window palette graph =
  let resource =
    Preview_scope.acquire
      window
      ~name:"gallery attachment media"
      ~create:(fun scope ->
        let register bytes =
          Bonsai.Effect.map
            (Registered.register
               app
               ~scope
               (Asset.Source.of_bytes ~format:Pnm bytes |> ok))
            ~f:(fun result ->
              Result.map result ~f:Registered.handle
              |> Result.map_error ~f:(fun error ->
                Error.create_s [%sexp (error : Registered.Error.t)]))
        in
        Bonsai.Effect.bind (register Image_samples.gradient_pnm) ~f:(function
          | Error error -> Bonsai.Effect.return (Error error)
          | Ok image ->
            Bonsai.Effect.map
              (register "Intentionally malformed attachment image")
              ~f:(Result.map ~f:(fun failed -> image, failed))))
      graph
  in
  let status, next_status =
    B.state_machine0
      ~default_model:4
      ~apply_action:(fun _ value () -> (value + 1) % 5)
      graph
  in
  let size, next_size =
    B.state_machine0
      ~default_model:2
      ~apply_action:(fun _ value () -> (value + 1) % 5)
      graph
  in
  let vertical, toggle_axis = B.toggle ~default_model:false graph in
  let image, toggle_image = B.toggle ~default_model:false graph in
  let broken, toggle_broken = B.toggle ~default_model:false graph in
  let image_state, set_image_state = B.state Image.State.Loading graph in
  let media, toggle_media = B.toggle ~default_model:true graph in
  let content, toggle_content = B.toggle ~default_model:true graph in
  let actions, toggle_actions = B.toggle ~default_model:true graph in
  let disabled, toggle_disabled = B.toggle ~default_model:false graph in
  let refined, toggle_refined = B.toggle ~default_model:false graph in
  let more, toggle_more = B.toggle ~default_model:false graph in
  let constrained, toggle_constrained = B.toggle ~default_model:false graph in
  let opened, open_attachment =
    B.state_machine0 ~default_model:0 ~apply_action:(fun _ value () -> value + 1) graph
  in
  let saved, save =
    B.state_machine0 ~default_model:0 ~apply_action:(fun _ value () -> value + 1) graph
  in
  let open B.Let_syntax in
  let%arr p = palette
  and resource = resource
  and status = status
  and next_status = next_status
  and size = size
  and next_size = next_size
  and vertical = vertical
  and toggle_axis = toggle_axis
  and image = image
  and toggle_image = toggle_image
  and broken = broken
  and toggle_broken = toggle_broken
  and image_state = image_state
  and set_image_state = set_image_state
  and media = media
  and toggle_media = toggle_media
  and content = content
  and toggle_content = toggle_content
  and actions = actions
  and toggle_actions = toggle_actions
  and disabled = disabled
  and toggle_disabled = toggle_disabled
  and refined = refined
  and toggle_refined = toggle_refined
  and more = more
  and toggle_more = toggle_more
  and constrained = constrained
  and toggle_constrained = toggle_constrained
  and opened = opened
  and open_attachment = open_attachment
  and saved = saved
  and save = save in
  match resource with
  | Preview_scope.Loading -> Palette.text p "Preparing attachment media…"
  | Failed error ->
    Palette.text p ("Attachment media unavailable: " ^ Error.to_string_hum error)
  | Ready (asset, invalid_asset) ->
    let status_label, status =
      [| "Pending", A.Status.Pending
       ; "Uploading", Uploading
       ; "Processing", Processing
       ; "Failed", Failed
       ; "Complete", Complete
      |].(status)
    in
    let size_label, size =
      [| "XS", A.Size.xsmall
       ; "S", A.Size.small
       ; "M", A.Size.medium
       ; "L", A.Size.large
       ; "56", A.Size.pixels 56. |> ok
      |].(size)
    in
    let checkbox name checked on_toggle =
      V.checkbox ~state:(if checked then Checked else Unchecked) ~on_toggle name
    in
    let content =
      Option.some_if
        content
        (A.Content.create
           [ A.Content.Item.title
               (A.Title.create ~key:(key "title") "Aurora · 京都.png" |> ok)
           ; A.Content.Item.description
               (A.Description.create ~key:(key "description") "PNG image · 2.4 MB")
           ])
    in
    let media =
      Option.some_if
        media
        (A.Media.create
           ?image:
             (Option.some_if
                image
                (A.Media.Image.create
                   ~asset:(if broken then invalid_asset else asset)
                   ~description:(Image.Description.label "Attachment landscape" |> ok)
                   ~on_change:set_image_state
                   ()))
           ~overlay:
             (V.text
                ~style:
                  (style
                     [ Font_weight 600; Font_size 11.; Foreground (Palette.foreground p) ])
                "PNG")
           [])
    in
    let actions =
      Option.some_if
        actions
        (A.Actions.create
           ~style:(style [ Gap (px 12.) ])
           [ V.button
               ~key:(key "save")
               ~accessible_name:"Save attachment"
               ~on_click:(save ())
               ~style:
                 (style
                    [ Padding (px 6.)
                    ; Font_size 12.
                    ; Foreground (Palette.foreground p)
                    ; Background (Background.solid (Palette.surface p))
                    ])
               "↓"
           ; V.button
               ~key:(key "unavailable")
               ~disabled:true
               ~accessible_name:"Unavailable attachment action"
               ~on_click:(open_attachment ())
               ~style:
                 (style
                    [ Padding (px 6.)
                    ; Font_size 12.
                    ; Foreground (Palette.muted p)
                    ; Background (Background.solid (Palette.surface p))
                    ])
               "×"
           ])
    in
    let card =
      A.create
        (Palette.appearance p)
        ~key:(key "rich-attachment")
        ~status
        ~size
        ~axis:(if vertical then Vertical else Horizontal)
        ~style:
          (style
             ([ Style.Property.Width (px (if vertical then 180. else 440.)) ]
              @
              if refined
              then [ Border_width 2.; Border_color (Palette.accent p); Radius 8. ]
              else []))
        ?media
        ?content
        ?actions
        ~trigger:
          (A.Trigger.create
             ~key:(key "open")
             ~accessible_name:"Open Aurora attachment"
             ~disabled
             ~on_click:(fun () -> open_attachment ())
             ?style:(Option.some_if refined (style [ Radius 8. ]))
             ()
           |> ok)
        ()
      |> named "Attachment preview card"
    in
    let additional name detail =
      A.create
        (Palette.appearance p)
        ~key:(key name)
        ~axis:Vertical
        ~style:(style [ Width (px 160.) ])
        ~media:(A.Media.create [ V.text ~style:(style [ Font_size 24. ]) "≡" ])
        ~content:
          (A.Content.create
             [ A.Content.Item.title (A.Title.create ~key:(key "title") name |> ok)
             ; A.Content.Item.description
                 (A.Description.create ~key:(key "detail") detail)
             ])
        ~trigger:
          (A.Trigger.create
             ~key:(key "open")
             ~accessible_name:("Open " ^ name ^ " attachment")
             ~on_click:(fun () -> open_attachment ())
             ()
           |> ok)
        ()
    in
    V.column
      ~style:(style [ Gap (px 12.) ])
      [ Palette.text p ~muted:true "From queued to ready, with room for the details."
      ; V.row
          ~style:(style [ Gap (px 8.); Wrap Wrap ])
          [ Palette.button p ("Attachment status: " ^ status_label) (next_status ())
          ; Palette.button p ("Attachment size: " ^ size_label) (next_size ())
          ; Palette.button
              p
              (if vertical
               then "Attachment layout: vertical"
               else "Attachment layout: horizontal")
              toggle_axis
          ]
      ; V.row
          ~style:(style [ Gap (px 12.); Wrap Wrap ])
          [ checkbox "Attachment media" (Option.is_some media) toggle_media
          ; checkbox "Attachment content" (Option.is_some content) toggle_content
          ; checkbox "Attachment actions" (Option.is_some actions) toggle_actions
          ]
      ; V.row
          ~style:(style [ Gap (px 12.); Wrap Wrap ])
          [ checkbox "Attachment image" image toggle_image
          ; checkbox "Simulate attachment decode failure" broken toggle_broken
          ; checkbox "Disable attachment" disabled toggle_disabled
          ; checkbox "Refine attachment style" refined toggle_refined
          ]
      ; V.row
          ~style:(style [ Gap (px 12.); Wrap Wrap ])
          [ checkbox "More attachments" more toggle_more
          ; checkbox "Constrain attachment group" constrained toggle_constrained
          ]
      ; A.group
          ~key:(key "attachment-group")
          ?style:(Option.some_if constrained (style [ Width (px 280.) ]))
          (card
           ::
           (if more
            then
              [ additional "Research" "PDF · 1.8 MB"
              ; additional "Notes" "Markdown · 4 KB"
              ]
            else []))
        |> named "Attachment preview group"
      ; Palette.text p (sprintf "Attachment opened: %d · saved: %d" opened saved)
      ; Palette.text
          p
          ~muted:true
          (if not image
           then "Attachment image: off"
           else (
             match image_state with
             | Loading -> "Attachment image: loading"
             | Ready _ -> "Attachment image: ready"
             | Failed _ -> "Attachment image: failed"))
      ]
;;
