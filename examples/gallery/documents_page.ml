open Core
open Gpuio
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View
module D = Gpuio_eio.Document
module Registered = Gpuio_eio.Asset
module Diff_state = Gpuio_gallery_model.Diff_state

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn

module Resources = struct
  type t =
    { markdown : D.t
    ; code : D.t
    ; diff : D.t
    ; images : D.t
    ; image : Asset.Handle.t
    ; mutable fragments : int
    ; mutable diff_fragments : int
    }

  let intro =
    "# A place for ideas\n\n\
     A native **Markdown** preview with Unicode: 世界 · 👨‍👩‍👧‍👦.\n\n\
     - Explore a direction\n\
     - Keep the useful details\n\
     - Share what you learn\n\n\
     Before [Read the **design** `notes`](gpuio-preview:notes) and [![世界 \
     guide](asset://gallery-link)](gpuio-preview:unicode) after.\n\n\
     | Idea | Next step |\n\
     | --- | --- |\n\
     | Native text | Keep 世界 readable |\n\
     | Small details | Review together |\n\n"
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
                         ; code
                         ; diff
                         ; images
                         ; image
                         ; fragments = 0
                         ; diff_fragments = 0
                         }))))))
  ;;

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
  [@@deriving equal]

  let all = [ Markdown; Code; Diff; Images ]

  let label = function
    | Markdown -> "Markdown"
    | Code -> "Code"
    | Diff -> "Diff"
    | Images -> "Image alternatives"
  ;;
end

let component app window palette graph =
  let resources =
    Preview_scope.acquire
      window
      ~name:"gallery-documents"
      ~create:(Resources.create app)
      graph
  in
  let mode, set_mode = B.state Mode.Markdown graph in
  let notice, set_notice = B.state "Ready to explore" graph in
  let highlight, toggle_highlight = B.toggle ~default_model:false graph in
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
  and mode = mode
  and set_mode = set_mode
  and notice = notice
  and set_notice = set_notice
  and highlight = highlight
  and toggle_highlight = toggle_highlight
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
      | Code -> resources.code, Code Document.Language.ocaml, "Code preview"
      | Diff -> resources.diff, Diff, "Diff preview"
      | Images -> resources.images, Markdown, "Image alternatives preview"
    in
    let appearance = Palette.document_appearance p in
    let config =
      Document.Config.create
        ~source:(D.handle source)
        ~mode:document_mode
        ~label
        ~appearance
        ~layout:(Viewport (if Mode.equal mode Images then 450. else 350.))
        ~images:
          (if Mode.equal mode Images then [ "asset://prism", resources.image ] else [])
        ~search:(if highlight then "let" else "")
        ?path:
          (match mode with
           | Markdown | Diff | Images -> None
           | Code -> Some "greeting.ml")
        ?diff:
          (match mode with
           | Diff -> Some (Diff_state.config diff_state)
           | Markdown | Code | Images -> None)
        ()
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
          [ (match mode with
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
                 ; V.switch ~checked:highlight ~on_toggle:toggle_highlight "Highlight let"
                 ])
          ; V.document
              ~key:(Key.of_string_exn (Mode.label mode))
              ~style:
                (style
                   [ Foreground (Palette.foreground p)
                   ; Background (Background.solid (Palette.background p))
                   ; Radius 10.
                   ])
              ~on_navigate:(fun navigation ->
                set_notice
                  (match navigation with
                   | Link link -> "Link requested: " ^ link
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
                 | Markdown | Code | Images -> None)
              config
          ; Palette.text
              p
              ~muted:true
              (match mode with
               | Diff -> Diff_state.notice diff_state ^ "\n" ^ notice
               | Markdown | Code | Images -> notice)
          ; Palette.text
              p
              ~muted:true
              "Select and copy native text. Links and line references return an \
               application request."
          ]
      ]
;;
