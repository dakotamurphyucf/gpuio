open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View
module L = Gpuio_bonsai.Virtual_list
module T = Gpuio_bonsai.Table
module Forest = Gpuio_bonsai.Tree

let ok = Or_error.ok_exn
let px = Length.px_exn
let style = Style.create_exn
let count = 1000
let row_id n = Table_data.Id.of_string (Int.to_string n) |> ok
let column_id name = Table_column.Id.of_string name |> ok
let tree_id name = Tree.Id.of_string name |> ok

module Messages = Gpuio_gallery_model.Message_stream
module Follow = Gpuio_gallery_model.Message_follow

module Grid = struct
  type t =
    { data : int Table_data.t
    ; config : Table.Config.t
    ; notice : string
    ; striped : bool
    ; custom_colors : bool
    ; scoped_presentation : bool
    ; rich_headers : bool
    ; compact_padding : bool
    }

  let initial =
    let columns =
      Table_column.Collection.create
        [ Table_column.create
            ~id:(column_id "entry")
            ~label:"ENTRY"
            ~width:130.
            ~pin:Left
            ~sortable:true
            ()
          |> ok
        ; Table_column.create ~id:(column_id "category") ~label:"CATEGORY" ~width:150. ()
          |> ok
        ; Table_column.create ~id:(column_id "detail") ~label:"DETAIL" ~width:380. ()
          |> ok
        ]
      |> ok
    in
    { data = Table_data.create (List.init count ~f:(fun n -> row_id n, n)) |> ok
    ; config =
        Table.Config.create
          ~label:"Preview results"
          ~column_selection:true
          ~columns
          ~row_height:36.
          ~max_active_rows:24
          ~max_active_cells:72
          ()
        |> ok
    ; striped = false
    ; custom_colors = false
    ; scoped_presentation = false
    ; rich_headers = false
    ; compact_padding = false
    ; notice = "Choose a cell or sort the entry column."
    }
  ;;

  let describe = function
    | Table.Selection.Empty -> "Nothing"
    | Row row -> "Row " ^ Table_data.Id.to_string (Table_data.Row_ref.id row)
    | Column column -> "Column " ^ Table_column.Id.to_string column
    | Cell (row, column) ->
      "Cell "
      ^ Table_data.Id.to_string (Table_data.Row_ref.id row)
      ^ " / "
      ^ Table_column.Id.to_string column
  ;;

  type action =
    | Request of Table_data.Row_ref.t Table.Request.t
    | Toggle_row_header
    | Toggle_boundary
    | Toggle_headers
    | Toggle_stripes
    | Toggle_colors
    | Toggle_scoped_presentation
    | Toggle_rich_headers
    | Header_action
    | Toggle_padding

  let request t request =
    let open Or_error.Let_syntax in
    let result =
      match request with
      | Table.Request.Resize sizes ->
        let%bind columns =
          List.fold_result
            sizes
            ~init:(Table.Config.columns t.config)
            ~f:(fun columns (column, width) ->
              Table_column.Collection.resize columns ~column ~width)
        in
        let%map config = Table.Config.with_columns t.config columns in
        { t with config; notice = "Column size accepted" }
      | Move (column, before) ->
        let%bind columns =
          Table_column.Collection.move (Table.Config.columns t.config) ~column ~before
        in
        let%map config = Table.Config.with_columns t.config columns in
        { t with config; notice = "Column order accepted" }
      | Sort (column, direction) ->
        let%bind config =
          Table.Config.with_sort
            t.config
            (Option.map direction ~f:(fun direction -> { Table.Sort.column; direction }))
        in
        let keys = List.init count ~f:row_id in
        let keys =
          match direction with
          | Some Descending -> List.rev keys
          | Some Ascending | None -> keys
        in
        let%map data = Table_data.reorder t.data keys in
        { t with data; config; notice = "Entry order updated" }
      | Select selection -> Ok { t with notice = describe selection ^ " selected" }
      | Context selection ->
        Ok { t with notice = describe selection ^ " context requested" }
      | Copy selection -> Ok { t with notice = describe selection ^ " copy requested" }
      | Activate (row, _) ->
        Ok
          { t with
            notice = "Opened entry " ^ Table_data.Id.to_string (Table_data.Row_ref.id row)
          }
    in
    match result with
    | Ok t -> t
    | Error error -> { t with notice = Error.to_string_hum error }
  ;;

  let presentation t p =
    let module A = Table.Appearance in
    let colors =
      if t.custom_colors
      then
        [ A.Part.Header_background, Palette.surface p
        ; Header_foreground, Palette.accent p
        ; Stripe_background, Color.with_opacity (Palette.accent p) 0.08 |> ok
        ; Hover_background, Color.with_opacity (Palette.accent p) 0.14 |> ok
        ; Selected_background, Color.with_opacity (Palette.accent p) 0.24 |> ok
        ; Selected_border, Palette.accent p
        ; Row_border, Palette.border p
        ; Column_border, Palette.border p
        ]
      else []
    in
    let padding = if t.compact_padding then Some (A.Padding.all 2. |> ok) else None in
    let column_padding =
      if t.compact_padding then [ column_id "entry", A.Padding.zero ] else []
    in
    let appearance =
      A.create ~striped:t.striped ~colors ?padding ~column_padding () |> ok
    in
    let columns = Table.Config.columns t.config |> Table_column.Collection.to_list in
    let group label names =
      Table_column.Group.create ~label ~columns:(List.map names ~f:column_id) |> ok
    in
    let unpinned =
      List.filter columns ~f:(fun column ->
        Table_column.Pin.equal (Table_column.pin column) Unpinned)
      |> List.map ~f:(fun column -> Table_column.Id.to_string (Table_column.id column))
    in
    let header_groups =
      if t.rich_headers
      then [ [ group "PINNED" [ "entry" ]; group "RESULT DETAILS" unpinned ] ]
      else []
    in
    let columns = Table_column.Collection.create ~header_groups columns |> ok in
    let config = Table.Config.with_columns t.config columns |> ok in
    Table.Config.with_appearance config appearance |> ok
  ;;

  let apply t = function
    | Toggle_scoped_presentation ->
      { t with scoped_presentation = not t.scoped_presentation }
    | Toggle_rich_headers -> { t with rich_headers = not t.rich_headers }
    | Header_action ->
      { t with
        notice = "Header action handled independently of table sorting and selection"
      }
    | Toggle_stripes -> { t with striped = not t.striped }
    | Toggle_colors -> { t with custom_colors = not t.custom_colors }
    | Toggle_padding -> { t with compact_padding = not t.compact_padding }
    | Request input -> request t input
    | Toggle_row_header ->
      { t with
        config =
          Table.Config.with_row_header t.config (not (Table.Config.row_header t.config))
      }
    | Toggle_boundary ->
      let boundary =
        match Table.Config.boundary t.config with
        | Stop -> Table.Boundary.Wrap
        | Wrap -> Stop
      in
      { t with config = Table.Config.with_boundary t.config boundary }
    | Toggle_headers ->
      let headers =
        match Table.Config.selectable_headers t.config with
        | None -> Some [ column_id "entry" ]
        | Some _ -> None
      in
      { t with config = Table.Config.with_selectable_headers t.config headers |> ok }
  ;;
