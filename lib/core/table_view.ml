open Core

let unique_keys kind keys =
  match List.find_a_dup keys ~compare:Key.compare with
  | None -> Ok ()
  | Some key -> Or_error.errorf "duplicate table %s key: %s" kind (Key.to_string key)
;;

module Cell = struct
  module Kind = struct
    type t =
      | Data
      | Column_header
      | Row_header
    [@@deriving equal, sexp_of]
  end

  type 'action t =
    { key : Key.t
    ; kind : Kind.t
    ; span : int
    ; style : Style.t
    ; children : 'action View.t list
    }

  let create ~key ?(kind = Kind.Data) ?(span = 1) ?(style = Style.empty) children =
    if span < 1 || span > 64
    then Or_error.error_string "table column span must be in 1..64"
    else Ok { key; kind; span; style; children }
  ;;
end

module Row = struct
  type 'action t =
    { key : Key.t
    ; style : Style.t
    ; cells : 'action Cell.t list
    ; columns : int
    }

  let create ~key ?(style = Style.empty) cells =
    let open Or_error.Let_syntax in
    let%bind () =
      if List.is_empty cells || List.length cells > 64
      then Or_error.error_string "table row requires 1..64 cells"
      else Ok ()
    in
    let%bind () = unique_keys "cell" (List.map cells ~f:(fun c -> c.Cell.key)) in
    let columns = List.sum (module Int) cells ~f:(fun c -> c.Cell.span) in
    if columns > 64
    then Or_error.error_string "table row spans exceed 64 columns"
    else Ok { key; style; cells; columns }
  ;;
end

module Section = struct
  type 'action t =
    { key : Key.t
    ; style : Style.t
    ; rows : 'action Row.t list
    }

  let create ~key ?(style = Style.empty) rows =
    let open Or_error.Let_syntax in
    let%bind () =
      if List.length rows > 4096
      then Or_error.error_string "table section exceeds 4096 rows"
      else Ok ()
    in
    let%map () = unique_keys "row" (List.map rows ~f:(fun r -> r.Row.key)) in
    { key; style; rows }
  ;;
end

let key = Key.of_string_exn
let px = Length.px_exn
let full = Length.percent_exn 100.
let style = Style.create_exn

let semantic view role =
  Accessibility.create ~role () |> Or_error.bind ~f:(View.with_accessibility view)
;;

let render_cell ~row ~column ~cell_style (cell : _ Cell.t) =
  let open Or_error.Let_syntax in
  let%bind metadata =
    Accessibility.Table_cell.create ~row ~column ~column_span:cell.span ()
  in
  let role =
    match cell.kind with
    | Data -> Accessibility.Role.Table_cell metadata
    | Column_header -> Column_header metadata
    | Row_header -> Row_header metadata
  in
  let module G = Style.Grid_location in
  let location =
    G.create
      ~column:
        (G.Axis.create
           ~start:(Line (G.Line.of_int_exn (column + 1)))
           ~end_:(Span (G.Span.of_int_exn cell.span)))
      ()
  in
  let open Style.Property in
  View.row
    ~key:cell.key
    ~style:
      (Style.merge
         [ style
             [ Min_width (px 0.)
             ; Padding_left (px 8.)
             ; Padding_right (px 8.)
             ; Padding_top (px 4.)
             ; Padding_bottom (px 4.)
             ; Align_items Center
             ]
         ; cell_style
         ; cell.style
         ; style [ Grid_location location ]
         ])
    cell.children
  |> fun view -> semantic view role
;;

