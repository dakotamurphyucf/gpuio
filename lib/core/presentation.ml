open Core
open Style.Property

module Size = struct
  type t =
    | Small
    | Medium
    | Large
  [@@deriving equal, sexp_of]
end

module Tone = struct
  type t =
    | Neutral
    | Accent
    | Success
    | Warning
    | Danger
  [@@deriving equal, sexp_of]
end

module Variant = struct
  type t =
    | Soft
    | Outline
    | Solid
  [@@deriving equal, sexp_of]
end

module Axis = struct
  type t =
    | Horizontal
    | Vertical
  [@@deriving equal, sexp_of]
end

module Appearance = struct
  type t =
    { surface : Color.t
    ; raised : Color.t
    ; foreground : Color.t
    ; muted : Color.t
    ; border : Color.t
    ; on_solid : Color.t
    ; accent : Color.t
    ; success : Color.t
    ; warning : Color.t
    ; danger : Color.t
    ; text_shimmer : Text_shimmer.Config.t
    }

  let create
        ~surface
        ~raised
        ~foreground
        ~muted
        ~border
        ~on_solid
        ~accent
        ~success
        ~warning
        ~danger
    =
    { surface
    ; raised
    ; foreground
    ; muted
    ; border
    ; on_solid
    ; accent
    ; success
    ; warning
    ; danger
    ; text_shimmer = Text_shimmer.Config.default
    }
  ;;

  let with_text_shimmer t text_shimmer = { t with text_shimmer }

  let themed_shimmer t ~dark =
    let appearance =
      Text_shimmer.Appearance.create
        ~dark
        ~foreground:t.foreground
        ~background:t.surface
        ()
      |> Or_error.ok_exn
    in
    (* Dark titles inherit [foreground]. Mixing toward that same color leaves no
       visible sweep, so the built-in dark palette uses a brighter highlight. *)
    with_text_shimmer
      t
      (Text_shimmer.Config.create
         ~appearance
         ?highlight:(Option.some_if dark (Color.rgb_exn 0xffffff))
         ()
       |> Or_error.ok_exn)
  ;;

  let light =
    let c = Color.rgb_exn in
    create
      ~surface:(c 0xffffff)
      ~raised:(c 0xf0f2f6)
      ~foreground:(c 0x202735)
      ~muted:(c 0x606a79)
      ~border:(c 0xd3d9e3)
      ~on_solid:(c 0xffffff)
      ~accent:(c 0x4058b7)
      ~success:(c 0x21734c)
      ~warning:(c 0x87550b)
      ~danger:(c 0xb7344b)
    |> fun t -> themed_shimmer t ~dark:false
  ;;

  let dark =
    let c = Color.rgb_exn in
    create
      ~surface:(c 0x1b202b)
      ~raised:(c 0x272e3b)
      ~foreground:(c 0xe5eaf2)
      ~muted:(c 0xa3aebe)
      ~border:(c 0x3e485b)
      ~on_solid:(c 0x161b24)
      ~accent:(c 0xa3b5ff)
      ~success:(c 0x8bd6af)
      ~warning:(c 0xf1c784)
      ~danger:(c 0xffa0af)
    |> fun t -> themed_shimmer t ~dark:true
  ;;
end

let internal_key = Key.of_string_exn
let px = Length.px_exn
let full = Length.percent_exn 100.
let style = Style.create_exn
let solid = Background.solid

let semantic ?live role view =
  let metadata = Accessibility.create ~role ?live () |> Or_error.ok_exn in
  View.with_accessibility view metadata |> Or_error.ok_exn
;;

let color (p : Appearance.t) = function
  | Tone.Neutral -> p.foreground
  | Accent -> p.accent
  | Success -> p.success
  | Warning -> p.warning
  | Danger -> p.danger
;;

let slot name content =
  View.column
    ~key:(internal_key name)
    ~style:(style [ Min_width (px 0.); Shrink 0. ])
    [ content ]
;;

let optional_slot name content = Option.map content ~f:(slot name)
let label ?key ?style text = View.text ?key ?style text |> semantic Label

let styled_label (appearance : Appearance.t) ?key ?(style = Style.empty) config =
  let content =
    Label.Expert.to_text_content
      config
      ~secondary:appearance.muted
      ~highlight:appearance.accent
  in
  View.styled_text
    ?key
    ~style:(Style.merge [ Style.create_exn [ Foreground appearance.foreground ]; style ])
    content
;;

let tag
      (p : Appearance.t)
      ?key
      ?style:(custom = Style.empty)
      ?(size = Size.Medium)
      ?(tone = Tone.Neutral)
      ?(variant = Variant.Soft)
      ?leading
      ?trailing
      text
  =
  let font, padding =
    match size with
    | Small -> 11., 4.
    | Medium -> 12., 6.
    | Large -> 14., 8.
  in
  let ink = color p tone in
  let treatment =
    match variant with
    | Soft -> [ Background (solid p.raised); Foreground ink ]
    | Outline -> [ Foreground ink; Border_width 1.; Border_color ink ]
    | Solid -> [ Background (solid ink); Foreground p.on_solid ]
  in
  View.row
    ?key
    ~style:
      (Style.merge
         [ style
             ([ Min_width (px 0.)
              ; Align_items Center
              ; Gap (px 5.)
              ; Padding_top (px (padding /. 2.))
              ; Padding_bottom (px (padding /. 2.))
              ; Padding_left (px padding)
              ; Padding_right (px padding)
              ; Radius 6.
              ; Font_size font
              ; Font_weight 500
              ]
              @ treatment)
         ; custom
         ])
    (List.filter_opt
       [ optional_slot "leading" leading
       ; Some
           (View.text
              ~key:(internal_key "label")
              ~style:(style [ Min_width (px 0.) ])
              text)
       ; optional_slot "trailing" trailing
       ])
;;

module Tag = struct
  module Size = struct
    type t =
      | XSmall
      | Small
      | Medium
      | Large
    [@@deriving equal, sexp_of]
  end

  module Palette = struct
    type t =
      { background : Color.t
      ; foreground : Color.t
      ; border : Color.t
      }
    [@@deriving equal, sexp_of]

    let create ~background ~foreground ~border = { background; foreground; border }
  end

  module Variant = struct
    type t =
      | Primary
      | Secondary
      | Danger
      | Success
      | Warning
      | Info
      | Custom of Palette.t
    [@@deriving equal, sexp_of]
  end

  let palette (p : Appearance.t) variant ~outline =
    let semantic ink =
      Palette.create
        ~background:ink
        ~foreground:(if outline then ink else p.on_solid)
        ~border:ink
    in
    match variant with
    | Variant.Primary | Info -> semantic p.accent
    | Danger -> semantic p.danger
    | Success -> semantic p.success
    | Warning -> semantic p.warning
    | Secondary ->
      Palette.create
        ~background:p.raised
        ~foreground:(if outline then p.muted else p.foreground)
        ~border:p.border
    | Custom t -> t
  ;;

  let create
        (p : Appearance.t)
        ?key
        ?style:(custom = Style.empty)
        ?(size = Size.Medium)
        ?(variant = Variant.Secondary)
        ?(outline = false)
        children
    =
    let padding_x, padding_y, radius =
      match size with
      | XSmall | Small -> 6., 2., 4.
      | Medium | Large -> 10., 4., 8.
    in
    let palette = palette p variant ~outline in
    let background =
      if outline
      then Color.rgba ~red:255 ~green:255 ~blue:255 ~alpha:0 |> Or_error.ok_exn
      else palette.background
    in
    let base =
      style
        [ Align_items Center
        ; Min_width (px 0.)
        ; Border_width 1.
        ; Radius radius
        ; Padding_left (px padding_x)
        ; Padding_right (px padding_x)
        ; Padding_top (px padding_y)
        ; Padding_bottom (px padding_y)
        ; Font_size 12.
        ; Line_height (Length.percent_exn 125.)
        ; Background (solid background)
        ; Foreground palette.foreground
        ; Border_color palette.border
        ]
      |> fun s -> Style.with_state_exn s Hovered [ Opacity 0.9 ]
    in
    View.row ?key ~style:(Style.merge [ base; custom ]) children
  ;;
