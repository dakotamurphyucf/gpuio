open Core

module Tree_item = struct
  type t =
    { level : int
    ; index : int
    ; count : int option
    ; expanded : bool option
    ; selected : bool
    ; disabled : bool
    ; busy : bool
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    t.level >= 1
    && t.level <= 128
    && t.index >= 0
    && t.index < 100_000
    && Option.for_all t.count ~f:(fun count -> count > t.index && count <= 100_000)
  ;;
end

module Option_item = struct
  type t =
    { index : int
    ; count : int option
    ; selected : bool
    ; disabled : bool
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    t.index >= 0
    && t.index < 1_000_000
    && Option.for_all t.count ~f:(fun count -> count > t.index && count <= 1_000_000)
  ;;
end

module Orientation = struct
  type t =
    | Horizontal
    | Vertical
  [@@deriving bin_io, equal, sexp_of]
end

module Table_info = struct
  type t =
    { rows : int option
    ; columns : int option
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Option.for_all t.rows ~f:(fun rows -> rows >= 0 && rows <= 1_000_000)
    && Option.for_all t.columns ~f:(fun columns -> columns >= 0 && columns <= 1024)
  ;;
end

module Table_cell = struct
  type t =
    { row : int
    ; column : int
    ; column_span : int
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    t.row >= 0
    && t.row < 1_000_000
    && t.column >= 0
    && t.column < 1024
    && t.column_span > 0
    && t.column_span <= 1024 - t.column
  ;;
end

module Role = struct
  type t =
    | Group
    | Label
    | Link
    | Separator
    | Description_list
    | Term
    | Definition
    | Status
    | Alert
    | Image
    | Heading of int
    | Navigation
    | Tree of bool
    | Tree_item of Tree_item.t
    | Toolbar of Orientation.t
    | Radio_group of Orientation.t
    | Log
    | List_box of bool
    | Option_item of Option_item.t
    | Table of Table_info.t
    | Row_group
    | Table_row of int
    | Table_cell of Table_cell.t
    | Column_header of Table_cell.t
    | Row_header of Table_cell.t
    | Caption
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Heading level -> level >= 1 && level <= 6
    | Tree_item item -> Tree_item.valid item
    | Option_item item -> Option_item.valid item
    | Table info -> Table_info.valid info
    | Table_row index -> index >= 0 && index < 1_000_000
    | Table_cell cell | Column_header cell | Row_header cell -> Table_cell.valid cell
    | Row_group | Caption -> true
    | Group
    | Label
    | Link
    | Separator
    | Description_list
    | Term
    | Definition
    | Status
    | Alert
    | Image
    | Navigation
    | Tree _
    | Toolbar _
    | Radio_group _
    | Log
    | List_box _ -> true
  ;;
end

module Current = struct
  type t =
    | True
    | Page
    | Step
    | Location
    | Date
    | Time
  [@@deriving bin_io, equal, sexp_of]
end

module Live = struct
  type t =
    | Off
    | Polite
    | Assertive
  [@@deriving bin_io, equal, sexp_of]
end

let valid_text text =
  (not (String.is_empty text))
  && String.length text <= 4096
  && Stdlib.String.is_valid_utf_8 text
  && not (String.contains text '\000')
;;

module Field = struct
  type t =
    { label : string
    ; help : string option
    ; error : string option
    ; required : bool
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    valid_text t.label
    && Option.for_all t.help ~f:valid_text
    && Option.for_all t.error ~f:valid_text
  ;;
end

module Config = struct
  type t =
    { role : Role.t option
    ; label : string option
    ; description : string option
    ; live : Live.t
    ; field : Field.t option
    ; current : Current.t option
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Option.for_all t.role ~f:Role.valid
    && Option.for_all t.label ~f:valid_text
    && Option.for_all t.description ~f:valid_text
    && (Option.is_none t.current || Option.is_some t.description)
    && Option.for_all t.field ~f:(fun field ->
      Field.valid field
      && Option.is_none t.current
      && Option.is_none t.role
      && Option.is_none t.label
      && Option.is_none t.description
      && Live.equal t.live Off)
  ;;
end
