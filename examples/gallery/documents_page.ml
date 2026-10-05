open Core
open Gpuio
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View
module D = Gpuio_eio.Document
module Registered = Gpuio_eio.Asset
module Profile = Gpuio_example_document
module Diff_state = Gpuio_gallery_model.Diff_state

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn

module Resources = struct
  type t =
    { markdown : D.t
    ; profile_preview : D.t
    ; html : D.t
    ; code : D.t
    ; diff : D.t
    ; images : D.t
    ; image : Asset.Handle.t
    ; mutable html_fragments : int
    ; mutable fragments : int
    ; mutable diff_fragments : int
    }

  let intro =
    "---\n\
     name: Design notes\n\
     status: Draft\n\
     ---\n\n\
     # A place for ideas\n\n\
     A native **Markdown** preview with Unicode: 世界 · 👨‍👩‍👧‍👦.\n\n\
     - Explore a direction\n\
     - Keep the useful details\n\
     - Share what you learn\n\n\
     <Note>MDX keeps **useful** child content; expressions stay text: {1 + 2}.</Note>\n\n\
     Before [Read the **design** `notes`](gpuio-preview:notes) and [![世界 \
     guide](asset://gallery-link)](gpuio-preview:unicode) after.\n\n\
     | Idea | Next step |\n\
     | --- | --- |\n\
     | Native text | Keep 世界 readable |\n\
     | Small details | Review together |\n\n"
    ^ "\n```ocaml\nlet next_step = \"Explore\"\n```\n\n"
  ;;

  let html =
    "<h1>A reader with room to breathe</h1>\n\
     <p>Native <strong>HTML</strong>, decoded &amp; readable: 世界.</p>\n\
     <p><a href=\"gpuio-preview:html\">Open the <em>design notes</em></a>.</p>\n\
     <blockquote><p>Keep the useful details.</p></blockquote>\n\
     <ul><li>Registered images</li><li>Selectable native text</li></ul>\n\
     <p><a href=\"gpuio-preview:prism\"><img src=\"asset://prism\" alt=\"Prism 世界\" \
     width=\"180\"></a></p>\n\
     <p><img src=\"https://example.invalid/missing.png\" alt=\"Explicit assets only\"></p>\n\
     <pre><code>let greeting = &quot;Hello, 世界&quot;\n\
     </code></pre>\n\
     <table><tr><th>Feature</th><th>Behavior</th></tr><tr><td>Links</td><td>Application \
     requests</td></tr></table>"
  ;;

  let image_examples =
    "Before ![Prism 世界](asset://prism) after.\n\n\
     Before [![Linked 世界](asset://prism)](gpuio-preview:prism) after.\n\n\
     Decoration ![](asset://prism) stays quiet.\n\n\
     Unnamed [![](asset://prism)](gpuio-preview:unnamed) link.\n\n\
     Missing ![Unavailable 世界](asset://missing) stays readable.\n\n\
     ![Reference 世界][prism]\n\n\
     [prism]: asset://prism\n"
  ;;

  let code =
    "open Core\n\n\
     let greeting name =\n\
    \  String.concat [ \"Hello, \"; name; \"!\" ]\n\n\
     let () = greeting \"世界\" |> print_endline\n"
  ;;

  let diff =
    "--- a/greeting.ml\n\
     +++ b/greeting.ml\n\
     @@ -1,2 +1,2 @@\n\
     -let greeting = \"Hello\"\n\
     +let greeting = \"Hello, 世界\"\n\
    \ let answer = 42\n\
     --- a/settings.json\n\
     +++ b/settings.json\n\
     @@ -1,4 +1,4 @@\n\
    \ {\n\
     -  \"theme\": \"light\",\n\
     +  \"theme\": \"dark\",\n\
    \   \"language\": \"世界\"\n\
    \ }\n"
  ;;

  let create app scope =
    let bind computation ~f =
      E.bind computation ~f:(function
        | Error error -> E.return (Error error)
        | Ok value -> f value)
    in
    let create source =
      E.map
        (D.create app ~scope source)
        ~f:(Result.map_error ~f:(fun error -> Error.create_s [%sexp (error : D.Error.t)]))
    in
    let image =
      E.map
        (Registered.register
           app
           ~scope
           (Asset.Source.of_bytes ~format:Pnm Image_samples.gradient_pnm |> ok))
        ~f:(fun result ->
          Result.map result ~f:Registered.handle
          |> Result.map_error ~f:(fun error ->
            Error.create_s [%sexp (error : Registered.Error.t)]))
    in
    bind
      (create
         (Text_source.of_string
            "Open `review` details, or inspect the card below.\n\n\
             ```review-card\n\
             Review the next step\n\
             ```\n\n\
             ```review-scroll\n\
             An independent checklist\n\
             ```\n"
          |> ok))
      ~f:(fun profile_preview ->
        bind
          (create (Text_source.of_string ~status:Streaming html |> ok))
          ~f:(fun html ->
            bind image ~f:(fun image ->
              bind
                (create (Text_source.of_string image_examples |> ok))
                ~f:(fun images ->
                  bind
                    (create (Text_source.of_string ~status:Streaming intro |> ok))
                    ~f:(fun markdown ->
                      bind
                        (create (Text_source.of_string code |> ok))
                        ~f:(fun code ->
                          E.map
                            (create (Text_source.of_string ~status:Streaming diff |> ok))
                            ~f:
                              (Result.map ~f:(fun diff ->
                                 { markdown
                                 ; profile_preview
                                 ; html
                                 ; code
                                 ; diff
                                 ; images
                                 ; image
                                 ; html_fragments = 0
                                 ; fragments = 0
                                 ; diff_fragments = 0
                                 }))))))))
  ;;

  let append_html t =
    if t.html_fragments = 6
    then Ok t.html_fragments
    else (
      let next = t.html_fragments + 1 in
      Result.map
        (D.append
           t.html
           (sprintf "<p>Finding %d: <strong>let ideas grow</strong>.</p>" next))
        ~f:(fun () ->
          t.html_fragments <- next;
          next))
  ;;

  let reset_html t = Result.map (D.reset t.html html) ~f:(fun () -> t.html_fragments <- 0)

  let append t =
    if t.fragments = 6
    then Ok t.fragments
    else (
      let next = t.fragments + 1 in
      let fragment =
        sprintf
          "## Finding %d\n\n\
           Small changes can make a useful difference.\n\n\
           ```ocaml\n\
           let step = %d\n\
           ```\n\n\
           [Explore finding %d](gpuio-preview:finding-%d)\n\n"
          next
          next
          next
          next
      in
      Result.map (D.append t.markdown fragment) ~f:(fun () ->
        t.fragments <- next;
        next))
  ;;

  let reset t = Result.map (D.reset t.markdown intro) ~f:(fun () -> t.fragments <- 0)

  let append_diff t =
    if t.diff_fragments = 3
    then Ok t.diff_fragments
    else (
      let next = t.diff_fragments + 1 in
      let fragment =
        sprintf
          "--- /dev/null\n\
           +++ b/worker-%d.rs\n\
           @@ -0,0 +1,3 @@\n\
           +fn main() {\n\
           +    println!(\"Hello, 世界\");\n\
           +}\n"
          next
      in
      Result.map (D.append t.diff fragment) ~f:(fun () ->
        t.diff_fragments <- next;
        next))
  ;;

  let reset_diff t = Result.map (D.reset t.diff diff) ~f:(fun () -> t.diff_fragments <- 0)
