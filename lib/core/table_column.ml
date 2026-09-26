open Core

let max_columns = 64
let max_header_levels = 4
let max_text_bytes = 262_144

let validate_text ~name ~max_bytes text =
  if String.is_empty text || String.length text > max_bytes
  then Or_error.errorf "%s must contain 1..%d bytes" name max_bytes
  else if (not (Stdlib.String.is_valid_utf_8 text)) || String.contains text '\000'
  then Or_error.errorf "%s must be UTF-8 without NUL" name
  else Ok ()
;;

module Id = struct
  module T = struct
    type t = string [@@deriving compare, equal, sexp_of]
  end

  include T
  include Comparator.Make (T)

  let of_string text =
    let%map.Or_error () = validate_text ~name:"table column id" ~max_bytes:256 text in
    text
  ;;

  let to_string t = t
end

module Pin = struct
  type t =
    | Unpinned
    | Left
  [@@deriving equal, sexp_of]
end

module Alignment = struct
  type t =
    | Left
    | Center
    | Right
  [@@deriving equal, sexp_of]
end

type t =
  { id : Id.t
  ; label : string
  ; width : float
  ; min_width : float
  ; max_width : float
  ; pin : Pin.t
  ; alignment : Alignment.t
  ; resizable : bool
  ; movable : bool
  ; sortable : bool
  }
[@@deriving equal, sexp_of]

let validate_width ~width ~min_width ~max_width =
  if
    Float.is_finite width
    && Float.is_finite min_width
    && Float.is_finite max_width
    && Float.(20. <= min_width && min_width <= width && width <= max_width)
    && Float.(max_width <= 16384.)
  then Ok ()
  else
    Or_error.error_string "table widths require finite 20 <= min <= width <= max <= 16384"
;;

let create
      ~id
      ~label
      ?(width = 160.)
      ?(min_width = 40.)
      ?(max_width = 4096.)
      ?(pin = Pin.Unpinned)
      ?(alignment = Alignment.Left)
      ?(resizable = true)
      ?(movable = true)
      ?(sortable = false)
      ()
  =
  let%bind.Or_error () = validate_text ~name:"table column label" ~max_bytes:4096 label in
  let%map.Or_error () = validate_width ~width ~min_width ~max_width in
  { id; label; width; min_width; max_width; pin; alignment; resizable; movable; sortable }
;;

let id t = t.id
let label t = t.label
let width t = t.width
let min_width t = t.min_width
let max_width t = t.max_width
let pin t = t.pin
let alignment t = t.alignment
let is_resizable t = t.resizable
let is_movable t = t.movable
let is_sortable t = t.sortable

let with_width t width =
  let%map.Or_error () =
    validate_width ~width ~min_width:t.min_width ~max_width:t.max_width
  in
  { t with width }
;;

module Group = struct
  type t =
    { label : string
    ; columns : Id.t list
    }
  [@@deriving equal, sexp_of]

  let create ~label ~columns =
    let%bind.Or_error () =
      validate_text ~name:"table group label" ~max_bytes:4096 label
    in
    let count = List.length columns in
    if count = 0 || count > max_columns
    then Or_error.errorf "table group requires 1..%d columns" max_columns
    else if Set.length (Set.of_list (module Id) columns) <> count
    then Or_error.error_string "duplicate table group column"
    else Ok { label; columns }
  ;;

  let label t = t.label
  let columns t = t.columns
end

