open Core
module W = Gpuio_protocol.Wire.Tab_appearance
module Variant = W.Variant

module Motion = struct
  type t = Gpuio_protocol.Wire.Tab_motion.t [@@deriving equal, sexp_of]

  let default_spring =
    Animation.Spring.create
      ~stiffness:400.
      ~damping:40.
      ~mass:1.
      ~epsilon:0.01
      ~max_duration:(Time_ns.Span.of_sec 2.)
      ()
    |> Or_error.ok_exn
  ;;

  let create ?(spring = default_spring) ?(color_duration = Time_ns.Span.of_ms 200.) () =
    let milliseconds = Time_ns.Span.to_ms color_duration in
    if
      (not (Float.is_finite milliseconds))
      || Float.(milliseconds < 0. || milliseconds > 60_000.)
    then Or_error.error_string "tab color duration must be between zero and 60 seconds"
    else
      Ok
        { Gpuio_protocol.Wire.Tab_motion.spring = Animation.Expert.spring_to_wire spring
        ; color_duration_ms = Float.iround_up_exn milliseconds |> Int64.of_int
        }
  ;;

  let default = create () |> Or_error.ok_exn
end

module Appearance = struct
  type t =
    { variant : Variant.t
    ; height : float
    ; gap : float
    ; padding : float
    ; tab_style : Style.t
    ; item_styles : (Choice.Id.t * Style.t) list
    }
  [@@deriving equal, sexp_of]

  let create
        ?(variant = Variant.Tab)
        ?(height = 32.)
        ?(gap = 4.)
        ?(padding = 12.)
        ?(tab_style = Style.empty)
        ?(item_styles = [])
        ()
    =
    let open Or_error.Let_syntax in
    let bounded v minimum maximum =
      Float.is_finite v && Float.(v >= minimum && v <= maximum)
    in
    let%bind () =
      if bounded height 16. 256. && bounded gap 0. 128. && bounded padding 0. 128.
      then Ok ()
      else Or_error.error_string "invalid tab appearance geometry"
    in
    let%bind () =
      if List.length item_styles <= 128
      then Ok ()
      else Or_error.error_string "tab appearance exceeds 128 overrides"
    in
    let%bind _ =
      Map.of_alist_or_error
        (module String)
        (List.map item_styles ~f:(fun (id, style) -> Choice.Id.to_string id, style))
    in
    let parts = tab_style :: List.map item_styles ~f:snd in
    let%bind () =
      List.fold_result parts ~init:() ~f:(fun () style ->
        Style.Expert.validate_scope
          style
          ~states:[ Base; Focused; Hovered; Pressed; Disabled; Selected ]
          ~properties:
            [ Width
            ; Height
            ; Min_width
            ; Min_height
            ; Max_width
            ; Max_height
            ; Grow
            ; Shrink
            ; Basis
            ; Align_items
            ; Justify_content
            ; Row_gap
            ; Column_gap
            ; Padding_top
            ; Padding_right
            ; Padding_bottom
            ; Padding_left
            ; Margin_top
            ; Margin_right
            ; Margin_bottom
            ; Margin_left
            ; Background
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
            ; Cursor
            ])
    in
    let%map () =
      if List.sum (module Int) parts ~f:Style.Expert.declaration_count <= 256
      then Ok ()
      else Or_error.error_string "tab appearance exceeds 256 declarations"
    in
    { variant; height; gap; padding; tab_style; item_styles }
  ;;

  let default = create () |> Or_error.ok_exn
  let variant t = t.variant
end

module Reveal_request = struct
  type t =
    { serial : int64
    ; target : Choice.Id.t
    }
  [@@deriving equal, sexp_of]

  let create target ~serial =
    if Int64.(serial < 1L)
    then Or_error.error_string "tab reveal serial must be positive"
    else Ok { serial; target }
  ;;
end

module Viewport = struct
  type t = { reveal : Reveal_request.t option } [@@deriving equal, sexp_of]

  let create ?reveal () = { reveal }
  let default = create ()
end

module Menu = struct
  type t =
    { label : string
    ; style : Style.t
    ; appearance : Choice.Appearance.t
    ; icons : (Choice.Id.t * Icon.Decoration.t) list
    }
  [@@deriving equal, sexp_of]

  let create
        ?(label = "All tabs")
        ?(style = Style.empty)
        ?(appearance = Choice.Appearance.default)
        ?(icons = [])
        ()
    =
    let open Or_error.Let_syntax in
    let%bind _ =
      Choice.Config.create
        ~label
        ~options:(Choice.Collection.create [] |> Or_error.ok_exn)
        ~selected:None
        ()
    in
    let%bind () =
      if List.length icons > Choice.Collection.max_choices
      then Or_error.error_string "too many tab menu icons"
      else Ok ()
    in
    let%bind _ =
      Map.of_alist_or_error
        (module String)
        (List.map icons ~f:(fun (id, icon) -> Choice.Id.to_string id, icon))
    in
    let%map () =
      Or_error.all_unit
        (List.map icons ~f:(fun (_, icon) ->
           let _, style = Icon.Expert.decoration icon in
           Style.Expert.validate_control_label style))
    in
    { label; style; appearance; icons }
  ;;

  let default = create () |> Or_error.ok_exn
end

module Expert = struct
  let motion_to_wire (t : Motion.t) = t
  let menu (t : Menu.t) = t.label, t.style, t.appearance
  let menu_icons (t : Menu.t) = t.icons

  let viewport_to_wire (t : Viewport.t) : Gpuio_protocol.Wire.Tab_viewport.t =
    { reveal =
        Option.map t.reveal ~f:(fun { Reveal_request.serial; target } ->
          { Gpuio_protocol.Wire.Tab_viewport.Reveal.serial
          ; target = Choice.Id.to_string target
          })
    }
  ;;

  let to_wire (t : Appearance.t) ~theme =
    let open Or_error.Let_syntax in
    let%bind tab_style = Style.Expert.to_wire t.tab_style ~theme in
    let%map item_styles =
      List.map t.item_styles ~f:(fun (id, style) ->
        let%map style = Style.Expert.to_wire style ~theme in
        Choice.Id.to_string id, style)
      |> Or_error.all
    in
    ({ variant = t.variant
     ; height = t.height
     ; gap = t.gap
     ; padding = t.padding
     ; tab_style
     ; item_styles
     }
     : W.t)
  ;;
end
