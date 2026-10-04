open Core
module W = Gpuio_protocol.Color_input_wire
module P = Gpuio_protocol.Color_presentation_wire
module Channel = W.Channel
module Field = W.Field

let rgba_to_wire color =
  let module R = Color_value.Rgba in
  Int64.(
    bit_or
      (shift_left (of_int (R.red color)) 24)
      (bit_or
         (shift_left (of_int (R.green color)) 16)
         (bit_or (shift_left (of_int (R.blue color)) 8) (of_int (R.alpha color)))))
;;

let rgba_of_wire_exn value =
  let channel shift =
    Int64.(bit_and (shift_right_logical value shift) 255L |> to_int_exn)
  in
  Color_value.Rgba.create
    ~red:(channel 24)
    ~green:(channel 16)
    ~blue:(channel 8)
    ~alpha:(channel 0)
  |> Or_error.ok_exn
;;

let value_to_wire = function
  | Color_value.Value.Empty -> W.Value.Empty
  | Color color -> W.Value.Color (rgba_to_wire color)
;;

let value_of_wire_exn = function
  | W.Value.Empty -> Color_value.Value.Empty
  | Color value -> Color_value.Value.Color (rgba_of_wire_exn value)
;;

let channels_of_wire_exn (t : W.Hsla.t) =
  Color_value.Hsla.create
    ~hue_degrees:t.hue_degrees
    ~saturation:t.saturation
    ~lightness:t.lightness
    ~alpha:t.alpha
  |> Or_error.ok_exn
;;

module Palette_entry = struct
  type t = W.Palette_entry.t [@@deriving equal, sexp_of]

  let create ~color ~label =
    let t = { W.Palette_entry.color = rgba_to_wire color; label } in
    if W.Palette_entry.valid t
    then Ok t
    else
      Or_error.error_string
        "color palette label must be nonblank UTF-8 without NUL, at most 256 bytes"
  ;;

  let color t = rgba_of_wire_exn t.W.Palette_entry.color
  let label t = t.W.Palette_entry.label
end

module Labels = struct
  type t = W.Labels.t [@@deriving equal, sexp_of]

  let create ~control ~hue ~saturation ~lightness ~alpha ~hex ~clear =
    let t = { W.Labels.control; hue; saturation; lightness; alpha; hex; clear } in
    if W.Labels.valid t
    then Ok t
    else
      Or_error.error_string
        "color labels must be nonblank UTF-8 without NUL, at most 4096 bytes each"
  ;;

  let english ~control =
    create
      ~control
      ~hue:"Hue"
      ~saturation:"Saturation"
      ~lightness:"Lightness"
      ~alpha:"Opacity"
      ~hex:"Hex color"
      ~clear:"Clear color"
  ;;
end

module Palette_section = struct
  type t =
    { layout : P.Section.t
    ; entries : Palette_entry.t list
    }
  [@@deriving equal, sexp_of]

  let create ~featured ~label entries =
    let layout =
      { P.Section.featured; label; count = Int64.of_int (List.length entries) }
    in
    if P.Section.valid layout
    then Ok { layout; entries }
    else Or_error.error_string "invalid palette section label or entry count (1..256)"
  ;;

  let featured ~label entries = create ~featured:true ~label entries
  let group ~label entries = create ~featured:false ~label entries
end

module Panel = P.Panel

module Panels = struct
  type t = P.Panels.t [@@deriving equal, sexp_of]

  let all = P.Panels.All

  let tabs ?(initial = Panel.Palette) ~palette_label ~channels_label () =
    let t = P.Panels.Tabs { palette_label; channels_label; initial } in
    if P.Panels.valid t then Ok t else Or_error.error_string "invalid color panel labels"
  ;;
end

module Appearance = struct
  type t =
    { geometry : P.t
    ; selected_border : Color.t option
    ; hover_border : Color.t option
    }
  [@@deriving equal, sexp_of]

  let create
        ?(swatch_size = 28.)
        ?(featured_size = 36.)
        ?(swatch_gap = 6.)
        ?(section_gap = 10.)
        ?(swatch_radius = 5.)
        ?(outline_width = 2.)
        ?(channel_height = 28.)
        ?(control_gap = 10.)
        ?(padding = 10.)
        ?selected_border
        ?hover_border
        ?(panels = Panels.all)
        ()
    =
    let geometry =
      { P.default with
        swatch_size
      ; featured_size
      ; swatch_gap
      ; section_gap
      ; swatch_radius
      ; outline_width
      ; channel_height
      ; control_gap
      ; padding
      ; panels
      }
    in
    if P.valid geometry
    then Ok { geometry; selected_border; hover_border }
    else Or_error.error_string "invalid color appearance dimensions"
  ;;

  let default = create () |> Or_error.ok_exn
end