end

let badge
      (p : Appearance.t)
      ?key
      ?style:(custom = Style.empty)
      ?size
      ?tone
      ?variant
      ?leading
      text
  =
  tag
    p
    ?key
    ~style:(Style.merge [ style [ Radius 999. ]; custom ])
    ?size
    ?tone
    ?variant
    ?leading
    text
;;

module Overlay_badge = struct
  type t =
    | Count of
        { value : int
        ; max : int
        ; accessibility : Accessibility.t
        }
    | Dot of Accessibility.t
    | Icon of Icon.Config.t

  let accessibility label =
    if String.is_empty (String.strip label)
    then Or_error.error_string "overlay badge label must not be blank"
    else Accessibility.create ~role:Image ~label ()
  ;;

  let count ?(max = 99) ~label value =
    if value < 0 || max < 0
    then Or_error.error_string "overlay badge count and maximum must be nonnegative"
    else
      let open Or_error.Let_syntax in
      let%map accessibility = accessibility label in
      Count { value; max; accessibility }
  ;;

  let dot ~label = Or_error.map (accessibility label) ~f:(fun metadata -> Dot metadata)
  let icon config = Icon config
end

let overlay_badge
      (p : Appearance.t)
      ?key
      ?style:(custom = Style.empty)
      ?(badge_style = Style.empty)
      ?(size = Size.Medium)
      ?(tone = Tone.Danger)
      ~badge
      content
  =
  let diameter, font, dot_size =
    match size with
    | Small -> 16., 10., 6.
    | Medium -> 20., 11., 8.
    | Large -> 24., 12., 10.
  in
  let visual =
    match badge with
    | Overlay_badge.Count { value = 0; _ } -> None
    | Count { value; max; accessibility } ->
      let text = if value > max then sprintf "%d+" max else Int.to_string value in
      Some
        ( [ Top (px 0.)
          ; Min_width (px diameter)
          ; Height (px diameter)
          ; Padding_left (px 4.)
          ; Padding_right (px 4.)
          ]
        , Some accessibility
        , Some text
        , [] )
    | Dot accessibility ->
      Some
        ( [ Top (px 0.); Width (px dot_size); Height (px dot_size) ]
        , Some accessibility
        , None
        , [] )
    | Icon config ->
      Some
        ( [ Bottom (px 0.)
          ; Width (px diameter)
          ; Height (px diameter)
          ; Border_width 1.
          ; Border_color p.surface
          ; Padding (px 2.)
          ]
        , None
        , None
        , [ View.icon ~style:(style [ Width full; Height full ]) config ] )
  in
  let overlay =
    Option.map visual ~f:(fun (geometry, metadata, text, children) ->
      let overlay_style =
        Style.merge
          [ style
              ([ Display Flex
               ; Position Absolute
               ; Right (px 0.)
               ; Radius 999.
               ; Align_items Center
               ; Justify_content Center
               ; Background (solid (color p tone))
               ; Foreground p.on_solid
               ; Font_size font
               ; Font_weight 600
               ; White_space No_wrap
               ]
               @ geometry)
          ; badge_style
          ; style [ Pointer_events false; Pointer_occlusion None; User_select false ]
          ]
      in
      let key = internal_key "badge" in
      let view =
        match text with
        | Some text -> View.text ~key ~style:overlay_style text
        | None -> View.row ~key ~style:overlay_style children
      in
      match metadata with
      | None -> view
      | Some metadata -> View.with_accessibility view metadata |> Or_error.ok_exn)
  in
  View.column
    ?key
    ~style:(Style.merge [ style [ Position Relative; Min_width (px 0.) ]; custom ])
    (slot "content" content :: Option.to_list overlay)
;;

let marker
      (p : Appearance.t)
      ?key
      ?style:(custom = Style.empty)
      ?(tone = Tone.Neutral)
      text
  =
  View.row
    ?key
    ~style:
      (Style.merge
         [ style
             [ Align_items Center
             ; Gap (px 6.)
             ; Min_width (px 0.)
             ; Foreground p.Appearance.foreground
             ]
         ; custom
         ])
    [ View.column
        ~key:(internal_key "dot")
        ~style:
          (style
             [ Width (px 6.)
             ; Height (px 6.)
             ; Shrink 0.
             ; Radius 3.
             ; Background (solid (color p tone))
             ])
        []
    ; View.text ~key:(internal_key "label") ~style:(style [ Min_width (px 0.) ]) text
    ]
;;

