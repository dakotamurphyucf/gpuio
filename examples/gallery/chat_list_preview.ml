open Core
open Gpuio
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View
module L = Gpuio_bonsai.Virtual_list
module P = Presentation
module D = Gpuio_eio.Document
module Editor = Gpuio_eio.Text_input

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn
let full = Length.percent_exn 100.
let key = Key.of_string_exn
let last = 100

module Source = struct
  type t =
    { document : D.t
    ; mutable chunks : int
    }

  let create app scope =
    E.map
      (D.create
         app
         ~scope
         (Text_source.of_string
            ~status:Streaming
            "An answer with room to grow.\n\n```ocaml\nlet ready = true\n```\n"
          |> ok))
      ~f:(function
        | Ok document -> Ok { document; chunks = 0 }
        | Error error -> Error (Error.create_s [%sexp (error : D.Error.t)]))
  ;;

  let append t =
    if t.chunks >= 12
    then Ok t.chunks
    else
      Result.map
        (D.append
           t.document
           (sprintf
              "\n\nUpdate **%d** · Keep the conversation moving. 京都\n"
              (t.chunks + 1)))
        ~f:(fun () ->
          t.chunks <- t.chunks + 1;
          t.chunks)
  ;;
end

let annotate view label =
  V.with_accessibility view (Accessibility.create ~role:Group ~label () |> ok) |> ok
;;

let component app window palette graph =
  let source =
    Preview_scope.acquire
      window
      ~name:"gallery-managed-chat"
      ~create:(Source.create app)
      graph
  in
  let actions, react =
    B.state_machine0 ~default_model:0 ~apply_action:(fun _ n () -> n + 1) graph
  in
  let notice, set_notice = B.state "Managed chunks: 0" graph in
  let editor =
    Editor.create
      window
      ~initial_text:"A draft outside the rows"
      ~config:
        (B.return
           (Text_input.Config.create ~mode:Single_line ~label:"Transcript draft" () |> ok))
      graph
  in
  let rows =
    List_collection.of_alist (module Int) (List.init last ~f:(fun i -> i + 1, ()))
    |> ok
    |> B.return
  in
  let open B.Let_syntax in
  B.Edge.lifecycle
    ~on_deactivate:
      (let%arr set_notice = set_notice in
       set_notice "Managed chunks: 0")
    graph;
  let list =
    L.component
      (module Int)
      rows
      ~row_key:Key.of_int
      ~config:
        (Virtual_list.Config.create
           ~height:(Estimated 140.)
           ~overscan:180.
           ~max_active:12
           ~scroll:Follow_tail_when_at_end
           ()
         |> ok)
      ~accessibility:
        (B.return (Accessibility.create ~label:"Managed conversation" () |> ok))
      ~style:
        (B.return
           (style [ Width (px 560.); Max_width full; Height (px 360.); Shrink 0. ]))
      ~render_row:(fun ~key:id ~data:_ ~lifetime:_ _graph ->
        let%arr id = id
        and source = source
        and p = palette
        and react = react in
        match source with
        | Loading -> Palette.text p "Preparing conversation…"
        | Failed error -> Palette.text p (Error.to_string_hum error)
        | Ready source ->
          let body =
            if id = last
            then
              V.document
                ~key:(key "body")
                (Document.Config.create
                   ~source:(D.handle source.document)
                   ~label:"Managed streamed answer"
                   ~mode:Markdown
                   ~layout:Flow
                   ~appearance:(Palette.document_appearance p)
                   ()
                 |> ok)
            else
              V.text
                ~key:(key "body")
                (sprintf
                   "A useful idea from message %d.%s"
                   id
                   (if id % 3 = 0 then "\nA second line adds a little context." else ""))
          in
          let reactions =
            P.Bubble.Reactions.create
              [ P.Bubble.Reactions.Item.action
                  ~key:(key "react")
                  ~on_click:(fun () -> react ())
                  (sprintf "React to row %d" id)
              ]
            |> ok
          in
          let bubble =
            P.Bubble.create
              (Palette.appearance p)
              ~variant:(if id % 2 = 0 then Secondary else Tinted)
              ~style:(style [ Width full ])
              ~reactions
              [ body ]
          in
          let message =
            P.Message.create
              (Palette.appearance p)
              ~alignment:(if id % 2 = 0 then Start else End)
              ~header:(P.Message.Header.create [ V.text (sprintf "Message %d" id) ])
              ~content:
                (P.Message.Content.create
                   [ P.Message.Content.Item.bubble ~key:(key "bubble") bubble ]
                 |> ok)
              ()
            |> fun view -> annotate view (sprintf "Managed message %d" id)
          in
          (* Absolute reactions need measured room; this padding also separates rows. *)
          V.column
            ~style:
              (style
                 [ Width full
                 ; Shrink 0.
                 ; Padding_top (px 12.)
                 ; Padding_bottom (px 28.)
                 ; Padding_left (px 12.)
                 ; Padding_right (px 12.)
                 ])
            [ message ])
      graph
  in
  let%arr list = list
  and source = source
  and p = palette
  and editor = editor
  and actions = actions
  and notice = notice
  and set_notice = set_notice in
  match list, source with
  | Error error, _ | _, Failed error -> Palette.text p (Error.to_string_hum error)
  | Ok _, Loading -> Palette.text p "Preparing conversation…"
  | Ok list, Ready source ->
    let append =
      E.bind
        (E.of_thunk (fun () -> Source.append source))
        ~f:(function
          | Ok n -> set_notice (sprintf "Managed chunks: %d" n)
          | Error error -> set_notice (Error.to_string_hum error))
    in
    let controller = L.Output.controller list in
    V.column
      ~style:(style [ Gap (px 12.) ])
      [ Palette.text
          p
          ~muted:true
          "History stays put while a response grows. Reactions have room of their own."
      ; L.Output.view list
      ; Editor.view
          ~style:(style [ Width (px 560.); Max_width full; Height (px 36.) ])
          editor
      ; V.row
          ~style:(style [ Gap (px 8.); Wrap Wrap ])
          [ Palette.button
              p
              "Transcript history"
              (L.Controller.scroll_to controller ~offset:8. 50 |> ok)
          ; Palette.button p "Transcript latest" (L.Controller.jump_to_latest controller)
          ; Palette.button p "Append transcript chunk" append
          ]
      ; Palette.text
          p
          ~muted:true
          (sprintf
             "%s · retained rows: %d/12 · reactions: %d"
             notice
             (L.Output.active_rows list)
             actions)
      ]
;;
