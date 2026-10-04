open Core
open Gpuio
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View
module L = Gpuio_bonsai.Selectable_list
module P = Gpuio_eio.List_search
module C = List_collection
module Editor = Gpuio_eio.Text_input

let ok = Or_error.ok_exn
let px = Length.px_exn
let style = Style.create_exn

module Entry = struct
  type t =
    { title : string
    ; detail : string
    }
end

type t =
  { search : (int, Entry.t, Int.comparator_witness) P.t
  ; fail_next : bool ref
  }

let initial () =
  List.concat_mapi
    [ "Design notes"; "Research library"; "Project ideas"; "Reading room" ]
    ~f:(fun group title ->
      (-(group + 1), { Entry.title; detail = "Collection" })
      :: List.init 250 ~f:(fun index ->
        let id = (group * 250) + index + 1 in
        ( id
        , { Entry.title = sprintf "%s · %04d" title id
          ; detail = "A small idea, ready for a closer look."
          } )))
  |> C.of_alist (module Int)
  |> ok
;;

let disabled key = key < 0 || key % 11 = 0
let group key = if key >= 10_000 then 4 else (key - 1) / 250

let matches items query =
  C.to_alist items
  |> List.fold ~init:Int.Map.empty ~f:(fun groups (key, entry) ->
    if
      key < 0
      || not (String.is_substring (String.lowercase entry.Entry.title) ~substring:query)
    then groups
    else Map.add_multi groups ~key:(group key) ~data:key)
  |> Map.to_alist
  |> List.concat_map ~f:(fun (group, keys) -> -(group + 1) :: List.rev keys)
;;

let create ~scope ~clock =
  let fail_next = ref true in
  let%map.Or_error search =
    P.create ~scope ~clock ~max_loaded:2048 (initial ()) ~search:(fun request ->
      let query = String.strip (P.Request.query request) |> String.lowercase in
      Eio.Time.Mono.sleep clock (if String.equal query "slow" then 0.8 else 0.18);
      if String.equal query "offline" && !fail_next
      then (
        fail_next := false;
        Or_error.error_string "The sample catalog is temporarily unavailable. Try again.")
      else if String.equal query "remote"
      then
        Ok
          { P.Page.upsert =
              [ ( -5
                , { Entry.title = "From your workspace"
                  ; detail = "Fetched asynchronously"
                  } )
              ; ( 10_001
                , { Entry.title = "Remote field notes"
                  ; detail = "Fetched without replacing the collection."
                  } )
              ]
          ; visible = [ -5; 10_001 ]
          }
      else (
        let query =
          if String.equal query "offline" || String.equal query "slow" then "" else query
        in
        Ok { P.Page.upsert = []; visible = matches (P.Request.items request) query }))
  in
  { search; fail_next }
;;

let query_key = Key.of_string_exn "selectable-catalog-query"

