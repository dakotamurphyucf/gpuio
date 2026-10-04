open Core

module Format = struct
  type t =
    | Bold
    | Italic
    | Monospace
  [@@deriving equal, sexp_of]

  let all = [ Bold; Italic; Monospace ]

  let command_id t =
    let name =
      match t with
      | Bold -> "bold"
      | Italic -> "italic"
      | Monospace -> "monospace"
    in
    Gpuio.Command.Id.of_string ("gallery.format." ^ name) |> Or_error.ok_exn
  ;;

  let label = function
    | Bold -> "Bold"
    | Italic -> "Italic"
    | Monospace -> "Monospace"
  ;;
end

module Alignment = struct
  type t =
    | Left
    | Center
    | Right
  [@@deriving equal, sexp_of]

  let all = [ Left; Center; Right ]

  let command_id t =
    let name =
      match t with
      | Left -> "left"
      | Center -> "center"
      | Right -> "right"
    in
    Gpuio.Command.Id.of_string ("gallery.align." ^ name) |> Or_error.ok_exn
  ;;

  let label = function
    | Left -> "Align left"
    | Center -> "Align center"
    | Right -> "Align right"
  ;;
end

type t =
  { enabled : bool
  ; italic_enabled : bool
  ; bold_loading : bool
  ; bold : bool
  ; italic : bool
  ; monospace : bool
  ; alignment : Alignment.t
  }

module Action = struct
  type t =
    | Toggle_enabled
    | Toggle_italic_enabled
    | Toggle_bold_loading
    | Toggle_format of Format.t
    | Align of Alignment.t
    | Toggle_all
end

let initial =
  { enabled = true
  ; italic_enabled = true
  ; bold_loading = false
  ; bold = true
  ; italic = false
  ; monospace = false
  ; alignment = Left
  }
;;

let enabled t = t.enabled
let italic_enabled t = t.italic_enabled
let bold_loading t = t.bold_loading

let format_enabled t format =
  t.enabled && ((not (Format.equal format Italic)) || t.italic_enabled)
;;

let format_loading t format = Format.equal format Bold && t.bold_loading
let can_toggle t format = format_enabled t format && not (format_loading t format)
let alignment t = t.alignment

let selected t = function
  | Format.Bold -> t.bold
  | Italic -> t.italic
  | Monospace -> t.monospace
;;

let master t =
  let count = List.count Format.all ~f:(selected t) in
  if count = 0
  then Gpuio.Check_state.Unchecked
  else if count = List.length Format.all
  then Checked
  else Indeterminate
;;

let apply t = function
  | Action.Toggle_enabled -> { t with enabled = not t.enabled }
  | Toggle_italic_enabled -> { t with italic_enabled = not t.italic_enabled }
  | Toggle_bold_loading -> { t with bold_loading = not t.bold_loading }
  | (Toggle_format _ | Align _ | Toggle_all) when not t.enabled -> t
  | Toggle_format format when not (can_toggle t format) -> t
  | Toggle_format Bold -> { t with bold = not t.bold }
  | Toggle_format Italic -> { t with italic = not t.italic }
  | Toggle_format Monospace -> { t with monospace = not t.monospace }
  | Align alignment -> { t with alignment }
  | Toggle_all ->
    let selected =
      Gpuio.Check_state.equal (Gpuio.Check_state.activate (master t)) Checked
    in
    { t with
      bold = (if can_toggle t Bold then selected else t.bold)
    ; italic = (if can_toggle t Italic then selected else t.italic)
    ; monospace = (if can_toggle t Monospace then selected else t.monospace)
    }
;;
