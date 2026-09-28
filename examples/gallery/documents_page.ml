open Core
open Gpuio
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View
module D = Gpuio_eio.Document

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn

module Resources = struct
  type t =
    { markdown : D.t
    ; code : D.t
    ; diff : D.t
    ; mutable fragments : int
    }

  let intro =
    "# A place for ideas\n\n\
     A native **Markdown** preview with Unicode: 世界 · 👨‍👩‍👧‍👦.\n\n\
     - Explore a direction\n\
     - Keep the useful details\n\
     - Share what you learn\n\n\
     Before [Read the **design** notes](gpuio-preview:notes) and [世界 \
     guide](gpuio-preview:unicode) after.\n\n"
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
    \ let answer = 42\n"
  ;;

  let create app scope =
    let create source =
      E.map
        (D.create app ~scope source)
        ~f:(Result.map_error ~f:(fun error -> Error.create_s [%sexp (error : D.Error.t)]))
    in
    E.bind
      (create (Text_source.of_string ~status:Streaming intro |> ok))
      ~f:(function
        | Error error -> E.return (Error error)
        | Ok markdown ->
          E.bind
            (create (Text_source.of_string code |> ok))
            ~f:(function
              | Error error -> E.return (Error error)
              | Ok code ->
                E.map
                  (create (Text_source.of_string diff |> ok))
                  ~f:(Result.map ~f:(fun diff -> { markdown; code; diff; fragments = 0 }))))
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
end

module Mode = struct
  type t =
    | Markdown
    | Code
    | Diff
  [@@deriving equal]

  let all = [ Markdown; Code; Diff ]

  let label = function
    | Markdown -> "Markdown"
    | Code -> "Code"
    | Diff -> "Diff"
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
  let open B.Let_syntax in
  B.Edge.lifecycle
    ~on_deactivate:(B.map set_notice ~f:(fun set_notice -> set_notice "Ready to explore"))
    graph;
  let%arr resources = resources
  and p = palette
  and mode = mode
  and set_mode = set_mode
  and notice = notice
  and set_notice = set_notice
  and highlight = highlight
  and toggle_highlight = toggle_highlight in
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
    in
    let appearance = Palette.document_appearance p in
    let config =
      Document.Config.create
        ~source:(D.handle source)
        ~mode:document_mode
        ~label
        ~appearance
        ~layout:(Viewport 350.)
        ~search:(if highlight then "let" else "")
        ?path:
          (match mode with
           | Markdown -> None
           | Code | Diff -> Some "greeting.ml")
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
          [ V.row
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
              ]
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
              config
          ; Palette.text p ~muted:true notice
          ; Palette.text
              p
              ~muted:true
              "Select and copy native text. Links and line references return an \
               application request."
          ]
      ]
;;
