open Core
module W = Gpuio_protocol.Accessibility_wire

module Tree_item = struct
  type t = W.Tree_item.t [@@deriving equal, sexp_of]

  let create
        ~level
        ~index
        ?count
        ?expanded
        ?(selected = false)
        ?(disabled = false)
        ?(busy = false)
        ()
    =
    let item = { W.Tree_item.level; index; count; expanded; selected; disabled; busy } in
    if W.Tree_item.valid item
    then Ok item
    else Or_error.error_string "invalid tree item level, sibling index or count"
  ;;
end

module Option_item = struct
  type t = W.Option_item.t [@@deriving equal, sexp_of]

  let create ~index ?count ?(selected = false) ?(disabled = false) () =
    let item = { W.Option_item.index; count; selected; disabled } in
    if W.Option_item.valid item
    then Ok item
    else Or_error.error_string "invalid list option index or count"
  ;;
end

module Orientation = W.Orientation

module Table_info = struct
  type t = W.Table_info.t [@@deriving equal, sexp_of]

  let create ?rows ?columns () =
    let t = { W.Table_info.rows; columns } in
    if W.Table_info.valid t
    then Ok t
    else Or_error.error_string "invalid table row/column count"
  ;;
end

module Table_cell = struct
  type t = W.Table_cell.t [@@deriving equal, sexp_of]

  let create ~row ~column ?(column_span = 1) () =
    let t = { W.Table_cell.row; column; column_span } in
    if W.Table_cell.valid t
    then Ok t
    else Or_error.error_string "invalid table cell index or column span"
  ;;
end

module Role = W.Role
module Current = W.Current
module Live = W.Live

module Field = struct
  type t = W.Field.t [@@deriving equal, sexp_of]

  let create ~label ?help ?error ?(required = false) () =
    let field = { W.Field.label; help; error; required } in
    if W.Field.valid field
    then Ok field
    else
      Or_error.error_string
        "field text must be nonempty UTF-8 without NUL, at most 4096 bytes"
  ;;

  let label t = t.W.Field.label
  let help t = t.W.Field.help
  let error t = t.W.Field.error
  let is_required t = t.W.Field.required
end

type t = W.Config.t [@@deriving equal, sexp_of]

let create ?role ?label ?description ?live ?current () =
  let live =
    Option.value
      live
      ~default:
        (match role with
         | Some (Role.Status | Log) -> Live.Polite
         | Some Alert -> Assertive
         | None
         | Some
             ( Group
             | Label
             | Link
             | Separator
             | Description_list
             | Term
             | Definition
             | Image
             | Heading _
             | Navigation
             | Tree _
             | Tree_item _
             | List_box _
             | Option_item _
             | Table _
             | Row_group
             | Table_row _
             | Table_cell _
             | Column_header _
             | Row_header _
             | Caption
             | Toolbar _
             | Radio_group _ ) -> Off)
  in
  let config = { W.Config.role; label; description; live; field = None; current } in
  if W.Config.valid config
  then Ok config
  else
    Or_error.error_string
      "invalid accessibility text, heading level, or current item without description"
;;

let field field =
  { W.Config.role = None
  ; label = None
  ; description = None
  ; live = Off
  ; field = Some field
  ; current = None
  }
;;

module Expert = struct
  let to_wire t = t
end
