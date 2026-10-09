open Core
module W = Gpuio_protocol.Split_group_wire

module Id = struct
  type t = string [@@deriving equal, compare, sexp_of]

  let of_string s =
    if W.valid_id s then Ok s else Or_error.error_string "invalid split panel ID"
  ;;

  let to_string t = t
end

module Axis = Split_pane.Axis

module Appearance = struct
  type t =
    { thickness : float
    ; hit_extent : float
    ; handle_style : Style.t
    ; item_styles : (Id.t * Style.t) list
    }
  [@@deriving equal, sexp_of]

  let create
        ?(thickness = 1.)
        ?(hit_extent = 8.)
        ?(handle_style = Style.empty)
        ?(item_styles = [])
        ()
    =
    let open Or_error.Let_syntax in
    let bounded v minimum maximum =
      Float.is_finite v && Float.(v >= minimum && v <= maximum)
    in
    let%bind () =
      if
        bounded thickness 1. 16.
        && bounded hit_extent 8. 32.
        && Float.(thickness <= hit_extent)
      then Ok ()
      else Or_error.error_string "invalid split group appearance geometry"
    in
    let%bind () =
      if List.length item_styles <= 64
      then Ok ()
      else Or_error.error_string "split group appearance exceeds 64 overrides"
    in
    let%bind _ =
      Map.of_alist_or_error
        (module String)
        (List.map item_styles ~f:(fun (id, style) -> Id.to_string id, style))
    in
    let parts = handle_style :: List.map item_styles ~f:snd in
    let%bind () =
      List.fold_result parts ~init:() ~f:(fun () style ->
        Style.Expert.validate_scope
          style
          ~states:[ Base; Focused; Hovered; Pressed; Disabled ]
          ~properties:
            [ Background
            ; Foreground
            ; Opacity
            ; Border_color
            ; Border_top_width
            ; Border_right_width
            ; Border_bottom_width
            ; Border_left_width
            ; Border_style
            ; Top_left_radius
            ; Top_right_radius
            ; Bottom_left_radius
            ; Bottom_right_radius
            ; Shadows
            ; Font_size
            ; Font_family
            ; Font_weight
            ; Text_align
            ; Line_height
            ; White_space
            ; Text_overflow
            ; Line_clamp
            ; Text_decoration
            ])
    in
    let%map () =
      if List.sum (module Int) parts ~f:Style.Expert.declaration_count <= 256
      then Ok ()
      else Or_error.error_string "split group appearance exceeds 256 declarations"
    in
    { thickness; hit_extent; handle_style; item_styles }
  ;;

  let default = create () |> Or_error.ok_exn
end

module Panel = struct
  type t = W.Panel.t [@@deriving equal, sexp_of]

  let create
        id
        ~label
        ?initial_size
        ?(minimum_size = 80.)
        ?(maximum_size = 16384.)
        ?(visible = true)
        ()
    =
    let t = { W.Panel.id; label; initial_size; minimum_size; maximum_size; visible } in
    if W.Panel.valid t
    then Ok t
    else Or_error.error_string "invalid split panel label, seed or range"
  ;;

  let id (t : t) = t.id
  let label (t : t) = t.label
  let is_visible (t : t) = t.visible
end

module Resize_request = struct
  type t = W.Resize_request.t [@@deriving equal, sexp_of]

  let create id ~size ~serial =
    let t = { W.Resize_request.id; size; serial } in
    if W.Resize_request.valid t
    then Ok t
    else Or_error.error_string "invalid split resize request"
  ;;
end

module Config = struct
  type t = W.Config.t [@@deriving equal, sexp_of]

  let max_panels = W.max_panels

  let create
        ~label
        ?(axis = Axis.Horizontal)
        ?(keyboard_step = 16.)
        ?(reset_generation = 0L)
        ?resize
        panels
    =
    let axis =
      match axis with
      | Axis.Horizontal -> Gpuio_protocol.Split_wire.Axis.Horizontal
      | Vertical -> Vertical
    in
    let t = { W.Config.label; axis; keyboard_step; reset_generation; resize; panels } in
    if W.Config.valid t
    then Ok t
    else Or_error.error_string "invalid split group label, generation or panel collection"
  ;;

  let panels (t : t) = t.panels
end

module Source = struct
  type t = W.Source.t =
    | Pointer
    | Keyboard
    | Accessibility
    | Request of int64
  [@@deriving equal, sexp_of]
end

module Snapshot = struct
  type t = W.Snapshot.t [@@deriving equal, sexp_of]

  let sizes (t : t) = t.sizes
  let source (t : t) = t.source
end

module Expert = struct
  let appearance_to_wire (t : Appearance.t) ~theme =
    let open Or_error.Let_syntax in
    let%bind handle_style = Style.Expert.to_wire t.handle_style ~theme in
    let%map item_styles =
      Or_error.all
        (List.map t.item_styles ~f:(fun (id, style) ->
           let%map style = Style.Expert.to_wire style ~theme in
           Id.to_string id, style))
    in
    { Gpuio_protocol.Wire.Split_group_appearance.thickness = t.thickness
    ; hit_extent = t.hit_extent
    ; handle_style
    ; item_styles
    }
  ;;

  let to_wire t = t
  let generation (t : Config.t) = t.reset_generation

  let snapshot_of_wire config snapshot =
    if W.Snapshot.valid_for snapshot config
    then Ok snapshot
    else
      Or_error.error_string "split snapshot does not match current panel IDs and ranges"
  ;;
end
