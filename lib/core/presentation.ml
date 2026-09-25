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
    }
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

let group_box
      (p : Appearance.t)
      ?key
      ?style:(custom = Style.empty)
      ?header
      ?footer
      children
  =
  View.column
    ?key
    ~style:
      (Style.merge
         [ style
             [ Min_width (px 0.)
             ; Gap (px 12.)
             ; Padding (px 16.)
             ; Radius 12.
             ; Border_width 1.
             ; Border_color p.Appearance.border
             ; Background (solid p.surface)
             ; Foreground p.foreground
             ]
         ; custom
         ])
    (List.filter_opt
       [ optional_slot "header" header
       ; Some
           (View.column
              ~key:(internal_key "body")
              ~style:(style [ Min_width (px 0.); Gap (px 12.) ])
              children)
       ; optional_slot "footer" footer
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

let status_bar (p : Appearance.t) ?key ?style:(custom = Style.empty) ?leading ?trailing ()
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
             ~style:(style [ Min_width (px 0.); Grow 1. ])
             [ content ])
       ; Some (View.column ~key:(internal_key "spacer") ~style:(style [ Grow 1. ]) [])
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
