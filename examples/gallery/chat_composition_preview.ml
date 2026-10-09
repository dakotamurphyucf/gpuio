open Core
open Gpuio
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View
module P = Presentation
module Bubble = P.Bubble
module Message = P.Message
module D = Gpuio_eio.Document
module Editor = Gpuio_eio.Text_input

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn
let key = Key.of_string_exn
let full = Length.percent_exn 100.

module Inset = struct
  type t =
    | Inherit
    | Yes
    | No
end

module Resources = struct
  type t =
    { document : D.t
    ; mutable chunks : int
    }

  let initial =
    "A steady place for the next idea. 世界\n\n\
     ```ocaml\n\
     let next_step = \"keep going\"\n\
     ```\n"
  ;;

  let create app scope =
    E.map
      (D.create app ~scope (Text_source.of_string ~status:Streaming initial |> ok))
      ~f:(function
        | Ok document -> Ok { document; chunks = 0 }
        | Error e -> Error (Error.create_s [%sexp (e : D.Error.t)]))
  ;;

  let append t =
    if t.chunks >= 6
    then Ok t.chunks
    else
      Result.map
        (D.append
           t.document
           (sprintf
              "\n\n\
               **Update %d** · Native layout grows with each new idea. A little context \
               makes the next step clearer. 京都\n"
              (t.chunks + 1)))
        ~f:(fun () ->
          t.chunks <- t.chunks + 1;
          t.chunks)
  ;;
end