end

module Mode = struct
  type t =
    | Markdown
    | Code
    | Diff
    | Images
    | Html
  [@@deriving equal]

  let all = [ Markdown; Html; Code; Diff; Images ]

  let label = function
    | Markdown -> "Markdown"
    | Code -> "Code"
    | Diff -> "Diff"
    | Images -> "Image alternatives"
    | Html -> "HTML"
  ;;
end

module Defaults_policy = struct
  type t =
    | Inherit
    | Builtin
    | Compact
  [@@deriving equal]
end

let component app window palette graph =
  let trace_profile =
    Array.exists (Sys.get_argv ()) ~f:(String.equal "--trace-document-profile")
  in
  let resources =
    Preview_scope.acquire
      window
      ~name:"gallery-documents"
      ~create:(Resources.create app)
      graph
  in
  let mode, set_mode = B.state Mode.Markdown graph in
  let defaults_policy, set_defaults_policy = B.state Defaults_policy.Inherit graph in
  let notice, set_notice = B.state "Ready to explore" graph in
  let highlight, toggle_highlight = B.toggle ~default_model:false graph in
  let refined, toggle_refined = B.toggle ~default_model:false graph in
  let frontmatter, toggle_frontmatter = B.toggle ~default_model:false graph in
  let descriptions, toggle_descriptions = B.toggle ~default_model:true graph in
  let native_profile, toggle_native_profile = B.toggle ~default_model:false graph in
  let warm_profile, toggle_warm_profile = B.toggle ~default_model:false graph in
  let custom_actions, toggle_custom_actions = B.toggle ~default_model:false graph in
  let enable_actions, toggle_enable_actions = B.toggle ~default_model:true graph in
  let native_copy, toggle_native_copy = B.toggle ~default_model:true graph in
  let mdx, toggle_mdx = B.toggle ~default_model:false graph in
  let preview_expanded, toggle_preview = B.toggle ~default_model:false graph in
  let preview_notice, set_preview_notice = B.state "Measuring preview…" graph in
  let copy_markdown, toggle_copy_markdown = B.toggle ~default_model:false graph in
  let diff_state, inject_diff =
    B.state_machine0
      ~default_model:Diff_state.initial
      ~apply_action:(fun _ state action -> Diff_state.apply state action)
      graph
  in
  let open B.Let_syntax in
  B.Edge.lifecycle
    ~on_deactivate:
      (let%arr set_notice = set_notice
       and inject_diff = inject_diff in
       E.Many [ set_notice "Ready to explore"; inject_diff Diff_state.Action.Reset ])
    graph;
  let%arr resources = resources
  and p = palette
  and defaults_policy = defaults_policy
  and set_defaults_policy = set_defaults_policy
  and mode = mode
  and set_mode = set_mode
  and notice = notice
  and set_notice = set_notice
  and highlight = highlight
  and toggle_highlight = toggle_highlight
  and refined = refined
  and toggle_refined = toggle_refined
  and frontmatter = frontmatter
  and toggle_frontmatter = toggle_frontmatter
  and descriptions = descriptions
  and toggle_descriptions = toggle_descriptions
  and native_profile = native_profile
  and toggle_native_profile = toggle_native_profile
  and warm_profile = warm_profile
  and toggle_warm_profile = toggle_warm_profile
  and custom_actions = custom_actions
  and toggle_custom_actions = toggle_custom_actions
  and enable_actions = enable_actions
  and toggle_enable_actions = toggle_enable_actions
  and native_copy = native_copy
  and toggle_native_copy = toggle_native_copy
  and mdx = mdx
  and toggle_mdx = toggle_mdx
  and copy_markdown = copy_markdown
  and toggle_copy_markdown = toggle_copy_markdown
  and preview_expanded = preview_expanded
  and toggle_preview = toggle_preview
  and preview_notice = preview_notice
  and set_preview_notice = set_preview_notice
  and diff_state = diff_state
  and inject_diff = inject_diff in
  match resources with
  | Loading -> Palette.text p "Preparing document previews…"
  | Failed error ->
    Palette.text p ("Unable to prepare documents: " ^ Error.to_string_hum error)
  | Ready resources ->
    let run f =
      E.bind (E.of_thunk f) ~f:(function
        | Ok message -> set_notice message
        | Error error -> set_notice (Error.to_string_hum error))
    in
    let source, document_mode, label =
      match mode with
      | Markdown -> resources.markdown, Document.Mode.Markdown, "Markdown preview"
      | Html -> resources.html, Html, "HTML reader"
      | Code -> resources.code, Code Document.Language.ocaml, "Code preview"
      | Diff -> resources.diff, Diff, "Diff preview"
      | Images -> resources.images, Markdown, "Image alternatives preview"
    in
    let appearance = Palette.document_appearance p in
    let text_style =
      if not refined
      then None
      else
        Some
          (Document.Style.create
             ~colors:
               [ Foreground, Palette.foreground p
               ; Muted_foreground, Palette.muted p
               ; Link, Palette.accent p
               ; Code_background, Palette.surface p
               ; Border, Palette.border p
               ]
             ~paragraph_gap_rem:1.4
             ~heading_sizes:
               (Document.Style.Heading_sizes.create
                  ~h1:32.
                  ~h2:26.
                  ~h3:22.
                  ~h4:19.
                  ~h5:17.
                  ~h6:15.
                |> ok)
             ~inline_code:
               (Document.Style.Inline_code.create
                  ~foreground:(Palette.accent p)
                  ~font_weight:600
                  ()
                |> ok)
             ~code_block:
               (style
                  [ Padding (px 16.)
                  ; Radius 12.
                  ; Border_width 1.
                  ; Border_color (Palette.border p)
                  ])
             ~table:
               (style [ Border_width 1.; Border_color (Palette.border p); Radius 8. ])
             ~table_head:
               (style
                  [ Background (Background.solid (Palette.surface p)); Font_weight 600 ])
             ~table_cell:(style [ Padding (px 12.) ])
             ()
           |> ok)
    in
    let rich_actions = Mode.equal mode Markdown || Mode.equal mode Html in
    let actions =
      if not rich_actions
      then Document.Actions.Config.default
      else (
        let action id label =
          Document.Actions.Action.create
            ~id:(Document.Actions.Id.of_string id |> ok)
            ~label
            ~enabled:enable_actions
            ()
          |> ok
        in
        Document.Actions.Config.create
          ~copy_code:native_copy
          ~copy_table:native_copy
          ~code:(if custom_actions then [ action "inspect" "Inspect snippet" ] else [])
          ~table:(if custom_actions then [ action "summary" "Summarize table" ] else [])
          ()
        |> ok)
    in
    let config =
      Document.Config.create
        ~source:(D.handle source)
        ~mode:document_mode
        ~actions
        ~markdown_options:
          (if Mode.equal mode Markdown
           then
             Document.Markdown_options.create
               ~frontmatter:
                 (if not frontmatter
                  then Disabled
                  else if descriptions
                  then Description_list
                  else Code_block)
               ~mdx
               ()
           else Document.Markdown_options.default)
        ?text_style:
          (match document_mode with
           | Markdown | Html -> text_style
           | Code _ | Diff -> None)
        ~label
        ~appearance
        ~selection_format:(if copy_markdown then Markdown else Plain_text)
        ~layout:(Viewport (if Mode.equal mode Images then 450. else 350.))
        ~images:
          (if Mode.equal mode Images || Mode.equal mode Html
           then [ "asset://prism", resources.image ]
           else [])
        ~search:(if highlight then "let" else "")
        ?path:
          (match mode with
           | Markdown | Diff | Images | Html -> None
           | Code -> Some "greeting.ml")
        ?diff:
          (match mode with
           | Diff -> Some (Diff_state.config diff_state)
           | Markdown | Code | Images | Html -> None)
        ()
      |> ok
    in
    let attach_profile view =
      if not (native_profile && rich_actions)
      then view
      else
        V.with_document_profile
          view
          (Profile.instance
             ~accent:(if warm_profile then Amber else Indigo)
             ~generation:1L
           |> ok)
          ~on_event:(fun event ->
            let signal =
              [%sexp_of: Profile.Event.t Document.Profile.Signal.t] event.signal
            in
            E.Many
              [ (if trace_profile
                 then
                   E.of_thunk (fun () ->
                     Eio.traceln
                       "GALLERY_DOCUMENT_PROFILE revision=%Ld signal=%s"
                       event.source_revision
                       (Sexp.to_string signal))
                 else E.Ignore)
              ; set_notice
                  (sprintf
                     "Profile · revision %Ld · %s"
                     event.source_revision
                     (Sexp.to_string_hum signal))
              ])
        |> ok
    in
    V.column
      ~style:(style [ Gap (px 20.) ])
      [ V.row
          ~style:(style [ Gap (px 10.) ])
          (List.map Mode.all ~f:(fun candidate ->
             Palette.button
               p
               ~selected:(Mode.equal mode candidate)
               (Mode.label candidate)
               (set_mode candidate)))
      ; Palette.card
          p
          ~title:"Words that keep their shape"
          [ (if Mode.equal mode Markdown
             then
               V.row
                 ~style:(style [ Gap (px 12.); Wrap Wrap ])
                 [ V.switch
                     ~checked:frontmatter
                     ~on_toggle:toggle_frontmatter
                     "YAML frontmatter"
                 ; V.switch
                     ~checked:descriptions
                     ~on_toggle:toggle_descriptions
                     "Metadata descriptions"
                 ; V.switch
                     ~checked:mdx
                     ~on_toggle:toggle_mdx
                     "MDX syntax (no evaluation)"
                 ]
             else V.column ~style:(style [ Display Hidden ]) [])
          ; (match mode with
             | Markdown | Html | Images ->
               V.switch
                 ~checked:refined
                 ~on_toggle:toggle_refined
                 "Refined reader styling"
             | Code | Diff -> V.column ~style:(style [ Display Hidden ]) [])
          ; (match mode with
             | Diff ->
               V.row
                 ~style:(style [ Gap (px 10.); Wrap Wrap ])
                 [ V.switch
                     ~checked:(Diff_state.controlled diff_state)
                     ~on_toggle:(inject_diff Diff_state.Action.Toggle_controlled)
                     "Application controls expansion"
                 ; V.switch
                     ~checked:(Diff_state.word_diff diff_state)
                     ~on_toggle:(inject_diff Diff_state.Action.Toggle_words)
                     "Emphasize changed words"
                 ; Palette.button
                     p
                     "Append a file"
                     (run (fun () ->
                        Result.map (Resources.append_diff resources) ~f:(fun n ->
                          sprintf "Appended files: %d / 3" n)))
                 ; Palette.button
                     p
                     "Reset diff"
                     (E.bind
                        (E.of_thunk (fun () -> Resources.reset_diff resources))
                        ~f:(function
                          | Error error -> set_notice (Error.to_string_hum error)
                          | Ok () ->
                            E.Many
                              [ inject_diff Diff_state.Action.Reset
                              ; set_notice "Diff reset"
                              ]))
                 ]
             | Images ->
               Palette.text
                 p
                 ~muted:true
                 "Registered images, useful alternatives, and quiet decorations."
             | Html ->
               V.row
                 ~style:(style [ Gap (px 10.); Wrap Wrap ])
                 [ Palette.button
                     p
                     "Append HTML"
                     (run (fun () ->
                        Result.map (Resources.append_html resources) ~f:(fun n ->
                          sprintf "HTML findings: %d / 6" n)))
                 ; Palette.button
                     p
                     "Reset HTML"
                     (run (fun () ->
                        Result.map (Resources.reset_html resources) ~f:(fun () ->
                          "HTML reset")))
                 ; V.switch ~checked:highlight ~on_toggle:toggle_highlight "Highlight let"
                 ]
             | Markdown | Code ->
               V.row
                 ~style:(style [ Gap (px 10.); Wrap Wrap ])
                 [ Palette.button
                     p
                     "Append a finding"
                     (run (fun () ->
                        Result.map (Resources.append resources) ~f:(fun n ->
                          sprintf "Appended findings: %d / 6" n)))
                 ; Palette.button
                     p
                     "Reset document"
                     (run (fun () ->
                        Result.map (Resources.reset resources) ~f:(fun () ->
                          "Document reset")))
                 ; (if Mode.equal mode Markdown
                    then
                      Palette.button
                        p
                        "Try unsupported YAML"
                        (run (fun () ->
                           Result.map
                             (D.reset
                                resources.markdown
                                "---\n\
                                 name: \"Quoted metadata\"\n\
                                 tags: [native, readable]\n\
                                 ---\n\n\
                                 # Source stays visible\n\n\
                                 This YAML falls back to code; Reset document restores \
                                 the description example.\n")
                             ~f:(fun () -> "Unsupported YAML sample loaded")))
                    else V.column ~style:(style [ Display Hidden ]) [])
                 ; V.switch ~checked:highlight ~on_toggle:toggle_highlight "Highlight let"
                 ; V.switch
                     ~checked:copy_markdown
                     ~on_toggle:toggle_copy_markdown
                     "Copy selection as Markdown"
                 ])
          ; (if rich_actions
             then
               V.row
                 ~style:(style [ Gap (px 12.); Wrap Wrap ])
                 [ V.switch
                     ~checked:native_profile
                     ~on_toggle:toggle_native_profile
                     "Native document profile"
                 ; V.switch
                     ~checked:warm_profile
                     ~on_toggle:toggle_warm_profile
                     "Amber code highlights"
                 ; V.switch
                     ~checked:custom_actions
                     ~on_toggle:toggle_custom_actions
                     "Custom block actions"
                 ; V.switch
                     ~checked:enable_actions
                     ~on_toggle:toggle_enable_actions
                     "Enable custom actions"
                 ; V.switch
                     ~checked:native_copy
                     ~on_toggle:toggle_native_copy
                     "Native Copy buttons"
                 ]
             else V.column ~style:(style [ Display Hidden ]) [])
          ; V.document
              ~key:(Key.of_string_exn (Mode.label mode))
              ~style:
                (style
                   [ Foreground (Palette.foreground p)
                   ; Background (Background.solid (Palette.background p))
                   ; Radius 10.
                   ])
              ?on_action:
                (if rich_actions && custom_actions
                 then
                   Some
                     (fun (event : Document.Actions.Event.t) ->
                       let detail =
                         match event.block with
                         | Code { language; code } ->
                           sprintf
                             "Snippet: %d UTF-8 bytes · %s"
                             (String.length code)
                             (Option.value language ~default:"plain text")
                         | Table { headers; rows; markdown = _ } ->
                           sprintf
                             "Table: %d columns · %d body rows"
                             (List.length headers)
                             (List.length rows)
                       in
                       set_notice
                         (sprintf
                            "%s · source revision %Ld · %s"
                            detail
                            event.source_revision
                            (Sexp.to_string_hum
                               ([%sexp_of: Document.Activation.t] event.activation))))
                 else None)
              ~on_navigate:(fun navigation ->
                set_notice
                  (match navigation with
                   | Link { url; activation } ->
                     sprintf
                       "Link requested: %s %s"
                       url
                       (Sexp.to_string_hum
                          ([%sexp_of: Document.Activation.t option] activation))
                   | Line { path; side; line } ->
                     sprintf
                       "Line requested: %s %s:%d"
                       (match side with
                        | Before -> "before"
                        | After -> "after")
                       (Option.value path ~default:"document")
                       line))
              ?on_diff:
                (match mode with
                 | Diff ->
                   Some (fun event -> inject_diff (Diff_state.Action.Observe event))
                 | Markdown | Code | Images | Html -> None)
              config
            |> attach_profile
          ; (if native_profile && rich_actions
             then
               V.column
                 ~style:(style [ Gap (px 8.) ])
                 [ Palette.text
                     p
                     ~muted:true
                     "Markdown profile · inline and block controls"
                 ; V.document
                     ~key:(Key.of_string_exn "native-profile-preview")
                     (Document.Config.create
                        ~source:(D.handle resources.profile_preview)
                        ~mode:Markdown
                        ~appearance
                        ~label:"Native review controls"
                        ~layout:Flow
                        ()
                      |> ok)
                   |> attach_profile
                 ]
             else V.column ~style:(style [ Display Hidden ]) [])
          ; (if Mode.equal mode Markdown
             then
               Palette.card
                 p
                 ~title:"Application reader defaults"
                 [ Palette.text
                     p
                     ~muted:true
                     "Launch with --document-defaults for wider paragraph spacing and \
                      Markdown selection copy. Compare inherited, built-in and explicit \
                      settings below."
                 ; V.row
                     ~style:(style [ Gap (px 8.); Wrap Wrap ])
                     (List.map
                        [ Defaults_policy.Inherit, "Inherit"
                        ; Builtin, "Built-in"
                        ; Compact, "Compact override"
                        ]
                        ~f:(fun (policy, label) ->
                          Palette.button
                            p
                            ~selected:(Defaults_policy.equal defaults_policy policy)
                            label
                            (set_defaults_policy policy)))
                 ; (let config =
                      Document.Config.create
                        ~source:(D.handle resources.profile_preview)
                        ~mode:Markdown
                        ~appearance
                        ~label:"Reader defaults preview"
                        ~layout:Flow
                        ()
                      |> ok
                    in
                    let config =
                      match defaults_policy with
                      | Inherit -> config
                      | Builtin ->
                        Document.Config.with_overrides
                          config
                          ~text_style:Builtin
                          ~selection_format:Builtin
                          ()
                        |> ok
                      | Compact ->
                        Document.Config.with_overrides
                          config
                          ~text_style:
                            (Value (Document.Style.create ~paragraph_gap_rem:0.4 () |> ok))
                          ~selection_format:(Value Plain_text)
                          ()
                        |> ok
                    in
                    V.document ~key:(Key.of_string_exn "reader-defaults-preview") config
                    |> V.without_document_profile
                    |> ok)
                 ]
             else V.column ~style:(style [ Display Hidden ]) [])
          ; Palette.text
              p
              ~muted:true
              (match mode with
               | Diff -> Diff_state.notice diff_state ^ "\n" ^ notice
               | Markdown | Code | Images | Html -> notice)
          ; Palette.text
              p
              ~muted:true
              "Select and copy native text. Links and line references return an \
               application request."
          ]
      ; V.row
          ~style:(style [ Gap (px 12.); Align_items Center ])
          [ Palette.button
              p
              (if preview_expanded then "Collapse preview" else "Expand preview")
              toggle_preview
          ; Palette.text p ~muted:true preview_notice
          ]
      ; V.document
          ~key:(Key.of_string_exn "line-preview")
          ~on_preview:(fun event ->
            set_preview_notice
              (match event.Document.Preview.Event.state with
               | Pending -> "Preparing preview…"
               | Collapsed -> "Body collapsed"
               | Source_view -> "Showing source; rich preview unavailable"
               | Rich { clamped = true } -> "More content available"
               | Rich { clamped = false } -> "All content visible"))
          (Document.Config.create
             ~source:(D.handle resources.markdown)
             ~mode:Markdown
             ~appearance
             ~layout:Flow
             ~label:"Compact preview"
             ?max_lines:(if preview_expanded then None else Some 6)
             ()
           |> ok)
      ]
;;