let render_section ~columns ~first_row ~cell_style (section : _ Section.t) =
  let open Or_error.Let_syntax in
  let%bind rows =
    List.mapi section.rows ~f:(fun index row ->
      let row_index = first_row + index in
      let%bind _, cells =
        List.fold_result row.Row.cells ~init:(0, []) ~f:(fun (column, result) cell ->
          let%map view = render_cell ~row:row_index ~column ~cell_style cell in
          column + cell.span, view :: result)
      in
      let open Style.Property in
      View.column
        ~key:row.key
        ~style:
          (Style.merge
             [ style [ Width full; Min_width (px 0.); Shrink 0. ]
             ; row.style
             ; style
                 [ Display Grid
                 ; Grid_columns columns
                 ; Grid_column_minimum Zero
                 ; Gap (px 0.)
                 ]
             ])
        (List.rev cells)
      |> fun view -> semantic view (Table_row row_index))
    |> Or_error.all
  in
  let open Style.Property in
  View.column
    ~key:section.key
    ~style:
      (Style.merge
         [ style [ Width full; Min_width (px 0.); Shrink 0. ]
         ; section.style
         ; style [ Display Flex; Direction Column; Gap (px 0.) ]
         ])
    rows
  |> fun view -> semantic view Row_group
;;

let create
      ~columns
      ~label
      ?key:root_key
      ?(style = Style.empty)
      ?(cell_style = Style.empty)
      ?header
      ?footer
      ?caption
      sections
  =
  let open Or_error.Let_syntax in
  let%bind () =
    if columns < 1 || columns > 64
    then Or_error.error_string "table columns must be in 1..64"
    else Ok ()
  in
  let%bind () =
    if List.length sections > 4096
    then Or_error.error_string "table exceeds 4096 body sections"
    else Ok ()
  in
  let%bind () = unique_keys "section" (List.map sections ~f:(fun s -> s.Section.key)) in
  let all_sections = Option.to_list header @ sections @ Option.to_list footer in
  let row_count =
    List.sum (module Int) all_sections ~f:(fun s -> List.length s.Section.rows)
  in
  let cell_count =
    List.sum
      (module Int)
      all_sections
      ~f:(fun s ->
        List.sum (module Int) s.Section.rows ~f:(fun r -> List.length r.Row.cells))
  in
  let%bind () =
    if row_count > 4096 || cell_count > 16384
    then Or_error.error_string "table exceeds 4096 rows or 16384 cells"
    else if
      List.exists all_sections ~f:(fun s ->
        List.exists s.Section.rows ~f:(fun r -> r.Row.columns <> columns))
    then Or_error.error_string "each table row must cover exactly the declared columns"
    else Ok ()
  in
  let%bind info = Accessibility.Table_info.create ~rows:row_count ~columns () in
  let%bind metadata = Accessibility.create ~role:(Table info) ~label () in
  let first_row = ref 0 in
  let section section =
    let result = render_section ~columns ~first_row:!first_row ~cell_style section in
    first_row := !first_row + List.length section.Section.rows;
    result
  in
  let scope name children =
    View.column
      ~key:(key name)
      ~style:(Style.create_exn [ Width full; Shrink 0.; Gap (px 0.) ])
      children
  in
  let%bind header =
    Option.map header ~f:section
    |> Option.value_map
         ~default:(Ok [])
         ~f:(Or_error.map ~f:(fun h -> [ scope "header" [ h ] ]))
  in
  let%bind bodies = List.map sections ~f:section |> Or_error.all in
  let%bind footer =
    Option.map footer ~f:section
    |> Option.value_map
         ~default:(Ok [])
         ~f:(Or_error.map ~f:(fun f -> [ scope "footer" [ f ] ]))
  in
  let%bind caption =
    match caption with
    | None -> Ok []
    | Some caption ->
      let%map view = semantic (scope "caption" [ caption ]) Caption in
      [ view ]
  in
  let open Style.Property in
  let make_style = Style.create_exn in
  View.column
    ?key:root_key
    ~style:
      (Style.merge
         [ make_style [ Width full; Min_width (px 0.) ]
         ; style
         ; make_style [ Display Flex; Direction Column; Gap (px 0.) ]
         ])
    (header @ [ scope "body" bodies ] @ footer @ caption)
  |> fun view -> View.with_accessibility view metadata
;;