let component app window palette graph =
  let resources =
    Preview_scope.acquire
      window
      ~name:"gallery-chat-composition"
      ~create:(Resources.create app)
      graph
  in
  let editor =
    Editor.create
      window
      ~initial_text:"Keep this draft"
      ~config:
        (B.return
           (Text_input.Config.create ~mode:Single_line ~label:"Message draft" () |> ok))
      graph
  in
  let variant, next_variant =
    B.state_machine0
      ~default_model:Bubble.Variant.Secondary
      ~apply_action:(fun _ v () ->
        match v with
        | Filled -> Secondary
        | Secondary -> Muted
        | Muted -> Tinted
        | Tinted -> Outline
        | Outline -> Ghost
        | Ghost -> Destructive
        | Destructive -> Filled)
      graph
  in
  let end_aligned, toggle_alignment = B.toggle ~default_model:false graph in
  let bubble_alignment, next_bubble_alignment =
    B.state_machine0
      ~default_model:None
      ~apply_action:(fun _ v () ->
        match v with
        | None -> Some P.Alignment.Start
        | Some P.Alignment.Start -> Some P.Alignment.End
        | Some P.Alignment.End -> None)
      graph
  in
  let top, toggle_top = B.toggle ~default_model:false graph in
  let reaction_start, toggle_reaction_start = B.toggle ~default_model:false graph in
  let avatar, toggle_avatar = B.toggle ~default_model:true graph in
  let header, toggle_header = B.toggle ~default_model:true graph in
  let footer, toggle_footer = B.toggle ~default_model:true graph in
  let expanded_footer, toggle_expanded_footer = B.toggle ~default_model:false graph in
  let reactions, toggle_reactions = B.toggle ~default_model:true graph in
  let typed_action, toggle_typed_action = B.toggle ~default_model:true graph in
  let typed_bubble, toggle_typed_bubble = B.toggle ~default_model:true graph in
  let markdown, toggle_markdown = B.toggle ~default_model:false graph in
  let compact, toggle_compact = B.toggle ~default_model:false graph in
  let clip, toggle_clip = B.toggle ~default_model:false graph in
  let inset, next_inset =
    B.state_machine0
      ~default_model:Inset.Inherit
      ~apply_action:(fun _ v () ->
        match v with
        | Inherit -> Inset.Yes
        | Yes -> No
        | No -> Inherit)
      graph
  in
  let actions, act =
    B.state_machine0 ~default_model:0 ~apply_action:(fun _ n () -> n + 1) graph
  in
  let reaction_actions, react =
    B.state_machine0 ~default_model:0 ~apply_action:(fun _ n () -> n + 1) graph
  in
  let notice, set_notice = B.state "Stream chunks: 0" graph in
  let open B.Let_syntax in
  B.Edge.lifecycle
    ~on_deactivate:
      (let%arr set_notice = set_notice in
       set_notice "Stream chunks: 0")
    graph;
  let%arr p = palette
  and resources = resources
  and editor = editor
  and variant = variant
  and next_variant = next_variant
  and end_aligned = end_aligned
  and toggle_alignment = toggle_alignment
  and bubble_alignment = bubble_alignment
  and next_bubble_alignment = next_bubble_alignment
  and top = top
  and toggle_top = toggle_top
  and reaction_start = reaction_start
  and toggle_reaction_start = toggle_reaction_start
  and avatar = avatar
  and toggle_avatar = toggle_avatar
  and header = header
  and toggle_header = toggle_header
  and footer = footer
  and toggle_footer = toggle_footer
  and expanded_footer = expanded_footer
  and toggle_expanded_footer = toggle_expanded_footer
  and reactions = reactions
  and toggle_reactions = toggle_reactions
  and typed_action = typed_action
  and toggle_typed_action = toggle_typed_action
  and typed_bubble = typed_bubble
  and toggle_typed_bubble = toggle_typed_bubble
  and markdown = markdown
  and toggle_markdown = toggle_markdown
  and clip = clip
  and toggle_clip = toggle_clip
  and compact = compact
  and toggle_compact = toggle_compact
  and inset = inset
  and next_inset = next_inset
  and actions = actions
  and act = act
  and reaction_actions = reaction_actions
  and react = react
  and notice = notice
  and set_notice = set_notice in
  match resources with
  | Loading -> Palette.text p "Preparing chat composition…"
  | Failed error -> Palette.text p (Error.to_string_hum error)
  | Ready resources ->
    let variant_name =
      match variant with
      | Filled -> "Filled"
      | Secondary -> "Secondary"
      | Muted -> "Muted"
      | Tinted -> "Tinted"
      | Outline -> "Outline"
      | Ghost -> "Ghost"
      | Destructive -> "Destructive"
    in
    let alignment_name =
      match bubble_alignment with
      | None -> "Inherit"
      | Some Start -> "Start"
      | Some End -> "End"
    in
    let inset_name, content_inset =
      match inset with
      | Inherit -> "Inherit", None
      | Yes -> "Yes", Some true
      | No -> "No", Some false
    in
    let annotate label view =
      V.with_accessibility view (Accessibility.create ~role:Group ~label () |> ok) |> ok
    in
    let reaction_item =
      if typed_action
      then
        Bubble.Reactions.Item.action
          ~key:(key "reaction")
          ~on_click:(fun () -> react ())
          "React to message"
      else
        Bubble.Reactions.Item.element
          ~key:(key "reaction")
          (V.button ~style:(style [ Radius 3. ]) ~on_click:(react ()) "React to message")
    in
    let reactions =
      Option.some_if
        reactions
        (Bubble.Reactions.create
           ~side:(if top then Top else Bottom)
           ~alignment:(if reaction_start then Start else End)
           [ reaction_item ]
         |> ok)
    in
    let body =
      if markdown
      then (
        let config =
          Document.Config.create
            ~source:(D.handle resources.document)
            ~mode:Markdown
            ~appearance:(Palette.document_appearance p)
            ~layout:Flow
            ~label:"Composed message document"
            ()
          |> ok
        in
        V.document ~key:(key "prose") ~style:(style [ Width full ]) config)
      else
        V.text
          ~key:(key "prose")
          ~style:(style [ User_select true ])
          ("A steady place for the next idea. 世界"
           ^ String.concat
               (List.init resources.chunks ~f:(fun _ ->
                  " More context arrives, and the layout has room to grow. 京都")))
    in
    let draft =
      Editor.view
        ~style:
          (style
             [ Height (px 36.)
             ; Width full
             ; Background (Background.solid (Palette.background p))
             ; Foreground (Palette.foreground p)
             ])
        editor
      |> fun view -> V.with_key view (key "draft")
    in
    let bubble =
      Bubble.create
        (Palette.appearance p)
        ~variant
        ?alignment:bubble_alignment
        ?reactions
        ~style:
          (style
             [ Width full
             ; Margin_top (px 24.)
             ; Overflow (if clip then Hidden else Visible)
             ])
        ~content_style:(style [ Gap (px 8.) ])
        [ V.column
            ~key:(key "payload")
            ~style:(style [ Width full; Gap (px 8.) ])
            [ body
            ; draft
            ; V.row
                ~key:(key "body-actions")
                [ Palette.button p "Message body action" (act ()) ]
            ]
          |> annotate "Bubble payload"
        ]
      |> fun bubble ->
      Bubble.with_accessibility
        bubble
        (Accessibility.create ~role:Group ~label:"Composed bubble" () |> ok)
      |> ok
    in
    let item =
      if typed_bubble
      then Message.Content.Item.bubble ~key:(key "bubble") bubble
      else Message.Content.Item.element ~key:(key "bubble") (Bubble.view bubble)
    in
    let message =
      Message.create
        (Palette.appearance p)
        ~key:(key "message")
        ~alignment:(if end_aligned then End else Start)
        ~style:(style [ Gap (px 32.) ])
        ?avatar:
          (Option.some_if
             avatar
             (Message.Avatar.create
                ~style:(style [ Width (px 32.); Height (px 32.) ])
                [ V.column
                    ~style:
                      (style
                         [ Width (px 32.)
                         ; Height (px 32.)
                         ; Align_items Center
                         ; Justify_content Center
                         ])
                    [ V.text "ME" ]
                  |> annotate "Message avatar"
                ]))
        ?header:
          (Option.some_if
             header
             (Message.Header.create ?content_inset [ V.text "Message header" ]))
        ~content:(Message.Content.create [ item ] |> ok)
        ?footer:
          (Option.some_if
             footer
             (Message.Footer.create
                ?content_inset
                [ V.column
                    ~style:(style [ Gap (px 4.) ])
                    ([ V.text "Message footer" ]
                     @
                     if expanded_footer
                     then
                       [ V.text "Delivered with care"; V.text "One more useful detail" ]
                     else [])
                ]))
        ()
      |> annotate "Composed message"
    in
    let append =
      E.bind
        (E.of_thunk (fun () -> Resources.append resources))
        ~f:(function
          | Ok chunks -> set_notice (sprintf "Stream chunks: %d" chunks)
          | Error error -> set_notice (Error.to_string_hum error))
    in
    let checkbox label checked on_toggle =
      V.checkbox ~state:(if checked then Checked else Unchecked) ~on_toggle label
    in
    V.column
      ~style:(style [ Gap (px 12.) ])
      [ Palette.text p ~muted:true "A conversation is more than a string of text."
      ; V.column
          ~style:
            (style
               [ Width (px (if compact then 360. else 560.))
               ; Max_width full
               ; Padding_top (px 24.)
               ; Padding_bottom (px 24.)
               ])
          [ message ]
      ; V.row
          ~style:(style [ Gap (px 8.); Wrap Wrap ])
          [ Palette.button p ("Bubble variant: " ^ variant_name) (next_variant ())
          ; Palette.button p ("Bubble edge: " ^ alignment_name) (next_bubble_alignment ())
          ; Palette.button p ("Message inset: " ^ inset_name) (next_inset ())
          ; Palette.button p "Append message chunk" append
          ]
      ; V.row
          ~style:(style [ Gap (px 12.); Wrap Wrap ])
          [ checkbox "Message end aligned" end_aligned toggle_alignment
          ; checkbox "Top reactions" top toggle_top
          ; checkbox "Start reactions" reaction_start toggle_reaction_start
          ; checkbox "Show message reactions" (Option.is_some reactions) toggle_reactions
          ]
      ; V.row
          ~style:(style [ Gap (px 12.); Wrap Wrap ])
          [ checkbox "Show message avatar" avatar toggle_avatar
          ; checkbox "Show message header" header toggle_header
          ; checkbox "Show message footer" footer toggle_footer
          ; checkbox "Expand message footer" expanded_footer toggle_expanded_footer
          ]
      ; V.row
          ~style:(style [ Gap (px 12.); Wrap Wrap ])
          [ checkbox "Typed reaction action" typed_action toggle_typed_action
          ; checkbox "Typed message bubble" typed_bubble toggle_typed_bubble
          ; checkbox "Message Markdown" markdown toggle_markdown
          ; checkbox "Compact message" compact toggle_compact
          ; checkbox "Clip bubble overflow" clip toggle_clip
          ]
      ; Palette.text
          p
          ~muted:true
          (sprintf "Message actions: %d · reactions: %d" actions reaction_actions)
      ; Palette.text p ~muted:true notice
      ]
;;