module Config = struct
  type t =
    { wire : W.Config.t
    ; sections : P.Section.t list
    }
  [@@deriving equal, sexp_of]

  let create
        ~labels
        ?palette
        ?palette_sections
        ?(alpha_policy = Color_value.Alpha_policy.Allow_alpha)
        ?(allow_empty = true)
        ?(disabled = false)
        ?(read_only = false)
        ()
    =
    let open Or_error.Let_syntax in
    let%bind palette, sections =
      match palette, palette_sections with
      | Some _, Some _ ->
        Or_error.error_string "supply palette or palette_sections, not both"
      | None, None -> Ok ([], [])
      | Some entries, None -> Ok (entries, [])
      | None, Some sections ->
        let layout = List.map sections ~f:(fun s -> s.Palette_section.layout) in
        if P.valid_sections layout
        then Ok (List.concat_map sections ~f:(fun s -> s.Palette_section.entries), layout)
        else
          Or_error.error_string
            "invalid palette sections: at most 32 sections, 256 entries, featured first"
    in
    let alpha_policy =
      match alpha_policy with
      | Allow_alpha -> W.Alpha_policy.Allow_alpha
      | Opaque_only -> W.Alpha_policy.Opaque_only
    in
    let t =
      { W.Config.labels; palette; alpha_policy; allow_empty; disabled; read_only }
    in
    if W.Config.valid t
    then Ok { wire = t; sections }
    else
      Or_error.error_string "invalid color configuration or palette limit (256 entries)"
  ;;

  let allows t value = W.Config.allows t.wire (value_to_wire value)
  let is_disabled t = t.wire.disabled
  let is_read_only t = t.wire.read_only
end

module Revision = struct
  type t = int64 [@@deriving equal, compare, sexp_of]

  let of_int64 t =
    if Int64.(t >= 0L) then Ok t else Or_error.error_string "negative color revision"
  ;;

  let to_int64 t = t
end

module Interaction_id = struct
  type t = int64 [@@deriving equal, compare, sexp_of]

  let to_int64 t = t
end

module Interaction = struct
  module Kind = W.Interaction_kind

  type t = W.Interaction.t [@@deriving equal, sexp_of]

  let id t = t.W.Interaction.id
  let kind t = t.W.Interaction.kind
end

module Draft = struct
  module Status = W.Draft_status

  type t = W.Draft.t [@@deriving equal, sexp_of]

  let text t = t.W.Draft.text
  let is_composing t = t.W.Draft.composing
  let status t = t.W.Draft.status
end

module Snapshot = struct
  type t =
    { window : Gpuio_protocol.Window_id.t
    ; node : Gpuio_protocol.Node_id.t
    ; data : W.Snapshot.t
    }
  [@@deriving equal, sexp_of]

  let revision t = t.data.revision
  let value t = value_of_wire_exn t.data.value
  let committed t = value_of_wire_exn t.data.committed
  let channels t = channels_of_wire_exn t.data.channels
  let interaction t = t.data.interaction
  let draft t = t.data.draft
  let value_allowed t = t.data.value_allowed
  let committed_allowed t = t.data.committed_allowed
end

module Source = W.Source
module Cancel_reason = W.Cancel_reason

module Event = struct
  type t =
    | Observed of Snapshot.t
    | Started of Snapshot.t
    | Preview of Snapshot.t
    | Committed of Source.t * Snapshot.t
    | Cancelled of Cancel_reason.t * Snapshot.t
  [@@deriving equal, sexp_of]

  let snapshot = function
    | Observed s | Started s | Preview s | Committed (_, s) | Cancelled (_, s) -> s
  ;;
end

module Command = struct
  type t =
    | Set of
        { value : Color_value.Value.t
        ; if_revision : Revision.t option
        }
    | Reset of { if_revision : Revision.t option }
    | Cancel
    | Focus of Field.t
    | Read_snapshot
  [@@deriving equal, sexp_of]
end

module Command_error = W.Error

module Expert = struct
  let config_to_wire t = t.Config.wire

  let config_of_wire t =
    if W.Config.valid t
    then Ok { Config.wire = t; sections = [] }
    else Or_error.error_string "invalid color configuration"
  ;;

  let presentation_to_wire (config : Config.t) ~appearance:(t : Appearance.t) ~theme =
    let open Or_error.Let_syntax in
    let resolve = function
      | None -> return None
      | Some color -> Theme.resolve theme color |> Or_error.map ~f:Option.some
    in
    let%bind selected_border = resolve t.selected_border in
    let%map hover_border = resolve t.hover_border in
    { t.geometry with sections = config.sections; selected_border; hover_border }
  ;;

  let value_to_wire = value_to_wire

  let snapshot_of_wire ~window ~node data =
    if W.Snapshot.valid data
    then Ok { Snapshot.window; node; data }
    else Or_error.error_string "invalid color snapshot"
  ;;

  let event_of_wire ~window ~node event =
    if not (W.Event.valid event)
    then Or_error.error_string "invalid color event"
    else (
      let%map.Or_error s = snapshot_of_wire ~window ~node (W.Event.snapshot event) in
      match event with
      | Observed _ -> Event.Observed s
      | Started _ -> Started s
      | Preview _ -> Preview s
      | Committed (source, _) -> Committed (source, s)
      | Cancelled (reason, _) -> Cancelled (reason, s))
  ;;

  let window t = t.Snapshot.window
  let node t = t.Snapshot.node

  let command_to_wire = function
    | Command.Set { value; if_revision } ->
      W.Command.Set { value = value_to_wire value; if_revision }
    | Reset { if_revision } -> W.Command.Reset { if_revision }
    | Cancel -> W.Command.Cancel
    | Focus field -> W.Command.Focus field
    | Read_snapshot -> W.Command.Read_snapshot
  ;;

  let error_of_wire t = t
end
