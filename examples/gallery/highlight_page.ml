open Core
open Gpuio
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View
module Editor = Gpuio_eio.Text_input
module D = Gpuio_eio.Document
module H = Highlight

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn
let full = Length.percent_exn 100.

module Resources = struct
  type t =
    { document : D.t
    ; mutable appended : bool
    }

  let source =
    "# A notebook\n\n\
     Bring **ideas** into focus. Keep `ideas` in reach.\n\n\
     ```ocaml\n\
     let ideas = 3\n\
     ```\n"
  ;;

  let create app scope =
    E.map
      (D.create app ~scope (Text_source.of_string ~status:Streaming source |> ok))
      ~f:(function
        | Error error -> Error (Error.create_s [%sexp (error : D.Error.t)])
        | Ok document -> Ok { document; appended = false })
  ;;

  let append t =
    if t.appended
    then Ok ()
    else
      Result.map
        (D.append t.document "\nNew ideas arrive. Some ideas stay.\n")
        ~f:(fun () -> t.appended <- true)
  ;;
end

let summary active = function
  | H.State.Pending -> "Finding matches…", None
  | Ready [ count ] ->
    if Int64.equal count.total 0L
    then "No matches", None
    else (
      let selected = Int64.min active Int64.(count.total - 1L) in
      let suffix =
        if Int64.equal count.stored count.total
        then ""
        else sprintf " · %Ld highlighted" count.stored
      in
      ( sprintf "%Ld matches · selected %Ld%s" count.total Int64.(selected + 1L) suffix
      , Some count.total ))
  | Ready _ -> "No search selected", None
  | Invalid_range _ -> "The saved range no longer fits the text", None
  | Capacity _ -> "This search exceeds the available capacity", None
  | Failed _ -> "Search is unavailable for this content", None
;;