module Marker = struct
  module Variant = struct
    type t =
      | Plain
      | Separator
      | Border
    [@@deriving equal, sexp_of]
  end

  module Loading_style = struct
    type t =
      | Spinner
      | Shimmer
    [@@deriving equal, sexp_of]
  end

  module Spinner = struct
    type t = Loading.Config.t

    let create ?(label = "Loading") ?animated ?period () =
      Loading.Config.create ~kind:Spinner ~label ?animated ?period ()
    ;;

    let default = create () |> Or_error.ok_exn
  end

  let validate_keys keys =
    match List.find_a_dup keys ~compare:Key.compare with
    | None -> Ok ()
    | Some key -> Or_error.errorf "duplicate marker item key: %s" (Key.to_string key)
  ;;

  module Icon = struct
    type 'action t =
      { key : Key.t
      ; style : Style.t
      ; children : 'action View.t list
      }

    let create ~key ?(style = Style.empty) children = { key; style; children }

    let view t =
      View.row
        ~key:t.key
        ~style:
          (Style.merge
             [ style
                 [ Width (px 16.)
                 ; Height (px 16.)
                 ; Shrink 0.
                 ; Grow 0.
                 ; Align_items Center
                 ; Justify_content Center
                 ]
             ; t.style
             ])
        t.children
    ;;
  end

  module Content = struct
    module Item = struct
      type 'action t =
        | Text of Key.t * Style.t * string
        | Element of Key.t * 'action View.t

      let text ~key ?(style = Style.empty) text =
        if
          String.length text > Gpuio_protocol.Text_shimmer_wire.max_text_bytes
          || not (Stdlib.String.is_valid_utf_8 text)
        then
          Or_error.error_string "marker text must be valid UTF-8 of at most 16384 bytes"
        else Ok (Text (key, style, text))
      ;;

      let element ~key view = Element (key, view)

      let key = function
        | Text (key, _, _) | Element (key, _) -> key
      ;;

      let is_text = function
        | Text _ -> true
        | Element _ -> false
      ;;

      let view t ~shimmer =
        match t with
        | Text (key, style, text) ->
          View.text ~key ~style text
          |> fun view -> View.with_text_shimmer view shimmer |> Or_error.ok_exn
        | Element (key, view) -> View.with_key view key
      ;;
    end

    type 'action t =
      { key : Key.t
      ; style : Style.t
      ; items : 'action Item.t list
      }

    let create ~key ?(style = Style.empty) items =
      Or_error.map
        (validate_keys (List.map items ~f:Item.key))
        ~f:(fun () -> { key; style; items })
    ;;

    let factor value =
      Animation.Target.create [ Opacity_factor, value ] |> Or_error.ok_exn
    ;;

    let stage milliseconds value =
      Animation.Stage.create
        ~timing:
          (Animation.Timing.tween
             ~easing:Animation.Easing.ease_in_out
             (Time_ns.Span.of_ms (Float.of_int milliseconds))
           |> Or_error.ok_exn)
        ~target:(factor value)
        ()
      |> Or_error.ok_exn
    ;;

    let still =
      Animation.Program.create ~initial:(factor 1.) [ stage 0 1. ] |> Or_error.ok_exn
    ;;

    let pulse shimmer =
      let config = Text_shimmer.Expert.to_wire shimmer in
      if not config.animated
      then still
      else (
        let first = config.duration_ms / 2 in
        let repeat =
          match config.repeat with
          | Once -> Animation.Repeat.Once
          | Loop -> Loop
        in
        Animation.Program.create
          ~initial:(factor 1.)
          ~repeat
          [ stage first 0.6; stage (config.duration_ms - first) 1. ]
        |> Or_error.ok_exn)
    ;;

    let view t ~variant ~loading ~shimmer =
      let program =
        if loading && not (List.exists t.items ~f:Item.is_text)
        then pulse shimmer
        else still
      in
      let separator = Variant.equal variant Separator in
      View.animate_program
        ~key:t.key
        ~style:
          (Style.merge
             [ style
                 ([ Display Flex; Direction Row; Min_width (px 0.) ]
                  @ if separator then [ Grow 0.; Shrink 0.; Text_align Center ] else [])
             ; t.style
             ])
        program
        (List.map t.items ~f:(fun item ->
           Item.view item ~shimmer:(Option.some_if loading shimmer)))
    ;;
  end

  module Item = struct
    type 'action t =
      | Icon of 'action Icon.t
      | Content of 'action Content.t
      | Element of Key.t * 'action View.t

    let icon t = Icon t
    let content t = Content t
    let element ~key view = Element (key, view)

    let key = function
      | Icon t -> t.Icon.key
      | Content t -> t.Content.key
      | Element (key, _) -> key
    ;;

    let is_icon = function
      | Icon _ -> true
      | Content _ | Element _ -> false
    ;;

    let view t ~variant ~loading ~shimmer =
      match t with
      | Icon t -> Icon.view t
      | Content t -> Content.view t ~variant ~loading ~shimmer
      | Element (key, view) -> View.with_key view key
    ;;
  end

  let create
        (p : Appearance.t)
        ?key
        ?style:(custom = Style.empty)
        ?(separator_style = Style.empty)
        ?(variant = Variant.Plain)
        ?(loading = false)
        ?(loading_style = Loading_style.Spinner)
        ?(spinner = Spinner.default)
        ?shimmer
        items
    =
    let open Or_error.Let_syntax in
    let keys = List.map items ~f:Item.key in
    let%bind () = validate_keys keys in
    let%bind () =
      if
        List.exists keys ~f:(fun key ->
          String.is_prefix (Key.to_string key) ~prefix:"gpuio:marker:")
      then
        Or_error.error_string
          "marker item keys must not use the reserved gpuio:marker: prefix"
      else Ok ()
    in
    let shimmer = Option.value shimmer ~default:p.text_shimmer in
    let is_separator = Variant.equal variant Separator in
    let line name margin =
      View.row
        ~key:(internal_key name)
        ~style:
          (Style.merge
             [ style
                 [ Grow 1.
                 ; Basis (px 0.)
                 ; Min_width (px 0.)
                 ; Height (px 1.)
                 ; margin
                 ; Background (solid p.border)
                 ]
             ; separator_style
             ])
        []
    in
    let automatic_spinner =
      if
        loading
        && Loading_style.equal loading_style Spinner
        && not (List.exists items ~f:Item.is_icon)
      then
        [ Icon.view
            (Icon.create
               ~key:(internal_key "gpuio:marker:spinner")
               [ View.loading
                   ~key:(internal_key "spinner")
                   ~style:(style [ Width (px 16.); Height (px 16.); Foreground p.muted ])
                   ~config:spinner
                   ()
               ])
        ]
      else []
    in
    let children =
      (if is_separator then [ line "gpuio:marker:before" (Margin_right (px 4.)) ] else [])
      @ automatic_spinner
      @ List.map items ~f:(fun item ->
        Item.view
          item
          ~variant
          ~loading:(loading && Loading_style.equal loading_style Shimmer)
          ~shimmer)
      @ if is_separator then [ line "gpuio:marker:after" (Margin_left (px 4.)) ] else []
    in
    let variant_style =
      match variant with
      | Plain -> []
      | Separator -> [ Justify_content Center ]
      | Border ->
        [ Border_bottom_width 1.; Border_color p.border; Padding_bottom (px 8.) ]
    in
    Ok
      (View.row
         ?key
         ~style:
           (Style.merge
              [ style
                  ([ Width full
                   ; Min_height (px 16.)
                   ; Min_width (px 0.)
                   ; Align_items Center
                   ; Gap (px 8.)
                   ; Font_size 14.
                   ; Line_height (Length.percent_exn 150.)
                   ; Foreground p.muted
                   ; Text_align Left
                   ]
                   @ variant_style)
              ; custom
              ])
         children)
  ;;
end

let link (p : Appearance.t) ?key ?style:(custom = Style.empty) ?disabled ~on_click text =
  let transparent = Color.rgba ~red:0 ~green:0 ~blue:0 ~alpha:0 |> Or_error.ok_exn in
  let base =
    style
      [ Foreground p.Appearance.accent
      ; Background (solid transparent)
      ; Padding (px 0.)
      ; Text_decoration Underline
      ; Cursor Pointer
      ]
    |> fun s -> Style.with_state_exn s Disabled [ Foreground p.muted; Cursor Not_allowed ]
  in
  View.button ?key ~style:(Style.merge [ base; custom ]) ?disabled ~on_click text
  |> semantic Link
;;

let composed_link
      (p : Appearance.t)
      ?key
      ?style:(custom = Style.empty)
      config
      ~on_click
      children
  =
  let base =
    style [ Foreground p.accent; Text_decoration Underline; Cursor Pointer ]
    |> fun s -> Style.with_state_exn s Disabled [ Foreground p.muted; Cursor Not_allowed ]
  in
  View.link ?key ~style:(Style.merge [ base; custom ]) config ~on_click children