module Collection = struct
  type column = t [@@deriving equal, sexp_of]

  type t =
    { columns : column list
    ; header_groups : Group.t list list
    }
  [@@deriving equal, sexp_of]

  let max_columns = max_columns
  let max_header_levels = max_header_levels
  let max_text_bytes = max_text_bytes
  let to_list t = t.columns
  let header_groups t = t.header_groups
  let find t key = List.find t.columns ~f:(fun column -> Id.equal column.id key)

  let index t key =
    List.find_mapi t.columns ~f:(fun index column ->
      Option.some_if (Id.equal column.id key) index)
  ;;

  let pinned_count t = List.count t.columns ~f:(fun column -> Pin.equal column.pin Left)
  let total_width t = List.sum (module Float) t.columns ~f:width

  let text_bytes t =
    let columns =
      List.sum
        (module Int)
        t.columns
        ~f:(fun column ->
          String.length (Id.to_string column.id) + String.length column.label)
    in
    columns
    + List.sum
        (module Int)
        t.header_groups
        ~f:(fun groups ->
          List.sum
            (module Int)
            groups
            ~f:(fun group ->
              String.length (Group.label group)
              + List.sum
                  (module Int)
                  (Group.columns group)
                  ~f:(fun key -> String.length (Id.to_string key))))
  ;;

  let validate_pin_partition columns =
    List.fold_result columns ~init:false ~f:(fun unpinned column ->
      match column.pin with
      | Pin.Unpinned -> Ok true
      | Left ->
        if unpinned
        then Or_error.error_string "left-pinned table columns must form a prefix"
        else Ok false)
    |> Or_error.map ~f:ignore
  ;;

  let validate_groups t =
    let keys = List.map t.columns ~f:id in
    let pinned = pinned_count t in
    List.fold_result
      t.header_groups
      ~init:Int.Set.empty
      ~f:(fun previous_boundaries groups ->
        let flattened = List.concat_map groups ~f:Group.columns in
        if List.is_empty groups || not (List.equal Id.equal keys flattened)
        then
          Or_error.error_string
            "table header level must partition columns in display order"
        else (
          let _, boundaries =
            List.fold
              groups
              ~init:(0, Int.Set.empty)
              ~f:(fun (offset, boundaries) group ->
                let offset = offset + List.length (Group.columns group) in
                offset, Set.add boundaries offset)
          in
          if pinned > 0 && not (Set.mem boundaries pinned)
          then Or_error.error_string "table header group crosses the pinned boundary"
          else if not (Set.is_subset previous_boundaries ~of_:boundaries)
          then Or_error.error_string "table header levels must refine their parent groups"
          else Ok boundaries))
    |> Or_error.map ~f:ignore
  ;;

  let create ?(header_groups = []) columns =
    let count = List.length columns in
    if count > max_columns
    then Or_error.errorf "table exceeds %d columns" max_columns
    else if List.length header_groups > max_header_levels
    then Or_error.errorf "table exceeds %d group header levels" max_header_levels
    else if List.exists header_groups ~f:(fun groups -> List.length groups > max_columns)
    then Or_error.errorf "table header level exceeds %d groups" max_columns
    else if Set.length (Set.of_list (module Id) (List.map columns ~f:id)) <> count
    then Or_error.error_string "duplicate table column id"
    else (
      let t = { columns; header_groups } in
      if text_bytes t > max_text_bytes
      then Or_error.errorf "table schema exceeds %d text bytes" max_text_bytes
      else (
        let%bind.Or_error () = validate_pin_partition columns in
        let%map.Or_error () = validate_groups t in
        t))
  ;;

  let find_required t key =
    match find t key with
    | Some column -> Ok column
    | None -> Or_error.errorf "unknown table column: %s" (Id.to_string key)
  ;;

  let resize t ~column:key ~width =
    let%bind.Or_error column = find_required t key in
    if not column.resizable
    then Or_error.error_string "table column is not user-resizable"
    else if not (Float.is_finite width)
    then Or_error.error_string "table resize width must be finite"
    else (
      let width = Float.min column.max_width (Float.max column.min_width width) in
      let columns =
        List.map t.columns ~f:(fun column ->
          if Id.equal column.id key then { column with width } else column)
      in
      Ok { t with columns })
  ;;

  let reorder_groups columns groups =
    let positions =
      Map.of_alist_exn
        (module Id)
        (List.mapi columns ~f:(fun index column -> column.id, index))
    in
    let position key = Map.find_exn positions key in
    let groups =
      List.map groups ~f:(fun (group : Group.t) ->
        { group with
          columns =
            List.sort group.columns ~compare:(fun a b ->
              Int.compare (position a) (position b))
        })
    in
    List.sort groups ~compare:(fun (a : Group.t) (b : Group.t) ->
      Int.compare (position (List.hd_exn a.columns)) (position (List.hd_exn b.columns)))
  ;;

  let move t ~column:key ~before =
    let%bind.Or_error column = find_required t key in
    let%bind.Or_error () =
      match before with
      | None -> Ok ()
      | Some key -> find_required t key |> Or_error.map ~f:ignore
    in
    if not column.movable
    then Or_error.error_string "table column is not user-movable"
    else if Option.value_map before ~default:false ~f:(Id.equal key)
    then Ok t
    else (
      let remaining =
        List.filter t.columns ~f:(fun column -> not (Id.equal column.id key))
      in
      let columns =
        match before with
        | None -> remaining @ [ column ]
        | Some destination ->
          List.concat_map remaining ~f:(fun other ->
            if Id.equal other.id destination then [ column; other ] else [ other ])
      in
      let header_groups = List.map t.header_groups ~f:(reorder_groups columns) in
      create ~header_groups columns)
  ;;
end