let component app window palette graph =
  let resources =
    Preview_scope.acquire
      window
      ~name:"gallery-highlighting"
      ~create:(Resources.create app)
      graph
  in
  let editor =
    Editor.create
      window
      ~initial_text:"ideas"
      ~config:
        (B.return
           (Text_input.Config.create
              ~mode:Single_line
              ~label:"Find in preview"
              ~placeholder:"Find a word…"
              ()
            |> ok))
      graph
  in
  let case_sensitive, toggle_case = B.toggle ~default_model:false graph in
  let whole_word, toggle_word = B.toggle ~default_model:false graph in
  let enabled, toggle_enabled = B.toggle ~default_model:true graph in
  let cursor, set_cursor = B.state None graph in
  let observed, set_observed = B.state None graph in
  let notice, set_notice = B.state "Add a paragraph to see the search update." graph in
  let open B.Let_syntax in
  B.Edge.lifecycle
    ~on_deactivate:
      (let%arr set_observed = set_observed
       and set_cursor = set_cursor
       and set_notice = set_notice in
       E.Many
         [ set_observed None
         ; set_cursor None
         ; set_notice "Add a paragraph to see the search update."
         ])
    graph;
  let%arr p = palette
  and resources = resources
  and editor = editor
  and case_sensitive = case_sensitive
  and toggle_case = toggle_case
  and whole_word = whole_word
  and toggle_word = toggle_word
  and enabled = enabled
  and toggle_enabled = toggle_enabled
  and cursor = cursor
  and set_cursor = set_cursor
  and observed = observed
  and set_observed = set_observed
  and notice = notice
  and set_notice = set_notice in
  let query_text =
    Option.value_map (Editor.snapshot editor) ~default:"ideas" ~f:Text_input.Snapshot.text
  in
  let signature = query_text, case_sensitive, whole_word in
  let active =
    match cursor with
    | Some (previous, active) when [%equal: string * bool * bool] previous signature ->
      active
    | Some _ | None -> 0L
  in
  let appearance = H.Appearance.create ~theme:(Palette.theme p) ~radius:3. () |> ok in
  let config =
    if (not enabled) || String.is_empty query_text
    then Ok H.Config.empty
    else
      let open Or_error.Let_syntax in
      let%bind query = H.Query.create ~case_sensitive ~whole_word query_text in
      let%bind spec = H.Spec.create ~query ~appearance ~active_index:active () in
      H.Config.create [ spec ]
  in
  let status, count =
    if not enabled
    then "Search paused", None
    else if String.is_empty query_text
    then "Enter a word to search", None
    else (
      match config with
      | Error _ -> "Choose a search of at most 4,096 UTF-8 bytes, without NUL", None
      | Ok config ->
        (match observed with
         | Some (previous, sample) when H.Config.equal previous config ->
           summary active sample.H.Observation.state
         | Some _ | None -> "Finding matches…", None))
  in
  let config = Result.ok config |> Option.value ~default:H.Config.empty in
  let step forward =
    match count with
    | None -> E.Ignore
    | Some total ->
      let next =
        if forward
        then if Int64.(active >= total - 1L) then 0L else Int64.(active + 1L)
        else if Int64.equal active 0L
        then Int64.(total - 1L)
        else Int64.(active - 1L)
      in
      set_cursor (Some (signature, next))
  in
  let body =
    match resources with
    | Loading -> [ Palette.text p "Preparing the notebook…" ]
    | Failed error -> [ Palette.text p (Error.to_string_hum error) ]
    | Ready resources ->
      let document =
        Document.Config.create
          ~source:(D.handle resources.document)
          ~mode:Markdown
          ~label:"Searchable notebook"
          ~appearance:(Palette.document_appearance p)
          ~layout:(Viewport 230.)
          ()
        |> ok
      in
      [ V.highlight_scope
          ~key:(Key.of_string_exn "gallery-search")
          ~style:(style [ Gap (px 14.); Width full ])
          ~config
          ~on_update:(fun sample ->
            let clamp =
              match sample.H.Observation.state with
              | Ready [ count ] when Int64.(count.total > 0L && active >= count.total) ->
                set_cursor (Some (signature, Int64.(count.total - 1L)))
              | Pending | Ready _ | Invalid_range _ | Capacity _ | Failed _ -> E.Ignore
            in
            E.Many [ set_observed (Some (config, sample)); clamp ])
          [ Palette.text p ~size:22. "Good ideas start small."
          ; V.row
              [ Palette.text p "Turn id"
              ; V.text
                  ~style:(style [ Foreground (Palette.accent p); Font_weight 700 ])
                  "eas"
              ; Palette.text p " into something useful."
              ]
          ; V.text
              ~style:(style [ Foreground (Palette.foreground p); User_select true ])
              "Keep ideas close. Some ideas need time."
          ; V.highlight_scope
              ~config:H.Config.empty
              [ Palette.text
                  p
                  ~muted:true
                  "Private ideas · this note is excluded from search."
              ]
          ; V.document ~key:(Key.of_string_exn "searchable-notebook") document
          ]
      ; Palette.button
          p
          "Add a paragraph"
          (E.bind
             (E.of_thunk (fun () -> Resources.append resources))
             ~f:(function
               | Ok () -> set_notice "The notebook now includes two more ideas."
               | Error error -> set_notice (Error.to_string_hum error)))
      ; Palette.text p ~muted:true notice
      ]
  in
  let range_config =
    H.Config.create
      [ H.Spec.create
          ~appearance
          ~ranges:[ H.Range.create ~start_byte:0L ~end_byte:6L |> ok ]
          ()
        |> ok
      ]
    |> ok
  in
  V.column
    ~style:(style [ Gap (px 20.) ])
    [ Palette.card
        p
        ~title:"Find the thread"
        [ Editor.view ~style:(style [ Height (px 40.); Width full ]) editor
        ; V.row
            ~style:(style [ Gap (px 12.); Wrap Wrap ])
            [ V.switch ~checked:case_sensitive ~on_toggle:toggle_case "Match case"
            ; V.switch ~checked:whole_word ~on_toggle:toggle_word "Whole words"
            ; V.switch ~checked:enabled ~on_toggle:toggle_enabled "Enable search"
            ]
        ; V.row
            ~style:(style [ Gap (px 10.); Align_items Center ])
            [ Palette.button
                p
                ~disabled:(Option.is_none count)
                "Previous match"
                (step false)
            ; Palette.button p ~disabled:(Option.is_none count) "Next match" (step true)
            ; Palette.text p status
            ]
        ; Palette.text
            p
            ~muted:true
            "The stronger color marks your selected match. Select and copy the text \
             below."
        ]
    ; Palette.card p ~title:"A living notebook" body
    ; Palette.card
        p
        ~title:"A saved passage"
        [ V.highlight_scope
            ~config:range_config
            [ Palette.text p ~size:22. "世界 · a bright idea" ]
        ; Palette.text
            p
            ~muted:true
            "This saved range stays highlighted independently of your search."
        ]
    ]
;;