;;

let separator
      (p : Appearance.t)
      ?key
      ?style:(custom = Style.empty)
      ?(axis = Axis.Horizontal)
      ()
  =
  let dimensions =
    match axis with
    | Horizontal -> [ Width full; Height (px 1.) ]
    | Vertical -> [ Width (px 1.); Align_self Stretch ]
  in
  View.column
    ?key
    ~style:
      (Style.merge
         [ style (Background (solid p.Appearance.border) :: Shrink 0. :: dimensions)
         ; custom
         ])
    []
  |> semantic Separator
;;

module Separator = struct
  let create
        (p : Appearance.t)
        ?key
        ?style:(custom = Style.empty)
        ?(line_style = Style.empty)
        ?(label_style = Style.empty)
        ?(axis = Axis.Horizontal)
        ?(pattern = Style.Border_style.Solid)
        ?color
        ?label
        ()
    =
    let dimensions, line_dimensions =
      match axis with
      | Horizontal ->
        ( Width full :: (if Option.is_none label then [ Height (px 1.) ] else [])
        , [ Width full; Height (px 1.); Border_top_width 1. ] )
      | Vertical ->
        ( Height full :: (if Option.is_none label then [ Width (px 1.) ] else [])
        , [ Width (px 1.); Height full; Border_left_width 1. ] )
    in
    let line =
      (* Auto insets use the parent's flex centering on both axes. A 50% offset
         without compensating for the edge width clips a one-pixel plain root. *)
      View.column
        ~key:(internal_key "line")
        ~style:
          (Style.merge
             [ style
                 ([ Position Absolute
                  ; Border_style pattern
                  ; Border_color (Option.value color ~default:p.border)
                  ]
                  @ line_dimensions)
             ; line_style
             ])
        []
    in
    let label =
      Option.map label ~f:(fun text ->
        View.text
          ~key:(internal_key "label")
          ~style:
            (Style.merge
               [ style
                   [ Min_width (px 0.)
                   ; Max_width full
                   ; Padding_left (px 8.)
                   ; Padding_right (px 8.)
                   ; Padding_top (px 4.)
                   ; Padding_bottom (px 4.)
                   ; Font_size 12.
                   ; Foreground p.muted
                   ; Background (solid p.surface)
                   ; White_space Normal
                   ; Text_align Center
                   ]
               ; label_style
               ])
          text)
    in
    View.column
      ?key
      ~style:
        (Style.merge
           [ style
               ([ Position Relative
                ; Min_width (px 0.)
                ; Min_height (px 0.)
                ; Shrink 0.
                ; Align_items Center
                ; Justify_content Center
                ; Overflow_x Hidden
                ; Overflow_y Hidden
                ]
                @ dimensions)
           ; custom
           ])
      (line :: Option.to_list label)
    |> semantic Separator
  ;;
end

module Group_variant = struct
  type t =
    | Card
    | Plain
    | Filled
    | Outline
  [@@deriving equal, sexp_of]
end

let group_box
      (p : Appearance.t)
      ?key
      ?style:(custom = Style.empty)
      ?(variant = Group_variant.Card)
      ?(header_style = Style.empty)
      ?(body_style = Style.empty)
      ?(footer_style = Style.empty)
      ?header
      ?footer
      children
  =
  let root, body =
    match variant with
    | Group_variant.Card ->
      ( [ Padding (px 16.)
        ; Radius 12.
        ; Border_width 1.
        ; Border_color p.Appearance.border
        ; Background (solid p.surface)
        ]
      , [] )
    | Plain -> [], []
    | Filled -> [], [ Padding (px 16.); Radius 12.; Background (solid p.raised) ]
    | Outline ->
      [], [ Padding (px 16.); Radius 12.; Border_width 1.; Border_color p.border ]
  in
  let styled_slot name custom =
    Option.map ~f:(fun child ->
      View.column
        ~key:(internal_key name)
        ~style:(Style.merge [ style [ Min_width (px 0.); Shrink 0. ]; custom ])
        [ child ])
  in
  View.column
    ?key
    ~style:
      (Style.merge
         [ style ([ Min_width (px 0.); Gap (px 12.); Foreground p.foreground ] @ root)
         ; custom
         ])
    (List.filter_opt
       [ styled_slot "header" header_style header
       ; Some
           (View.column
              ~key:(internal_key "body")
              ~style:
                (Style.merge
                   [ style ([ Min_width (px 0.); Gap (px 12.) ] @ body); body_style ])
              children)
       ; styled_slot "footer" footer_style footer
       ])
  |> semantic Group
;;

let secondary (p : Appearance.t) name text =
  Option.map text ~f:(fun text ->
    View.text
      ~key:(internal_key name)
      ~style:(style [ Foreground p.Appearance.muted; Font_size 12.; Min_width (px 0.) ])
      text)
;;

let settings_group (p : Appearance.t) ?key ?style ~title ?description children =
  let header =
    View.column
      ~style:(Style.create_exn [ Gap (px 4.) ])
      (View.text
         ~key:(internal_key "title")
         ~style:(Style.create_exn [ Font_weight 600 ])
         title
       :: Option.to_list (secondary p "description" description))
  in
  group_box p ?key ?style ~header children
;;

module Description = struct
  type 'action t =
    { key : Key.t
    ; term : string
    ; definition : 'action View.t
    }

  let create ~key ~term ~definition = { key; term; definition }
end

let description_list
      (p : Appearance.t)
      ?key
      ?style:(custom = Style.empty)
      ?(stacked = false)
      entries
  =
  let entry { Description.key; term; definition } =
    let term =
      View.text
        ~key:(internal_key "term")
        ~style:
          (style
             [ Foreground p.Appearance.muted; Font_size 12.; Grow 1.; Min_width (px 0.) ])
        term
      |> semantic Term
    in
    let definition =
      View.column
        ~key:(internal_key "definition")
        ~style:(style [ Grow 2.; Min_width (px 0.) ])
        [ definition ]
      |> semantic Definition
    in
    let compose = if stacked then View.column else View.row in
    compose
      ~key
      ~style:(style [ Gap (px (if stacked then 4. else 16.)); Min_width (px 0.) ])
      [ term; definition ]
  in
  View.column
    ?key
    ~style:
      (Style.merge [ style [ Gap (px 12.); Foreground p.Appearance.foreground ]; custom ])
    (List.map entries ~f:entry)
  |> semantic Description_list
;;

let empty_state
      (p : Appearance.t)
      ?key
      ?style:(custom = Style.empty)
      ?icon
      ~title
      ?description
      ?actions
      ()
  =
  View.column
    ?key
    ~style:
      (Style.merge
         [ style
             [ Padding (px 32.)
             ; Gap (px 12.)
             ; Align_items Center
             ; Text_align Center
             ; Foreground p.Appearance.foreground
             ; Min_width (px 0.)
             ]
         ; custom
         ])
    (List.filter_opt
       [ optional_slot "icon" icon
       ; Some
           (View.text
              ~key:(internal_key "title")
              ~style:(style [ Font_size 18.; Font_weight 600 ])
              title)
       ; secondary p "description" description
       ; optional_slot "actions" actions
       ])
  |> semantic Group