let component t window palette scrollbar graph =
  let open B.Let_syntax in
  let snapshot = P.value t.search in
  let horizontal, set_horizontal = B.state false graph in
  let multiple, set_multiple = B.state true graph in
  let notice, set_notice =
    B.state "Select a few entries, then search. Hidden selections stay with you." graph
  in
  let editor =
    Editor.create
      window
      ~config:
        (B.return
           (Text_input.Config.create
              ~mode:Single_line
              ~label:"Search catalog"
              ~placeholder:"Search the collection…"
              ~clear_on_escape:false
              ()
            |> ok))
      graph
  in
  let source = B.map snapshot ~f:P.Snapshot.items in
  let identity = B.map source ~f:C.identity |> B.cutoff ~equal:phys_equal in
  let visible = B.map snapshot ~f:P.Snapshot.visible |> B.cutoff ~equal:phys_equal in
  let layout =
    let%arr identity = identity
    and visible = visible in
    let keys = C.Identity.keys identity in
    List_selection.Catalog.create
      identity
      ~visible
      ~disabled:(List.filter keys ~f:disabled)
      ()
    |> ok
    |> fun catalog ->
    List_rows.Layout.create
      catalog
      ~decorations:(List.filter keys ~f:(fun key -> key < 0))
      ()
    |> ok
  in
  let interaction =
    let%arr snapshot = snapshot
    and multiple = multiple in
    L.Interaction.create
      ~epoch:(P.Snapshot.epoch snapshot)
      ~mode:(if multiple then Multiple else Single)
      ~boundary:Wrap
      ~busy:(P.Snapshot.is_busy snapshot)
      ~disabled:(P.Snapshot.is_stale snapshot)
      ()
  in
  let query =
    let%arr editor = editor
    and snapshot = snapshot
    and p = palette
    and set_notice = set_notice in
    let on_event = function
      | Text_input.Event.Changed value
        when Option.is_none (Text_input.Snapshot.composition value) ->
        E.bind
          (E.of_thunk (fun () ->
             P.set_query
               t.search
               ~source:(P.Snapshot.source_id snapshot)
               (Text_input.Snapshot.text value)))
          ~f:(function
            | Ok () -> E.Ignore
            | Error error -> set_notice (Error.to_string_hum error))
      | Changed _ | Submitted _ | Search_changed _ -> E.Ignore
    in
    Editor.view
      editor
      ~initial_text:(P.Snapshot.query snapshot)
      ~on_event
      ~style:
        (style
           [ Height (px 42.)
           ; Shrink 0.
           ; Background (Background.solid (Palette.background p))
           ; Border_width 1.
           ; Border_color (Palette.border p)
           ; Radius 8.
           ; Padding (px 10.)
           ])
    |> fun view -> V.with_key view query_key
  in
  let before = B.map query ~f:(fun view -> [ view ]) in
  let after =
    let%arr snapshot = snapshot
    and p = palette in
    let text =
      match P.Snapshot.status snapshot with
      | Ready ->
        if List.is_empty (P.Snapshot.visible snapshot)
        then "No entries match. Try another search."
        else "Ready to explore"
      | Debouncing -> "Waiting for your next thought…"
      | Loading -> "Searching the catalog…"
      | Failed error -> Error.to_string_hum error
      | Cancelled -> "Search cancelled. Your previous results are still here."
      | Closed -> "Catalog closed"
    in
    [ V.column
        ~style:(style [ Padding_top (px 8.); Shrink 0. ])
        [ Palette.text p ~muted:true text ]
    ]
  in
  let config =
    B.map horizontal ~f:(fun horizontal ->
      if horizontal
      then Virtual_list.Config.horizontal ~max_active:16 ~width:(Fixed 240.) () |> ok
      else Virtual_list.Config.create ~max_active:16 ~height:(Fixed 64.) () |> ok)
  in
  let on_action =
    let%arr set_notice = set_notice in
    fun action ->
      let label target =
        Option.value_map
          (C.find (P.Snapshot.items (P.snapshot t.search)) (C.Item_ref.key target))
          ~default:"Retired entry"
          ~f:(fun entry -> entry.Entry.title)
      in
      match action with
      | L.Action.Confirm (target, Primary) -> set_notice ("Opened · " ^ label target)
      | Confirm (target, Secondary) -> set_notice ("Preview · " ^ label target)
      | Context target ->
        set_notice ("Actions for · " ^ label target ^ " — Copy title, pin, or open")
      | Cancel -> set_notice "Cursor cleared. Selection and search text are unchanged."
  in
  let output =
    L.component
      source
      ~layout
      ~config
      ~interaction
      ~label:"Searchable catalog"
      ~item_label:(fun ~key:_ entry -> entry.Entry.title)
      ~query:(B.return query_key)
      ~before
      ~after
      ~on_action
      ~style:(B.return (style [ Height (px 360.); Min_width (px 0.); Gap (px 10.) ]))
      ~render_row:(fun ~row ~controller ~lifetime graph ->
        let local, set_local = B.state 0 graph in
        let%arr row = row
        and p = palette
        and controller = controller
        and lifetime = lifetime
        and local = local
        and set_local = set_local in
        let item = L.Row.item row in
        let entry = List_rows.Item.data item in
        match List_rows.Item.kind item with
        | Decoration ->
          V.column
            ~style:(style [ Justify_content Center; Height (Length.percent_exn 100.) ])
            [ Palette.text p ~size:12. ~muted:true (String.uppercase entry.title) ]
          |> fun view ->
          V.with_accessibility
            view
            (Accessibility.create ~role:(Heading 3) ~label:entry.title () |> ok)
          |> ok
        | Option _ ->
          V.row
            ~style:
              (style
                 [ Align_items Center
                 ; Gap (px 8.)
                 ; Min_width (px 0.)
                 ; Height (Length.percent_exn 100.)
                 ])
            [ V.column
                ~style:(style [ Grow 1.; Min_width (px 0.); Gap (px 3.) ])
                [ Palette.text p ~size:14. entry.title
                ; Palette.text
                    p
                    ~size:11.
                    ~muted:true
                    (if List_rows.Item.is_disabled item
                     then "Unavailable in this collection"
                     else entry.detail)
                ]
            ; Palette.button
                p
                ~disabled:(List_rows.Item.is_disabled item)
                (sprintf "Inspect · %d" local)
                (Gpuio_bonsai.Managed_rows.Lifetime.guard
                   lifetime
                   (E.Many
                      [ set_local (local + 1)
                      ; L.Controller.confirm
                          controller
                          ~target:(List_rows.Item.target item)
                          Secondary
                      ]))
            ])
      graph
  in
  let%arr output = output
  and snapshot = snapshot
  and editor = editor
  and p = palette
  and horizontal = horizontal
  and set_horizontal = set_horizontal
  and multiple = multiple
  and set_multiple = set_multiple
  and notice = notice
  and set_notice = set_notice
  and scrollbar = scrollbar in
  let output = ok output in
  let controller = L.Output.controller output in
  let report f =
    E.bind (E.of_thunk f) ~f:(function
      | Ok () -> E.Ignore
      | Error error -> set_notice (Error.to_string_hum error))
  in
  let replace_query text =
    let current_source () =
      C.Source_id.equal
        (P.Snapshot.source_id snapshot)
        (P.Snapshot.source_id (P.snapshot t.search))
    in
    E.bind (E.of_thunk current_source) ~f:(fun current ->
      if not current
      then E.Ignore
      else
        E.bind (Editor.replace editor ~selection:End ~undo:Record text) ~f:(function
          | Error _ -> set_notice "Search is unavailable while editing or unmounted."
          | Ok value ->
            report (fun () ->
              let current = P.snapshot t.search in
              let text = Text_input.Snapshot.text value in
              if not (current_source ())
              then Ok ()
              else if String.equal text (P.Snapshot.query current)
              then P.refresh t.search
              else P.set_query t.search ~source:(P.Snapshot.source_id snapshot) text)))
  in
  let selected = List_selection.selected (L.Output.state output) in
  let selected_labels =
    List.take selected 3
    |> List.filter_map ~f:(fun target ->
      Option.map
        (C.find (P.Snapshot.items snapshot) (C.Item_ref.key target))
        ~f:(fun entry -> entry.Entry.title))
  in
  let cursor = List_selection.cursor (L.Output.state output) in
  let stream =
    report (fun () ->
      let current = P.Snapshot.items (P.snapshot t.search) in
      match
        Option.bind cursor ~f:(fun target ->
          if C.contains_ref current target
          then
            C.find current (C.Item_ref.key target)
            |> Option.map ~f:(fun data -> C.Item_ref.key target, data)
          else None)
      with
      | None -> Ok ()
      | Some (key, entry) ->
        let%bind.Or_error items =
          C.set
            current
            ~key
            ~data:{ entry with detail = "A live update arrived while you were reading." }
        in
        P.update_source t.search ~refresh:false items)
  in
  Palette.card
    p
    ~title:"Find something worth keeping"
    [ Palette.text
        p
        ~muted:true
        "A persistent collection with search, sections and independent selection. Try \
         remote for fetched entries, or recovery for a retryable failure."
    ; V.row
        ~style:(style [ Gap (px 8.); Wrap Wrap ])
        [ Palette.button p "All entries" (replace_query "")
        ; Palette.button p "Fetch remote" (replace_query "remote")
        ; Palette.button p "No matches" (replace_query "no-such-entry")
        ; Palette.button
            p
            "Try recovery"
            (E.Many
               [ E.of_thunk (fun () -> t.fail_next := true); replace_query "offline" ])
        ; Palette.button
            p
            ~disabled:
              (not
                 (match P.Snapshot.status snapshot with
                  | Failed _ | Cancelled -> true
                  | _ -> false))
            "Retry search"
            (report (fun () -> P.retry t.search ~epoch:(P.Snapshot.epoch snapshot)))
        ; Palette.button
            p
            ~disabled:(not (P.Snapshot.is_busy snapshot))
            "Cancel search"
            (E.of_thunk (fun () -> P.cancel t.search ~epoch:(P.Snapshot.epoch snapshot)))
        ]
    ; V.with_scrollbar (L.Output.view output) scrollbar |> ok
    ; V.row
        ~style:(style [ Gap (px 8.); Wrap Wrap ])
        [ Palette.button
            p
            (if multiple then "Use single selection" else "Use multiple selection")
            (set_multiple (not multiple))
        ; Palette.button
            p
            (if horizontal then "Use vertical list" else "Use horizontal list")
            (set_horizontal (not horizontal))
        ; Palette.button p "Clear selection" (L.Controller.clear_selection controller)
        ; Palette.button
            p
            ~disabled:(Option.is_none cursor)
            "Update current detail"
            stream
        ; Palette.button
            p
            "New collection"
            (report (fun () -> P.update_source t.search (initial ())))
        ]
    ; Palette.text
        p
        (sprintf
           "%d selected · %d loaded · %d / 16 mounted"
           (List.length selected)
           (C.length (P.Snapshot.items snapshot))
           (L.Output.active_rows output))
    ; Palette.text
        p
        ~muted:true
        (if List.is_empty selected_labels
         then "Nothing selected yet."
         else String.concat ~sep:"  ·  " selected_labels)
    ; Palette.text p notice
    ; Palette.text
        p
        ~size:11.
        ~muted:true
        "Arrows move the cursor. Space toggles selection. Shift extends a range. Enter \
         opens; ⌘Enter previews. Right-click or Shift-F10 requests actions. Escape \
         clears only the cursor."
    ]
;;
