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

module Messages = struct
  type t =
    { rows : (int, string, Int.comparator_witness) List_collection.t
    ; lines : int
    }

  type action =
    | Grow
    | Reset

  let text n =
    sprintf
      "Entry %04d\n%s"
      n
      (String.concat
         ~sep:"\n"
         (List.init (1 + (n % 3)) ~f:(fun _ -> "An idea with room to develop.")))
  ;;

  let initial =
    { rows =
        List_collection.of_alist (module Int) (List.init count ~f:(fun n -> n, text n))
        |> ok
    ; lines = 0
    }
  ;;

  let apply t action =
    let lines =
      match action with
      | Grow -> Int.min 8 (t.lines + 1)
      | Reset -> 0
    in
    let text =
      text 0 ^ String.concat (List.init lines ~f:(fun _ -> "\nAnother useful detail."))
    in
    { lines; rows = List_collection.set t.rows ~key:0 ~data:text |> ok }
  ;;
end

module Grid = struct
  type t =
    { data : int Table_data.t
    ; config : Table.Config.t
    ; notice : string
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
          ~columns
          ~row_height:36.
          ~max_active_rows:24
          ~max_active_cells:72
          ()
        |> ok
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

  let apply t request =
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
        { data; config; notice = "Entry order updated" }
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
  [@@deriving equal]

  let label = function
    | Messages -> "Message list"
    | Outline -> "Outline tree"
    | Results -> "Result table"
  ;;

  let all = [ Messages; Outline; Results ]
end

let component palette graph =
  let mode, set_mode = B.state Mode.Messages graph in
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
      (B.map messages ~f:(fun t -> t.Messages.rows))
      ~row_key:Key.of_int
      ~style:(viewport_style 235.)
      ~config:
        (Virtual_list.Config.create
           ~height:(Estimated 85.)
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
      graph
  in
  let tree =
    Forest.component
      (B.return outline)
      ~label:"Preview outline"
      ~config:(Virtual_list.Config.create ~height:(Fixed 36.) ~max_active:16 () |> ok)
      ~style:(viewport_style 235.)
      ~initial_expanded:(B.return [ tree_id "research" ])
      graph
  in
  let table =
    T.component
      (B.map grid ~f:(fun t -> t.Grid.data))
      ~config:(B.map grid ~f:(fun t -> t.Grid.config))
      ~style:(viewport_style 255.)
      ~on_request:(B.map grid_request ~f:(fun inject request -> inject request))
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
      graph
  in
  let%arr p = palette
  and mode = mode
  and set_mode = set_mode
  and messages = messages
  and list = list
  and tree = tree
  and table = table
  and grid = grid
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
                  (L.Controller.scroll_to (L.Output.controller list) 0 |> ok)
              ; Palette.button
                  p
                  "Last entry"
                  (L.Controller.scroll_to (L.Output.controller list) (count - 1) |> ok)
              ; Palette.button p "Grow first entry" (change_messages Grow)
              ; Palette.button p "Reset first entry" (change_messages Reset)
              ]
          ; L.Output.view list
          ; Palette.text p (sprintf "Extra lines: %d / 8" messages.lines)
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
          ; T.Output.view table
          ; Palette.text p ("Table selection: " ^ Grid.describe (T.Output.selection table))
          ; Palette.text p ~muted:true grid.notice
          ; Palette.text
              p
              ~muted:true
              (sprintf "Mounted cells: %d / 72" (T.Output.active_cells table))
          ] )
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
    ; V.column
        (List.map previews ~f:(fun (candidate, content) ->
           V.tab_panel
             ~key:(Key.of_string_exn (Mode.label candidate))
             ~label:(Mode.label candidate ^ " preview")
             ~active:(Mode.equal mode candidate)
             [ content ]))
    ]
;;