;;

module Empty_state = struct
  module Media_variant = struct
    type t =
      | Unframed
      | Icon
    [@@deriving equal, sexp_of]
  end

  let media
        (p : Appearance.t)
        ?key
        ?style:(custom = Style.empty)
        ?(variant = Media_variant.Unframed)
        children
    =
    let frame =
      match variant with
      | Unframed -> []
      | Icon ->
        [ Width (px 32.)
        ; Height (px 32.)
        ; Radius 8.
        ; Background (solid p.raised)
        ; Foreground p.foreground
        ; Font_size 16.
        ]
    in
    View.column
      ?key
      ~style:
        (Style.merge
           [ style
               ([ Shrink 0.
                ; Align_items Center
                ; Justify_content Center
                ; Margin_bottom (px 8.)
                ]
                @ frame)
           ; custom
           ])
      children
  ;;

  let title ?key ?style:(custom = Style.empty) children =
    View.column
      ?key
      ~style:
        (Style.merge
           [ style
               [ Min_width (px 0.)
               ; Max_width full
               ; Font_size 14.
               ; Font_weight 500
               ; White_space Normal
               ]
           ; custom
           ])
      children
  ;;

  let description (p : Appearance.t) ?key ?style:(custom = Style.empty) children =
    View.column
      ?key
      ~style:
        (Style.merge
           [ style
               [ Width full
               ; Min_width (px 0.)
               ; Font_size 14.
               ; Line_height (Length.percent_exn 162.5)
               ; Foreground p.muted
               ; White_space Normal
               ]
           ; custom
           ])
      children
  ;;

  let column_style gap =
    style
      [ Width full
      ; Max_width (px 384.)
      ; Min_width (px 0.)
      ; Align_items Center
      ; Gap (px gap)
      ]
  ;;

  let header ?key ?style:(custom = Style.empty) ?media ?title ?description () =
    View.column
      ?key
      ~style:(Style.merge [ column_style 8.; custom ])
      (List.filter_opt
         [ optional_slot "media" media
         ; optional_slot "title" title
         ; optional_slot "description" description
         ])
  ;;

  let content ?key ?style:(custom = Style.empty) children =
    View.column
      ?key
      ~style:(Style.merge [ column_style 10.; style [ Font_size 14. ]; custom ])
      children
  ;;

  let create
        (p : Appearance.t)
        ?key
        ?style:(custom = Style.empty)
        ?(children_style = Style.empty)
        ?header
        ?content
        children
    =
    let extras =
      if List.is_empty children
      then None
      else
        Some
          (View.column
             ~key:(internal_key "extra")
             ~style:
               (Style.merge
                  [ style [ Min_width (px 0.); Gap (px 16.); Align_items Center ]
                  ; children_style
                  ])
             children)
    in
    View.column
      ?key
      ~style:
        (Style.merge
           [ style
               [ Width full
               ; Min_width (px 0.)
               ; Grow 1.
               ; Align_items Center
               ; Justify_content Center
               ; Gap (px 16.)
               ; Padding (px 24.)
               ; Radius 12.
               ; Border_style Dashed
               ; Border_color p.border
               ; Text_align Center
               ; Foreground p.foreground
               ]
           ; custom
           ])
      (List.filter_opt
         [ optional_slot "header" header; optional_slot "content" content; extras ])
    |> semantic Group
  ;;
end

let alert
      (p : Appearance.t)
      ?key
      ?style:(custom = Style.empty)
      ?(tone = Tone.Neutral)
      ?(live = Accessibility.Live.Polite)
      ?icon
      ~title
      ?actions
      children
  =
  let heading =
    View.row
      ~style:(style [ Align_items Center; Gap (px 8.); Min_width (px 0.) ])
      (List.filter_opt
         [ optional_slot "icon" icon
         ; Some
             (View.text
                ~key:(internal_key "title")
                ~style:(style [ Font_weight 600; Grow 1.; Min_width (px 0.) ])
                title)
         ; optional_slot "actions" actions
         ])
  in
  group_box
    p
    ?key
    ~style:(Style.merge [ style [ Border_color (color p tone) ]; custom ])
    ~header:heading
    children
  |> semantic ~live Alert
;;

let banner
      (p : Appearance.t)
      ?key
      ?style:(custom = Style.empty)
      ?tone
      ?live
      ?icon
      ~title
      ?actions
      children
  =
  alert
    p
    ?key
    ~style:(Style.merge [ style [ Radius 0.; Width full ]; custom ])
    ?tone
    ?live
    ?icon
    ~title
    ?actions
    children
;;