end

let outline =
  let leaf label = Tree.Node.create ~label ~children:Leaf () |> ok in
  let branch label children =
    Tree.Node.create
      ~label
      ~children:(Branch { ids = List.map children ~f:tree_id; next = End })
      ()
    |> ok
  in
  Tree.create
    ~roots:[ tree_id "research"; tree_id "archive" ]
    [ tree_id "research", branch "Research" [ "notes"; "sketches" ]
    ; tree_id "notes", leaf "Field notes"
    ; tree_id "sketches", branch "Sketches" [ "orchard"; "observatory" ]
    ; tree_id "orchard", leaf "Orchard study"
    ; tree_id "observatory", leaf "Observatory study"
    ; tree_id "archive", branch "Archive" [ "release" ]
    ; tree_id "release", leaf "Release checklist"
    ]
  |> ok
  |> Tree_loading.create
  |> Tree_loading.snapshot
;;

module Mode = struct
  type t =
    | Messages
    | Outline
    | Results
    | Scrollbars
    | Cards
    | Selectable
    | Structural
  [@@deriving equal]

  let label = function
    | Messages -> "Message list"
    | Outline -> "Outline tree"
    | Results -> "Result table"
    | Scrollbars -> "Scrollbars"
    | Cards -> "Horizontal cards"
    | Selectable -> "Searchable list"
    | Structural -> "Structural table"
  ;;

  let all = [ Messages; Selectable; Cards; Outline; Results; Structural; Scrollbars ]
