open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View
module Editor = Gpuio_eio.Text_input
module Scope = Gpuio_eio.Scope

let component window palette graph =
  let wrap, toggle_wrap = B.toggle ~default_model:true graph in
  let indent, toggle_indent = B.toggle ~default_model:true graph in
  let whitespace, toggle_whitespace = B.toggle ~default_model:false graph in
  let margin, toggle_margin = B.toggle ~default_model:false graph in
  let notice, set_notice =
    B.state "Explore the draft without moving your selection" graph
  in
  let open B.Let_syntax in
  let config =
    let%arr wrap = wrap
    and indent = indent
    and whitespace = whitespace
    and margin = margin in
    let layout =
      Text_area_layout.create
        ~soft_wrap:wrap
        ~wrapping_indent:(if indent then Match_first_line else Flush_left)
        ~show_whitespace:whitespace
        ?cursor_margin_lines:(if margin then Some 3 else None)
        ()
      |> Or_error.ok_exn
    in
    Text_input.Config.create
      ~mode:Multiline
      ~label:"Working notes"
      ~searchable:true
      ~min_rows:8
      ~max_rows:8
      ~layout
      ()
    |> Or_error.ok_exn
  in
  let editor =
    Editor.create
      window
      ~config
      ~initial_text:
        "    A long thought can wrap naturally and keep its indentation, giving every \
         continuation room to breathe. Resize the window or turn wrapping off to \
         explore.\n\n\
         Notes for tomorrow\n\
        \    Sketch the first idea\n\
        \    Gather a few references\n\
        \    Leave room for a surprising detail\n\n\
         A second chapter\n\
        \    Try a smaller experiment\n\
        \    Read the result carefully\n\
        \    Keep the useful parts\n\n\
         Closing thoughts\n\
        \    A little space around the cursor can make a long draft easier to navigate.\n\
        \    Keep writing…"
      graph
  in
  let search = Gpuio_eio.Search_bar.create window ~editor graph in
  let range_busy = B.Expert.Var.create false in
  let range_scope =
    Preview_scope.acquire
      window
      ~name:"selection-bounds"
      ~create:(fun scope ->
        B.Effect.of_thunk (fun () ->
          Scope.on_cancel scope (fun () -> B.Expert.Var.set range_busy false)
          |> Or_error.map ~f:(fun _ -> scope)))
      graph
  in
  let begin_range scope =
    B.Effect.of_thunk (fun () ->
      if (not (Scope.is_active scope)) || B.Expert.Var.get range_busy
      then false
      else (
        B.Expert.Var.set range_busy true;
        true))
  in
  let finish_range scope =
    B.Effect.of_thunk (fun () ->
      if Scope.is_active scope
      then (
        B.Expert.Var.set range_busy false;
        true)
      else false)
  in
  let%arr p = palette
  and editor = editor
  and range_scope = range_scope
  and range_busy = B.Expert.Var.value range_busy
  and search = search
  and wrap = wrap
  and indent = indent
  and whitespace = whitespace
  and margin = margin
  and toggle_wrap = toggle_wrap
  and toggle_indent = toggle_indent
  and toggle_whitespace = toggle_whitespace
  and toggle_margin = toggle_margin
  and notice = notice
  and set_notice = set_notice in
  let report_error error =
    set_notice (Text_input.Command_error.sexp_of_t error |> Sexp.to_string_hum)
  in
  let scroll y =
    let open B.Effect.Let_syntax in
    let offset = Editor_viewport.Offset.create ~x:0. ~y |> Or_error.ok_exn in
    let%bind result = Editor.scroll_to editor offset in
    match result with
    | Ok () ->
      set_notice "Your selection stays in place · use the arrow keys to return to it"
    | Error error -> report_error error
  in
  let inspect =
    let open B.Effect.Let_syntax in
    let%bind result = Editor.read_viewport editor in
    match result with
    | Error error -> report_error error
    | Ok None -> set_notice "The field is not laid out yet"
    | Ok (Some viewport) ->
      set_notice
        (sprintf
           "Layout covers lines %d–%d · scroll %.0f px"
           (Editor_viewport.first_buffer_line viewport + 1)
           (Editor_viewport.buffer_line_limit viewport)
           (Editor_viewport.Offset.y (Editor_viewport.offset viewport)))
  in
  let inspect_range, range_available =
    match range_scope with
    | Preview_scope.Loading | Failed _ -> B.Effect.Ignore, false
    | Ready scope ->
      let action =
        let open B.Effect.Let_syntax in
        let%bind started = begin_range scope in
        if not started
        then B.Effect.Ignore
        else (
          let%bind snapshot = Editor.read_snapshot editor in
          if not (Scope.is_active scope)
          then B.Effect.Ignore
          else (
            let%bind result =
              match snapshot with
              | Error error -> B.Effect.return (Error error)
              | Ok snapshot ->
                Editor.range_bounds
                  editor
                  ~snapshot
                  ~range:(Text_input.Snapshot.selection snapshot)
            in
            let%bind active = finish_range scope in
            if not active
            then B.Effect.Ignore
            else (
              match result with
              | Error Stale_revision ->
                set_notice "The draft changed. Inspect the selection again."
              | Error error -> report_error error
              | Ok None ->
                set_notice
                  "The selection is not in the current layout. Scroll to it and try \
                   again."
              | Ok (Some bounds) ->
                set_notice
                  (sprintf
                     "Selection bounds · x %.1f · y %.1f · %.1f × %.1f px · revision %Ld"
                     (Editor_geometry.x bounds)
                     (Editor_geometry.y bounds)
                     (Editor_geometry.width bounds)
                     (Editor_geometry.height bounds)
                     (Text_input.Revision.to_int64 (Editor_geometry.revision bounds))))))
      in
      action, true
  in
  Palette.card
    p
    ~title:"Room to write"
    [ Palette.text
        p
        ~muted:true
        "Adjust the layout while you write. Your draft, selection and undo history stay \
         with you."
    ; V.row
        ~style:(Style.create_exn [ Gap (Length.px_exn 12.); Wrap Wrap ])
        [ V.switch ~checked:wrap ~on_toggle:toggle_wrap "Wrap long lines"
        ; V.switch ~checked:indent ~on_toggle:toggle_indent "Continue indentation"
        ; V.switch ~checked:whitespace ~on_toggle:toggle_whitespace "Show whitespace"
        ; V.switch ~checked:margin ~on_toggle:toggle_margin "More cursor breathing room"
        ]
    ; Gpuio_eio.Search_bar.wrap
        ~bar_style:
          (Style.create_exn
             [ Background (Background.solid (Palette.surface p))
             ; Border_width 1.
             ; Border_color (Palette.border p)
             ; Radius 10.
             ; Padding (Length.px_exn (Palette.size p 12.))
             ])
        search
        (Editor.view
           ~style:(Style.create_exn [ Height (Length.px_exn (Palette.size p 220.)) ])
           editor)
    ; V.row
        ~style:(Style.create_exn [ Gap (Length.px_exn 10.); Wrap Wrap ])
        [ V.button
            ~config:(Button.Config.create ~focus:Preserve ())
            ~on_click:(Gpuio_eio.Search_bar.open_ search ())
            "Find in notes"
        ; V.button
            ~config:(Button.Config.create ~focus:Preserve ())
            ~on_click:(Gpuio_eio.Search_bar.open_ search ~replace:true ())
            "Find and replace"
        ; V.button
            ~config:(Button.Config.create ~focus:Preserve ())
            ~on_click:(scroll 0.)
            "Back to top"
        ; V.button
            ~config:(Button.Config.create ~focus:Preserve ())
            ~on_click:(scroll 1e9)
            "Jump to end"
        ; V.button
            ~config:(Button.Config.create ~focus:Preserve ())
            ~on_click:inspect
            "Inspect view"
        ; V.button
            ~config:(Button.Config.create ~focus:Preserve ())
            ~disabled:(range_busy || not range_available)
            ~on_click:inspect_range
            (if range_busy then "Inspecting selection…" else "Inspect selection bounds")
        ]
    ; Palette.text p ~muted:true notice
    ]
;;