module Alert = struct
  module Variant = struct
    type t =
      | Default
      | Info
      | Success
      | Warning
      | Error
    [@@deriving equal, sexp_of]
  end

  module Size = struct
    type t =
      | XSmall
      | Small
      | Medium
      | Large
    [@@deriving equal, sexp_of]
  end

  module Layout = struct
    type t =
      | Card
      | Banner
    [@@deriving equal, sexp_of]
  end

  module Icon = struct
    type 'action t =
      | Default
      | Hidden
      | Custom of 'action View.t
  end

  module Close = struct
    type 'action t =
      { label : string
      ; style : Style.t
      ; disabled : bool
      ; on_click : unit -> 'action
      }

    let create ~label ?(style = Style.empty) ?(disabled = false) ~on_click () =
      if
        String.is_empty (String.strip label)
        || String.length label > 1024
        || String.contains label '\000'
        || not (Stdlib.String.is_valid_utf_8 label)
      then
        Or_error.error_string
          "alert close label must be nonblank UTF-8 without NUL, at most 1024 bytes"
      else Ok { label; style; disabled; on_click }
    ;;

    let view t (p : Appearance.t) =
      let base =
        style
          [ Shrink 0.
          ; Padding (px 2.)
          ; Radius 6.
          ; Font_size 20.
          ; Background
              (solid (Color.rgba ~red:0 ~green:0 ~blue:0 ~alpha:0 |> Or_error.ok_exn))
          ]
        |> fun s -> Style.with_state_exn s Hovered [ Background (solid p.raised) ]
      in
      View.button
        ~key:(internal_key "close")
        ~accessible_name:t.label
        ~style:(Style.merge [ base; t.style ])
        ~disabled:t.disabled
        ~on_click:t.on_click
        "×"
    ;;
  end

  let title ?key ?style:(custom = Style.empty) text =
    View.text
      ?key
      ~style:
        (Style.merge
           [ style
               [ Width full
               ; Min_width (px 0.)
               ; Font_weight 600
               ; White_space No_wrap
               ; Text_overflow Ellipsis
               ; Overflow Hidden
               ]
           ; custom
           ])
      text
  ;;

  let colors (p : Appearance.t) = function
    | Variant.Default -> p.foreground, p.surface, p.border
    | (Info | Success | Warning | Error) as variant ->
      let ink =
        match variant with
        | Default -> p.foreground
        | Info -> p.accent
        | Success -> p.success
        | Warning -> p.warning
        | Error -> p.danger
      in
      ( ink
      , Color.with_opacity ink 0.04 |> Or_error.ok_exn
      , Color.with_opacity ink 0.3 |> Or_error.ok_exn )
  ;;

  let default_icon = function
    | Variant.Default | Info -> "ⓘ"
    | Success -> "✓"
    | Warning -> "!"
    | Error -> "×"
  ;;

  let create
        (p : Appearance.t)
        ?key
        ?style:(custom = Style.empty)
        ?(title_style = Style.empty)
        ?(body_style = Style.empty)
        ?(icon_style = Style.empty)
        ?(variant = Variant.Default)
        ?(size = Size.Medium)
        ?(layout = Layout.Card)
        ?(icon = Icon.Default)
        ?title
        ?close
        ?(live = Accessibility.Live.Off)
        ?(visible = true)
        children
    =
    if not visible
    then View.row ?key ~style:(style [ Display Hidden ]) []
    else (
      let padding_x, padding_y, gap, radius =
        match size with
        | XSmall -> 12., 6., 6., 8.
        | Small -> 12., 8., 6., 8.
        | Medium -> 16., 10., 12., 8.
        | Large -> 20., 14., 12., 12.
      in
      let banner = Layout.equal layout Banner in
      let foreground, background, border = colors p variant in
      let icon =
        match icon with
        | Hidden -> None
        | Default ->
          Some (View.text ~style:(style [ Font_size 18. ]) (default_icon variant))
        | Custom view -> Some view
      in
      let icon =
        Option.map icon ~f:(fun view ->
          View.column
            ~key:(internal_key "icon")
            ~style:
              (Style.merge
                 [ style [ Shrink 0.; Margin_top (px (if banner then 0. else 5.)) ]
                 ; icon_style
                 ])
            [ view ])
      in
      let title =
        if banner
        then None
        else
          Option.map title ~f:(fun view ->
            View.column
              ~key:(internal_key "title")
              ~style:
                (Style.merge
                   [ style [ Width full; Min_width (px 0.); Font_weight 600 ]
                   ; title_style
                   ])
              [ view ])
      in
      let body =
        View.column
          ~key:(internal_key "body")
          ~style:(Style.merge [ style [ Min_width (px 0.); Gap (px 3.2) ]; body_style ])
          children
      in
      let content =
        View.column
          ~key:(internal_key "content")
          ~style:
            (style
               [ Grow 1.
               ; Basis (px 0.)
               ; Min_width (px 0.)
               ; Overflow Hidden
               ; Gap (px 12.)
               ])
          (List.filter_opt [ title; Some body ])
      in
      let main =
        View.row
          ~key:(internal_key "main")
          ~style:
            (style
               [ Grow 1.
               ; Basis (px 0.)
               ; Min_width (px 0.)
               ; Overflow Hidden
               ; Gap (px gap)
               ; Align_items (if banner then Center else Start)
               ])
          (List.filter_opt [ icon; Some content ])
      in
      View.row
        ?key
        ~style:
          (Style.merge
             [ style
                 [ Width full
                 ; Min_width (px 0.)
                 ; Gap (px gap)
                 ; Padding_left (px padding_x)
                 ; Padding_right (px padding_x)
                 ; Padding_top (px padding_y)
                 ; Padding_bottom (px padding_y)
                 ; Radius (if banner then 0. else radius)
                 ; Border_width 1.
                 ; Border_color border
                 ; Background (solid background)
                 ; Foreground foreground
                 ; Font_size 14.
                 ; Line_height (Length.percent_exn 150.)
                 ; Align_items (if banner then Center else Start)
                 ; Justify_content Space_between
                 ]
             ; custom
             ])
        (main :: Option.to_list (Option.map close ~f:(fun t -> Close.view t p)))
      |> semantic ~live Alert)
  ;;
end

let shortcut_label (p : Appearance.t) ?key ?style:(custom = Style.empty) names =
  View.row
    ?key
    ~style:(Style.merge [ style [ Gap (px 3.); Align_items Center ]; custom ])
    (List.mapi names ~f:(fun i name ->
       View.text
         ~key:(Key.of_int i)
         ~style:
           (style
              [ Font_size 11.
              ; Min_width (px 0.)
              ; Foreground p.Appearance.muted
              ; Background (solid p.raised)
              ; Border_width 1.
              ; Border_color p.border
              ; Radius 4.
              ; Padding_left (px 5.)
              ; Padding_right (px 5.)
              ; Padding_top (px 2.)
              ; Padding_bottom (px 2.)
              ])
         name))
;;

let status_bar
      (p : Appearance.t)
      ?key
      ?style:(custom = Style.empty)
      ?leading
      ?center
      ?trailing
      ()
  =
  View.row
    ?key
    ~style:
      (Style.merge
         [ style
             [ Min_width (px 0.)
             ; Align_items Center
             ; Gap (px 12.)
             ; Padding (px 8.)
             ; Border_top_width 1.
             ; Border_color p.Appearance.border
             ; Foreground p.muted
             ; Font_size 11.
             ]
         ; custom
         ])
    (List.filter_opt
       [ Option.map leading ~f:(fun content ->
           View.column
             ~key:(internal_key "leading")
             ~style:
               (style
                  [ Min_width (px 0.); Grow (if Option.is_none center then 1. else 0.) ])
             [ content ])
       ; Some
           (View.row
              ~key:(internal_key "center")
              ~style:
                (style
                   [ Min_width (px 0.)
                   ; Grow 1.
                   ; Basis (px 0.)
                   ; Align_items Center
                   ; Justify_content
                       (if Option.is_some leading
                        then if Option.is_some trailing then Center else End
                        else Start)
                   ])
              (Option.to_list center))
       ; optional_slot "trailing" trailing
       ])
;;

let attachment
      (p : Appearance.t)
      ?key
      ?style:(custom = Style.empty)
      ?preview
      ~name
      ?detail
      ?actions
      ()
  =
  View.row
    ?key
    ~style:
      (Style.merge
         [ style
             [ Min_width (px 0.)
             ; Align_items Center
             ; Gap (px 10.)
             ; Padding (px 10.)
             ; Radius 10.
             ; Border_width 1.
             ; Border_color p.Appearance.border
             ; Foreground p.foreground
             ; Background (solid p.surface)
             ]
         ; custom
         ])
    (List.filter_opt
       [ optional_slot "preview" preview
       ; Some
           (View.column
              ~key:(internal_key "body")
              ~style:(style [ Grow 1.; Min_width (px 0.); Gap (px 3.) ])
              (View.text
                 ~key:(internal_key "name")
                 ~style:(style [ Font_size 13.; Font_weight 500 ])
                 name
               :: Option.to_list (secondary p "detail" detail)))
       ; optional_slot "actions" actions
       ])
  |> semantic Group
;;