end

(* The follow recipe clips its supplied action to a 48px slot. Keep this
   overlay action within that slot even when preview typography grows. *)
let follow_button p on_click =
  V.button
    "Follow latest"
    ~on_click
    ~style:
      (style
         [ Height (px 44.)
         ; Padding_left (px 12.)
         ; Padding_right (px 12.)
         ; Font_size (Palette.size p 13.)
         ; Line_height (px 20.)
         ; Background (Background.solid (Palette.surface p))
         ; Foreground (Palette.foreground p)
         ; Border_color (Palette.border p)
         ; Border_width 1.
         ; Radius 8.
         ]
       |> fun s ->
       Style.with_state_exn s Hovered [ Background (Background.solid (Palette.border p)) ]
      )
;;

let component app searchable window palette graph =
  let structural = Structural_table_preview.component palette graph in
  let scrollbars = Scrollbar_preview.component app window palette graph in
  let scrollbar = B.map scrollbars ~f:Scrollbar_preview.description in
  let selectable =
    Selectable_preview.component searchable window palette scrollbar graph
  in
  let cards = Horizontal_list_preview.component palette scrollbar graph in
  let mode, set_mode = B.state Mode.Messages graph in
  let jump_enabled, toggle_jump = B.toggle ~default_model:true graph in
  let fade_enabled, toggle_fade = B.toggle ~default_model:true graph in
  let follow_motion, toggle_follow_motion = B.toggle ~default_model:true graph in
  let messages, change_messages =
    B.state_machine0
      ~default_model:Messages.initial
      ~apply_action:(fun _ state action -> Messages.apply state action)
      graph
  in
  let grid, grid_request =
    B.state_machine0
      ~default_model:Grid.initial
      ~apply_action:(fun _ state action -> Grid.apply state action)
      graph
  in
  let open B.Let_syntax in
  let viewport_style height =
    let%arr p = palette in
    style
      [ Height (px height)
      ; Shrink 0.
      ; Foreground (Palette.foreground p)
      ; Background (Background.solid (Palette.background p))
      ; Border_color (Palette.border p)
      ; Radius 10.
      ]
  in
  let list =
    L.component
      (module Int)
      (B.map messages ~f:Messages.rows)
      ~row_key:Key.of_int
      ~accessibility:
        (B.return
           (Accessibility.create ~role:Log ~label:"Preview conversation" ~live:Off ()
            |> ok))
      ~style:(viewport_style 235.)
      ~config:
        (Virtual_list.Config.create
           ~height:(Estimated 85.)
           ~scroll:Follow_tail_when_at_end
           ~max_active:24
           ~overscan:100.
           ()
         |> ok)
      ~render_row:(fun ~key:_ ~data ~lifetime:_ _graph ->
        let%arr data = data
        and p = palette in
        V.column
          ~style:
            (style
               [ Padding (px 12.)
               ; Border_bottom_width 1.
               ; Border_color (Palette.border p)
               ; Shrink 0.
               ])
          [ Palette.text p data ])
      ~scrollbar
      graph
  in
  let tree =
    Forest.component
      (B.return outline)
      ~label:"Preview outline"
      ~config:(Virtual_list.Config.create ~height:(Fixed 36.) ~max_active:16 () |> ok)
      ~style:(viewport_style 235.)
      ~initial_expanded:(B.return [ tree_id "research" ])
      ~scrollbar
      graph
  in
  let headers =
    let%arr grid = grid
    and p = palette
    and inject = grid_request in
    if not grid.Grid.rich_headers
    then []
    else
      [ Table_header.create
          ~key:(Key.of_string_exn "entry-header-action")
          ~target:(Table_header.Target.column (column_id "entry"))
          (V.button
             ~style:
               (style
                  [ Height (px 28.)
                  ; Padding (px 2.)
                  ; Padding_left (px 8.)
                  ; Padding_right (px 8.)
                  ; Font_size (Palette.size p 12.)
                  ; Line_height (px 18.)
                  ; Background (Background.solid (Palette.surface p))
                  ; Foreground (Palette.foreground p)
                  ; Radius 4.
                  ; Shrink 0.
                  ])
             ~on_click:(inject Grid.Header_action)
             "Inspect")
      ; Table_header.create
          ~key:(Key.of_string_exn "details-group")
          ~target:
            (Table_header.Target.group
               ~level:0
               ~columns:[ column_id "category"; column_id "detail" ]
             |> ok)
          (Palette.text p "RESULT DETAILS · retained Views")
      ]
  in
  let table =
    T.component
      (B.map grid ~f:(fun t -> t.Grid.data))
      ~config:(B.map2 grid palette ~f:Grid.presentation)
      ~headers
      ~header_presentation:
        (B.map2 grid palette ~f:(fun grid p ->
           if not grid.Grid.scoped_presentation
           then Table_presentation.Header.empty
           else
             style [ Background (Background.solid (Palette.surface p)); Font_weight 700 ]
             |> fun style ->
             Style.with_state_exn style Hovered [ Foreground (Palette.accent p) ]
             |> Table_presentation.Header.create
             |> ok))
      ~render_row_presentation:(fun ~row:_ ~data ~lifetime:_ _graph ->
        let%arr n = data
        and grid = grid
        and p = palette in
        if not grid.Grid.scoped_presentation
        then Ok Table_presentation.Row.empty
        else (
          let tint opacity =
            Style.Property.Background
              (Background.solid (Color.with_opacity (Palette.accent p) opacity |> ok))
          in
          style (if n % 5 = 0 then [ tint 0.08; Font_weight 600 ] else [])
          |> fun style ->
          Style.with_state_exn style Selected [ tint 0.24 ]
          |> fun style ->
          Style.with_state_exn style Focused [ Border_color (Palette.accent p) ]
          |> fun style ->
          Style.with_state_exn style Hovered [ tint 0.16 ]
          |> fun style ->
          Style.with_state_exn style Pressed [ tint 0.3 ] |> Table_presentation.Row.create))
      ~style:(viewport_style 255.)
      ~on_request:
        (B.map grid_request ~f:(fun inject request -> inject (Grid.Request request)))
      ~render_cell:(fun ~row:_ ~data ~column ~lifetime:_ _graph ->
        let%arr n = data
        and column = column
        and p = palette in
        let name = Table_column.Id.to_string (Table_column.id column) in
        let text =
          if String.equal name "entry"
          then sprintf "%04d" n
          else if String.equal name "category"
          then if n % 2 = 0 then "Research" else "Design"
          else sprintf "Finding %04d · 日本語 · 👨‍👩‍👧‍👦" n
        in
        T.Cell.create
          ~column:(Table_column.id column)
          ~copy_text:text
          (Palette.text p text))
      ~scrollbar
      graph
  in
  let%arr p = palette
  and scrollbars = scrollbars
  and cards = cards
  and selectable = selectable
  and structural = structural
  and mode = mode
  and set_mode = set_mode
  and messages = messages
  and list = list
  and tree = tree
  and table = table
  and grid = grid
  and grid_request = grid_request
  and jump_enabled = jump_enabled
  and toggle_jump = toggle_jump
  and fade_enabled = fade_enabled
  and toggle_fade = toggle_fade
  and follow_motion = follow_motion
  and toggle_follow_motion = toggle_follow_motion
  and change_messages = change_messages in
  let list = ok list
  and tree = ok tree
  and table = ok table in
  let target = T.Output.target table (row_id (count - 1)) |> ok in
  let selected =
    Forest.Output.state tree
    |> Tree_state.selected
    |> List.map ~f:Tree.Id.to_string
    |> String.concat ~sep:", "
  in
  let previews =
    [ ( Mode.Messages
      , Palette.card
          p
          ~title:"A thousand entries, a small viewport"
          [ V.row
              ~style:(style [ Gap (px 10.); Wrap Wrap ])
              [ Palette.button
                  p
                  "First entry"
                  (L.Controller.scroll_to
                     (L.Output.controller list)
                     (Messages.first messages)
                   |> ok)
              ; Palette.button
                  p
                  "Jump to latest"
                  (L.Controller.jump_to_latest (L.Output.controller list))
              ; Palette.button
                  p
                  ~disabled:(not (Messages.can_prepend messages))
                  "Earlier history"
                  (change_messages Prepend)
              ; Palette.button
                  p
                  ~disabled:(not (Messages.can_append messages))
                  "New message"
                  (change_messages Append)
              ; Palette.button
                  p
                  ~disabled:(Messages.streamed_lines messages >= 8)
                  "Grow latest response"
                  (change_messages Stream_latest)
              ; Palette.button p "Reset latest response" (change_messages Reset_latest)
              ]
          ; V.row
              ~style:(style [ Gap (px 10.); Wrap Wrap ])
              [ Palette.button p ~selected:jump_enabled "Show follow button" toggle_jump
              ; Palette.button p ~selected:fade_enabled "Fade transcript edge" toggle_fade
              ; Palette.button
                  p
                  ~selected:follow_motion
                  "Animate follow controls"
                  toggle_follow_motion
              ]
          ; Follow.view
              ~viewport:(L.Output.viewport list)
              ~jump_enabled
              ~motion:follow_motion
              ~fade:(Option.some_if fade_enabled (Palette.background p))
              ~jump:
                (follow_button p (L.Controller.jump_to_latest (L.Output.controller list)))
              (L.Output.view list)
          ; Palette.text
              p
              (sprintf
                 "%d messages · Latest response: %d / 8 extra lines"
                 (List_collection.length (Messages.rows messages))
                 (Messages.streamed_lines messages))
          ; Palette.text
              p
              ~muted:true
              (match L.Output.viewport list with
               | None -> "Measuring the conversation…"
               | Some viewport ->
                 if viewport.following_tail
                 then "Following new messages. Scroll up to read earlier history."
                 else "Reading history. Jump to latest to follow new messages again.")
          ; Palette.text
              p
              ~muted:true
              (sprintf "Mounted list rows: %d / 24" (L.Output.active_rows list))
          ] )
    ; ( Mode.Outline
      , Palette.card
          p
          ~title:"An outline with structure"
          [ Forest.Output.view tree
          ; Palette.text p ("Selected outline: " ^ selected)
          ; Palette.button
              p
              "Reveal observatory"
              (Forest.Controller.reveal
                 (Forest.Output.controller tree)
                 ~focus:true
                 (Forest.Output.target tree (tree_id "observatory") |> ok))
          ] )
    ; ( Mode.Results
      , Palette.card
          p
          ~title:"Details worth exploring"
          [ V.row
              ~style:(style [ Gap (px 10.) ])
              [ Palette.button
                  p
                  "Select last result"
                  (T.Controller.batch
                     (T.Output.controller table)
                     [ Set_selection (Cell (target, column_id "detail"))
                     ; Reveal (target, Some (column_id "detail"))
                     ]
                   |> ok)
              ; Palette.button
                  p
                  "First result"
                  (T.Controller.scroll_to
                     (T.Output.controller table)
                     (T.Output.target table (row_id 0) |> ok)
                   |> ok)
              ]
          ; V.row
              ~style:(style [ Gap (px 10.); Wrap Wrap ])
              [ Palette.button
                  p
                  ~selected:(Table.Config.row_header grid.config)
                  "Row headers"
                  (grid_request Toggle_row_header)
              ; Palette.button
                  p
                  ~selected:
                    (Table.Boundary.equal (Table.Config.boundary grid.config) Wrap)
                  "Wrap table navigation"
                  (grid_request Toggle_boundary)
              ; Palette.button
                  p
                  ~selected:(Option.is_some (Table.Config.selectable_headers grid.config))
                  "Select only entry header"
                  (grid_request Toggle_headers)
              ]
          ; V.row
              ~style:(style [ Gap (px 10.); Wrap Wrap ])
              [ Palette.button
                  p
                  ~selected:grid.striped
                  "Striped rows"
                  (grid_request Toggle_stripes)
              ; Palette.button
                  p
                  ~selected:grid.custom_colors
                  "Table part colors"
                  (grid_request Toggle_colors)
              ; Palette.button
                  p
                  ~selected:grid.compact_padding
                  "Compact cell padding"
                  (grid_request Toggle_padding)
              ; Palette.button
                  p
                  ~selected:grid.rich_headers
                  "Rich table headers"
                  (grid_request Toggle_rich_headers)
              ; Palette.button
                  p
                  ~selected:grid.scoped_presentation
                  "Header and row styling"
                  (grid_request Toggle_scoped_presentation)
              ]
          ; T.Output.view table
          ; Palette.text p ("Table selection: " ^ Grid.describe (T.Output.selection table))
          ; Palette.text p ~muted:true grid.notice
          ; Palette.text
              p
              ~muted:true
              (sprintf "Mounted cells: %d / 72" (T.Output.active_cells table))
          ; Palette.text
              p
              ~muted:true
              (match T.Output.column_viewport table with
               | None -> "Visible columns: measuring…"
               | Some viewport ->
                 "Visible columns: "
                 ^ String.concat
                     ~sep:", "
                     (List.map (Table.Column_viewport.columns viewport) ~f:(fun column ->
                        Table_column.Id.to_string (Table.Column_viewport.Column.id column)
                        ^ (match Table.Column_viewport.Column.pin column with
                           | Left -> " (pinned)"
                           | Unpinned -> "")
                        ^
                        if Table.Column_viewport.Column.fully_visible column
                        then ""
                        else " (partial)")))
          ] )
    ; Mode.Cards, cards
    ; Mode.Selectable, selectable
    ; Mode.Structural, structural
    ; Mode.Scrollbars, Scrollbar_preview.viewport scrollbars
    ]
  in
  V.column
    ~style:(style [ Gap (px 20.) ])
    [ V.row
        ~style:(style [ Gap (px 10.); Wrap Wrap ])
        (List.map Mode.all ~f:(fun candidate ->
           Palette.button
             p
             ~selected:(Mode.equal mode candidate)
             (Mode.label candidate)
             (set_mode candidate)))
    ; Scrollbar_preview.controls scrollbars
    ; V.column
        (List.map previews ~f:(fun (candidate, content) ->
           V.tab_panel
             ~key:(Key.of_string_exn (Mode.label candidate))
             ~label:(Mode.label candidate ^ " preview")
             ~active:(Mode.equal mode candidate)
             [ content ]))
    ]
;;