module Attachment = struct
  module Status = struct
    type t =
      | Pending
      | Uploading
      | Processing
      | Failed
      | Complete
    [@@deriving equal, sexp_of]

    let is_in_progress = function
      | Uploading | Processing -> true
      | Pending | Failed | Complete -> false
    ;;
  end

  module Size = struct
    type t =
      | Xsmall
      | Small
      | Medium
      | Large
      | Pixels of float
    [@@deriving equal, sexp_of]

    let xsmall = Xsmall
    let small = Small
    let medium = Medium
    let large = Large

    let pixels value =
      if Float.is_finite value && Float.(value >= 1. && value <= 1_000_000.)
      then Ok (Pixels value)
      else Or_error.error_string "attachment size must be finite and in 1..1000000"
    ;;

    let media = function
      | Xsmall -> 28.
      | Small -> 32.
      | Medium -> 40.
      | Large -> 48.
      | Pixels value -> value
    ;;

    let radius = function
      | Xsmall -> 12.
      | Small | Medium | Large | Pixels _ -> 16.
    ;;

    let media_radius = function
      | Xsmall -> 4.
      | Small | Medium | Large | Pixels _ -> 6.
    ;;

    let fields t ~has_media ~has_content =
      let gap, font, horizontal, vertical =
        match t with
        | Xsmall -> 6., 12., 6., 4.
        | Small -> 10., 12., 8., 6.
        | Medium -> 8., 14., 10., 8.
        | Large -> 12., 16., 16., 12.
        | Pixels value -> 4., value *. 0.875, value *. 0.25, value *. 0.25
      in
      [ Gap (px gap); Font_size font ]
      @
      if has_media
      then [ Padding (px vertical) ]
      else if has_content
      then
        [ Padding_left (px horizontal)
        ; Padding_right (px horizontal)
        ; Padding_top (px vertical)
        ; Padding_bottom (px vertical)
        ]
      else []
    ;;
  end

  let alpha color factor = Color.with_opacity color factor |> Or_error.ok_exn

  let fill_box =
    [ Position Absolute; Top (px 0.); Right (px 0.); Bottom (px 0.); Left (px 0.) ]
  ;;

  let single_line =
    [ Min_width (px 0.)
    ; Max_width full
    ; White_space No_wrap
    ; Text_overflow Ellipsis
    ; Overflow_x Hidden
    ]
  ;;

  module Title = struct
    type t =
      { key : Key.t
      ; style : Style.t
      ; status : Status.t option
      ; shimmer : Text_shimmer.Config.t option
      ; text : string
      }

    let create ~key ?(style = Style.empty) ?status ?shimmer text =
      if
        String.length text > Gpuio_protocol.Text_shimmer_wire.max_text_bytes
        || not (Stdlib.String.is_valid_utf_8 text)
      then
        Or_error.error_string
          "attachment title must be valid UTF-8 of at most 16384 bytes"
      else Ok { key; style; status; shimmer; text }
    ;;

    let view t ~status ~shimmer =
      let status = Option.value t.status ~default:status in
      let shimmer = Option.value t.shimmer ~default:shimmer in
      View.text
        ~key:t.key
        ~style:(Style.merge [ style (single_line @ [ Font_weight 500 ]); t.style ])
        t.text
      |> fun view ->
      View.with_text_shimmer view (Option.some_if (Status.is_in_progress status) shimmer)
      |> Or_error.ok_exn
    ;;
  end

  module Description = struct
    type t =
      { key : Key.t
      ; style : Style.t
      ; status : Status.t option
      ; text : string
      }

    let create ~key ?(style = Style.empty) ?status text = { key; style; status; text }

    let view t (p : Appearance.t) ~status =
      let status = Option.value t.status ~default:status in
      let foreground =
        if Status.equal status Failed then alpha p.danger 0.8 else p.muted
      in
      View.text
        ~key:t.key
        ~style:
          (Style.merge
             [ style
                 (single_line
                  @ [ Foreground foreground
                    ; Font_size 12.
                    ; Line_height (Length.percent_exn 125.)
                    ])
             ; t.style
             ])
        t.text
    ;;
  end

  module Content = struct
    module Item = struct
      type 'action t =
        | Title of Title.t
        | Description of Description.t
        | Element of Key.t * 'action View.t

      let title t = Title t
      let description t = Description t
      let element ~key t = Element (key, t)

      let view t p ~status ~shimmer =
        match t with
        | Title t -> Title.view t ~status ~shimmer
        | Description t -> Description.view t p ~status
        | Element (key, view) ->
          View.column ~key ~style:(style [ Min_width (px 0.) ]) [ view ]
      ;;
    end

    type 'action t =
      { style : Style.t
      ; items : 'action Item.t list
      }

    let create ?(style = Style.empty) items = { style; items }

    let view t p ~status ~shimmer ~axis =
      View.column
        ~key:(internal_key "content")
        ~style:
          (Style.merge
             [ style
                 ([ Min_width (px 0.)
                  ; Max_width full
                  ; Grow 1.
                  ; Shrink 1.
                  ; Basis (px 0.)
                  ; Gap (px 2.)
                  ; Line_height (Length.percent_exn 125.)
                  ]
                  @
                  match axis with
                  | Axis.Horizontal -> []
                  | Vertical ->
                    [ Width full; Padding_left (px 4.); Padding_right (px 4.) ])
             ; t.style
             ])
        (List.map t.items ~f:(fun item -> Item.view item p ~status ~shimmer))
    ;;
  end

  module Media = struct
    module Image = struct
      type 'action t =
        { config : Image.Config.t
        ; on_change : (Image.State.t -> 'action) option
        }

      let create ~asset ~description ?(fit = Image.Fit.Cover) ?on_change () =
        { config = Image.Config.create ~asset ~description ~fit (); on_change }
      ;;

      let view t ~dimmed =
        View.image
          ~key:(internal_key "image")
          ~style:
            (style
               (fill_box
                @ [ Width full; Height full; Opacity (if dimmed then 0.6 else 1.) ]))
          ?on_change:t.on_change
          t.config
      ;;
    end

    type 'action t =
      { style : Style.t
      ; size : Size.t option
      ; image : 'action Image.t option
      ; overlay : 'action View.t option
      ; children : 'action View.t list
      }

    let create ?(style = Style.empty) ?size ?image ?overlay children =
      { style; size; image; overlay; children }
    ;;

    let view t (p : Appearance.t) ~status ~size ~axis =
      let size = Option.value t.size ~default:size in
      let failed = Status.equal status Failed && Option.is_none t.image in
      let dimensions =
        match axis with
        | Axis.Horizontal ->
          [ Width (px (Size.media size)); Height (px (Size.media size)) ]
        | Vertical -> [ Width full; Aspect_ratio 1. ]
      in
      let foreground = if failed then p.danger else p.foreground in
      let background = if failed then alpha p.danger 0.1 else p.raised in
      let above key children =
        View.row
          ~key:(internal_key key)
          ~style:(style (fill_box @ [ Align_items Center; Justify_content Center ]))
          children
      in
      View.row
        ~key:(internal_key "media")
        ~style:
          (Style.merge
             [ style
                 ([ Position Relative
                  ; Shrink 0.
                  ; Align_items Center
                  ; Justify_content Center
                  ; Overflow Hidden
                  ; Radius (Size.media_radius size)
                  ; Foreground foreground
                  ; Background (solid background)
                  ]
                  @ dimensions)
             ; t.style
             ])
        (List.filter_opt
           [ Option.map t.image ~f:(fun image ->
               Image.view
                 image
                 ~dimmed:(Status.is_in_progress status || Status.equal status Failed))
           ; (if List.is_empty t.children
              then None
              else Some (above "children" t.children))
           ; Option.map t.overlay ~f:(fun overlay -> above "overlay" [ overlay ])
           ])
    ;;
  end

  module Actions = struct
    type 'action t =
      { style : Style.t
      ; children : 'action View.t list
      }

    let create ?(style = Style.empty) children = { style; children }

    let view t ~axis =
      let position =
        match axis with
        | Axis.Horizontal -> [ Position Relative ]
        | Vertical -> [ Position Absolute; Top (px 12.); Right (px 12.) ]
      in
      View.row
        ~key:(internal_key "actions")
        ~style:
          (Style.merge
             [ style ([ Shrink 0.; Align_items Center; Gap (px 4.) ] @ position)
             ; t.style
             ; style [ Pointer_occlusion Pointer ]
             ])
        t.children
    ;;
  end

  module Trigger = struct
    type 'action t =
      { key : Key.t
      ; accessible_name : string
      ; style : Style.t
      ; disabled : bool
      ; on_click : unit -> 'action
      }

    let create
          ~key
          ~accessible_name
          ?(style = Style.empty)
          ?(disabled = false)
          ~on_click
          ()
      =
      if
        String.is_empty accessible_name
        || String.length accessible_name > 1024
        || String.contains accessible_name '\000'
        || not (Stdlib.String.is_valid_utf_8 accessible_name)
      then
        Or_error.error_string
          "attachment trigger name must be nonempty UTF-8 without NUL, at most 1024 bytes"
      else Ok { key; accessible_name; style; disabled; on_click }
    ;;

    let view t (p : Appearance.t) ~radius =
      let transparent = Color.rgb_exn 0 |> fun c -> alpha c 0. in
      let defaults =
        style
          [ Width full
          ; Height full
          ; Padding (px 0.)
          ; Radius radius
          ; Background (solid transparent)
          ; Foreground p.foreground
          ; Border_width 1.
          ; Border_color transparent
          ]
        |> fun s ->
        Style.with_state_exn s Focused [ Border_color p.accent ]
        |> fun s -> Style.with_state_exn s Disabled [ Cursor Not_allowed ]
      in
      View.column
        ~key:(internal_key "activation")
        ~style:(style fill_box)
        [ View.button
            ~key:t.key
            ~accessible_name:t.accessible_name
            ~disabled:t.disabled
            ~style:(Style.merge [ defaults; t.style ])
            ~on_click:t.on_click
            ""
        ]
    ;;
  end

  let create
        (p : Appearance.t)
        ?key
        ?style:(custom = Style.empty)
        ?(status = Status.Complete)
        ?(size = Size.medium)
        ?(axis = Axis.Horizontal)
        ?shimmer
        ?media
        ?content
        ?actions
        ?trigger
        ()
    =
    let shimmer = Option.value shimmer ~default:p.text_shimmer in
    let radius = Size.radius size in
    let clickable = Option.exists trigger ~f:(fun t -> not t.Trigger.disabled) in
    let border = if Status.equal status Failed then alpha p.danger 0.3 else p.border in
    let dimensions =
      match axis with
      | Axis.Horizontal -> [ Min_width (px 160.); Align_items Center ]
      | Vertical ->
        [ Width (px (if Option.is_some content then 120. else 96.)); Align_items Start ]
    in
    let defaults =
      style
        ([ Position Relative
         ; Shrink 0.
         ; Max_width full
         ; Min_width (px 0.)
         ; Radius radius
         ; Border_width 1.
         ; Border_color border
         ; Border_style (if Status.equal status Pending then Dashed else Solid)
         ; Background (solid p.surface)
         ; Foreground p.foreground
         ; Line_height (Length.percent_exn 125.)
         ]
         @ Size.fields
             size
             ~has_media:(Option.is_some media)
             ~has_content:(Option.is_some content)
         @ dimensions)
      |> fun s ->
      Style.with_state_exn
        s
        Hovered
        (if clickable then [ Background (solid (alpha p.raised 0.5)) ] else [])
    in
    let children =
      List.filter_opt
        [ Option.map media ~f:(fun t -> Media.view t p ~status ~size ~axis)
        ; Option.map content ~f:(fun t -> Content.view t p ~status ~shimmer ~axis)
        ; Option.map trigger ~f:(fun t -> Trigger.view t p ~radius)
        ; Option.map actions ~f:(fun t -> Actions.view t ~axis)
        ]
    in
    (match axis with
     | Axis.Horizontal -> View.row ?key
     | Vertical -> View.column ?key)
      ~style:(Style.merge [ defaults; custom ])
      children
    |> semantic Group
  ;;

  let group ~key ?style:(custom = Style.empty) children =
    View.row
      ~key
      ~style:
        (Style.merge
           [ style
               [ Width full
               ; Min_width (px 0.)
               ; Gap (px 12.)
               ; Padding_top (px 4.)
               ; Padding_bottom (px 4.)
               ; Overflow_x Scroll
               ]
           ; custom
           ])
      children
  ;;
end

let message
      (p : Appearance.t)
      ?key
      ?style:(custom = Style.empty)
      ?avatar
      ~author
      ?detail
      ?actions
      ?footer
      content
  =
  let header =
    View.row
      ~style:(style [ Align_items Center; Gap (px 9.); Min_width (px 0.) ])
      (List.filter_opt
         [ optional_slot "avatar" avatar
         ; Some
             (View.text
                ~key:(internal_key "author")
                ~style:(style [ Font_size 12.; Font_weight 600; Min_width (px 0.) ])
                author)
         ; secondary p "detail" detail
         ; Some (View.column ~key:(internal_key "spacer") ~style:(style [ Grow 1. ]) [])
         ; optional_slot "actions" actions
         ])
  in
  group_box
    p
    ?key
    ~style:(Style.merge [ style [ Border_width 0.; Padding (px 6.) ]; custom ])
    ~header
    ?footer
    [ content ]
;;

let bubble
      (p : Appearance.t)
      ?key
      ?style:(custom = Style.empty)
      ?(tone = Tone.Neutral)
      content
  =
  View.column
    ?key
    ~style:
      (Style.merge
         [ style
             [ Min_width (px 0.)
             ; Padding (px 12.)
             ; Radius 14.
             ; Background (solid p.Appearance.raised)
             ; Foreground (color p tone)
             ]
         ; custom
         ])
    [ content ]
;;

let tool_result (p : Appearance.t) ?key ?style ~title ?status ?actions ?footer content =
  let header =
    View.row
      ~style:(Style.create_exn [ Align_items Center; Gap (px 8.); Min_width (px 0.) ])
      (List.filter_opt
         [ Some
             (View.text
                ~key:(internal_key "title")
                ~style:
                  (Style.create_exn
                     [ Font_size 12.; Font_weight 600; Grow 1.; Min_width (px 0.) ])
                title)
         ; optional_slot "status" status
         ; optional_slot "actions" actions
         ])
  in
  group_box p ?key ?style ~header ?footer [ content ]
;;
