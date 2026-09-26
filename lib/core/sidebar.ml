open Core
module Id = Choice.Id

let max_items = 4096
let max_depth = 16
let max_groups = 128
let max_text_bytes = 262_144
let text_bytes text = String.length text
let valid_text = Gpuio_protocol.Accessibility_wire.valid_text

module Item = struct
  type t =
    { choice : Choice.t
    ; compact_label : string
    ; children : t list
    ; count : int
    ; depth : int
    ; text_bytes : int
    }
  [@@deriving equal, sexp_of]

  let create ~id ~label ?(compact_label = "•") ?disabled ?(children = []) () =
    let open Or_error.Let_syntax in
    let%bind choice = Choice.create ~id ~label ?disabled () in
    let count = 1 + List.sum (module Int) children ~f:(fun c -> c.count) in
    let depth = 1 + List.fold children ~init:0 ~f:(fun d c -> Int.max d c.depth) in
    let text_bytes =
      text_bytes (Id.to_string id)
      + text_bytes label
      + text_bytes compact_label
      + List.sum (module Int) children ~f:(fun c -> c.text_bytes)
    in
    if
      (not (valid_text compact_label))
      || count > max_items
      || depth > max_depth
      || text_bytes > max_text_bytes
    then Or_error.error_string "invalid or oversized sidebar subtree"
    else Ok { choice; compact_label; children; count; depth; text_bytes }
  ;;

  let id t = Choice.id t.choice
  let label t = Choice.label t.choice
  let compact_label t = t.compact_label
  let is_disabled t = Choice.is_disabled t.choice
  let children t = t.children
end

module Group = struct
  type t =
    { id : Id.t
    ; label : string option
    ; items : Item.t list
    }
  [@@deriving equal, sexp_of]

  let create ~id ?label items =
    if
      Option.for_all label ~f:valid_text
      && List.sum (module Int) items ~f:(fun item -> item.Item.count) <= max_items
      && List.sum (module Int) items ~f:(fun item -> item.Item.text_bytes)
         <= max_text_bytes
    then Ok { id; label; items }
    else Or_error.error_string "invalid or oversized sidebar group"
  ;;

  let id t = t.id
  let label t = t.label
  let items t = t.items
end

module Collapse = struct
  type t =
    | Icon
    | Offcanvas
    | Never
  [@@deriving equal, sexp_of]
end

module Side = struct
  type t =
    | Left
    | Right
  [@@deriving equal, sexp_of]
end

module Request = struct
  type t =
    | Select of Id.t
    | Toggle of Id.t
    | Toggle_collapsed
  [@@deriving equal, sexp_of]
end

module Entry = struct
  type t =
    { item : Item.t
    ; parent : Id.t option
    }
  [@@deriving equal, sexp_of]
end

type t =
  { groups : Group.t list
  ; index : Entry.t String.Map.t
  ; selected : Id.t option
  ; expanded : String.Set.t
  ; collapse : Collapse.t
  ; collapsed : bool
  ; disabled : bool
  }
[@@deriving equal, sexp_of]

let groups t = t.groups
let selected t = t.selected
let collapse t = t.collapse
let is_collapsed t = t.collapsed && not (Collapse.equal t.collapse Never)
let is_disabled t = t.disabled
let with_collapsed t collapsed = { t with collapsed }
let with_collapse t collapse = { t with collapse }
let with_disabled t disabled = { t with disabled }

let find t id =
  Map.find t.index (Id.to_string id) |> Option.map ~f:(fun e -> e.Entry.item)
;;

let is_expanded t id = Set.mem t.expanded (Id.to_string id)

let rec ordered_items items =
  List.concat_map items ~f:(fun item -> item :: ordered_items (Item.children item))
;;

let expanded t =
  List.concat_map t.groups ~f:(fun group -> ordered_items (Group.items group))
  |> List.filter_map ~f:(fun item ->
    Option.some_if (is_expanded t (Item.id item)) (Item.id item))
;;

let validate_selection index = function
  | None -> Ok ()
  | Some id ->
    if Map.mem index (Id.to_string id)
    then Ok ()
    else Or_error.error_string "sidebar selection is absent"
;;

let index groups =
  let open Or_error.Let_syntax in
  let count =
    List.sum
      (module Int)
      groups
      ~f:(fun group ->
        List.sum (module Int) (Group.items group) ~f:(fun item -> item.Item.count))
  in
  let bytes =
    List.sum
      (module Int)
      groups
      ~f:(fun group ->
        text_bytes (Id.to_string (Group.id group))
        + Option.value_map (Group.label group) ~default:0 ~f:text_bytes
        + List.sum (module Int) (Group.items group) ~f:(fun item -> item.Item.text_bytes))
  in
  let%bind () =
    if List.length groups > max_groups || count > max_items || bytes > max_text_bytes
    then Or_error.error_string "sidebar collection limit exceeded"
    else if
      List.contains_dup groups ~compare:(fun a b -> Id.compare (Group.id a) (Group.id b))
    then Or_error.error_string "duplicate sidebar group ID"
    else Ok ()
  in
  let rec add index parent items =
    List.fold_result items ~init:index ~f:(fun index item ->
      let id = Item.id item in
      let%bind index =
        match Map.add index ~key:(Id.to_string id) ~data:{ Entry.item; parent } with
        | `Duplicate -> Or_error.error_string "duplicate sidebar item ID"
        | `Ok index -> Ok index
      in
      add index (Some id) (Item.children item))
  in
  List.fold_result groups ~init:String.Map.empty ~f:(fun index group ->
    add index None (Group.items group))
;;

let create
      ~groups
      ~selected
      ?(expanded = [])
      ?(collapse = Collapse.Icon)
      ?(collapsed = false)
      ?(disabled = false)
      ()
  =
  let open Or_error.Let_syntax in
  let%bind index = index groups in
  let%bind () = validate_selection index selected in
  let%map expanded =
    List.fold_result expanded ~init:String.Set.empty ~f:(fun set id ->
      let key = Id.to_string id in
      if Set.mem set key
      then Or_error.error_string "duplicate sidebar expansion ID"
      else (
        match Map.find index key with
        | Some entry when not (List.is_empty (Item.children entry.item)) ->
          Ok (Set.add set key)
        | None | Some _ -> Or_error.error_string "sidebar expansion must name a branch"))
  in
  { groups; index; selected; expanded; collapse; collapsed; disabled }
;;

let select t selected =
  let%map.Or_error () = validate_selection t.index selected in
  { t with selected }
;;

let with_groups t groups =
  let%map.Or_error index = index groups in
  let selected =
    Option.filter t.selected ~f:(fun id -> Map.mem index (Id.to_string id))
  in
  let expanded =
    Set.filter t.expanded ~f:(fun id ->
      Option.exists (Map.find index id) ~f:(fun entry ->
        not (List.is_empty (Item.children entry.item))))
  in
  { t with groups; index; selected; expanded }
;;

let is_visible t id =
  let rec ancestors id =
    match Map.find t.index (Id.to_string id) with
    | None -> false
    | Some entry ->
      (match entry.parent with
       | None -> true
       | Some parent -> is_expanded t parent && ancestors parent)
  in
  if is_collapsed t
  then (
    match t.collapse with
    | Offcanvas -> false
    | Icon ->
      Option.exists
        (Map.find t.index (Id.to_string id))
        ~f:(fun e -> Option.is_none e.parent)
    | Never -> ancestors id)
  else ancestors id
;;

let apply_request t request =
  if t.disabled
  then t
  else (
    match request with
    | Request.Toggle_collapsed ->
      if Collapse.equal t.collapse Never
      then t
      else { t with collapsed = not t.collapsed }
    | Select id | Toggle id ->
      (match find t id with
       | None -> t
       | Some item ->
         if Item.is_disabled item || not (is_visible t id)
         then t
         else (
           match request with
           | Select id -> { t with selected = Some id }
           | Toggle id ->
             if
               List.is_empty (Item.children item)
               || (is_collapsed t && Collapse.equal t.collapse Icon)
             then t
             else (
               let key = Id.to_string id in
               { t with
                 expanded =
                   (if Set.mem t.expanded key
                    then Set.remove t.expanded key
                    else Set.add t.expanded key)
               })
           | Toggle_collapsed -> t)))
;;

module Labels = struct
  type t =
    { navigation : string
    ; current : string
    ; expand_sidebar : string
    ; collapse_sidebar : string
    ; toggle_item : label:string -> expanded:bool -> string
    }

  let create ~navigation ~current ~expand_sidebar ~collapse_sidebar ~toggle_item =
    if
      List.for_all [ navigation; current; expand_sidebar; collapse_sidebar ] ~f:valid_text
    then Ok { navigation; current; expand_sidebar; collapse_sidebar; toggle_item }
    else Or_error.error_string "invalid sidebar labels"
  ;;

  let english =
    create
      ~navigation:"Sidebar"
      ~current:"Current destination"
      ~expand_sidebar:"Expand sidebar"
      ~collapse_sidebar:"Collapse sidebar"
      ~toggle_item:(fun ~label ~expanded ->
        (if expanded then "Collapse " else "Expand ") ^ label)
    |> Or_error.ok_exn
  ;;
end

module Appearance = struct
  type t =
    { width : float
    ; compact_width : float
    ; style : Style.t
    ; item_style : Style.t
    ; current_style : Style.t
    ; group_style : Style.t
    }

  let create
        ?(width = 240.)
        ?(compact_width = 56.)
        ?(style = Style.empty)
        ?(item_style = Style.empty)
        ?(current_style = Style.empty)
        ?(group_style = Style.empty)
        ()
    =
    if
      (not (Float.is_finite width && Float.is_finite compact_width))
      || Float.(compact_width < 32. || width > 4096. || compact_width > width)
    then
      Or_error.error_string
        "sidebar widths must satisfy 32 <= compact <= expanded <= 4096"
    else Ok { width; compact_width; style; item_style; current_style; group_style }
  ;;

  let default = create () |> Or_error.ok_exn
end

module Decoration = struct
  type 'action t =
    { icon : Icon.Decoration.t option
    ; suffix : 'action View.t option
    ; context_menu : Menu.t option
    }

  let create ?icon ?suffix ?context_menu () = { icon; suffix; context_menu }
end

let toggle t ?key ?style ?(labels = Labels.english) ~on_request () =
  View.button
    ?key
    ?style
    ~disabled:(t.disabled || Collapse.equal t.collapse Never)
    ~on_click:(fun () -> on_request Request.Toggle_collapsed)
    (if is_collapsed t then labels.expand_sidebar else labels.collapse_sidebar)
;;

let view
      t
      ?key
      ?(side = Side.Left)
      ?(appearance = Appearance.default)
      ?(labels = Labels.english)
      ~hidden
      ?header
      ?footer
      ?(decorate = fun _ -> Decoration.create ())
      ~on_request
      ()
  =
  let open Or_error.Let_syntax in
  let open Style.Property in
  let px = Length.px_exn
  and full = Length.percent_exn 100. in
  let style = Style.create_exn
  and key_of = Key.of_string_exn in
  let compact = is_collapsed t && Collapse.equal t.collapse Icon in
  let offcanvas = is_collapsed t && Collapse.equal t.collapse Offcanvas in
  let hidden_style hidden = if hidden then style [ Display Hidden ] else Style.empty in
  let rec items values = List.map values ~f:item |> Or_error.all
  and item value =
    let id = Item.id value in
    let label = Item.label value in
    let decoration = decorate value in
    let selected = Option.equal Id.equal t.selected (Some id) in
    let item_style =
      Style.merge
        [ style
            [ Grow 1.
            ; Min_width (px 0.)
            ; Width full
            ; Padding (px 8.)
            ; Radius 6.
            ; Background (Background.solid (Color.token_exn "background"))
            ; Foreground (Color.token_exn "foreground")
            ; Border_width 0.
            ]
        ; appearance.item_style
        ; (if selected
           then
             Style.merge
               [ style
                   [ Font_weight 700
                   ; Border_left_width 3.
                   ; Border_color (Color.token_exn "accent")
                   ]
               ; appearance.current_style
               ]
           else Style.empty)
        ]
    in
    let text =
      if compact
      then if Option.is_some decoration.icon then "" else Item.compact_label value
      else label
    in
    let%bind metadata =
      Accessibility.create
        ~role:Link
        ~label
        ?current:(if selected then Some Location else None)
        ?description:(if selected then Some labels.current else None)
        ()
    in
    let%bind link =
      View.with_accessibility
        (View.button
           ~key:(key_of "link")
           ~style:item_style
           ?leading_icon:decoration.icon
           ~disabled:(t.disabled || Item.is_disabled value)
           ~on_click:(fun () -> on_request (Request.Select id))
           text)
        metadata
    in
    let placement =
      Placement.create
        ~side:
          (match side with
           | Left -> Right
           | Right -> Left)
        ~align:Center
        ~offset:6.
        ()
      |> Or_error.ok_exn
    in
    let%bind tooltip =
      Tooltip.Config.create ~label ~placement ~disabled:(not compact) ~hoverable:false ()
    in
    let link =
      View.tooltip
        ~key:(key_of "tooltip")
        ~style:(style [ Grow 1.; Min_width (px 0.) ])
        ~config:tooltip
        ~anchor:link
        ~content:(View.text label)
        ()
    in
    let suffix =
      View.column
        ~key:(key_of "suffix")
        ~style:(hidden_style compact)
        (Option.to_list decoration.suffix)
    in
    let row =
      View.row
        ~key:(key_of "item")
        ~style:(style [ Grow 1.; Min_width (px 0.); Align_items Center; Gap (px 6.) ])
        [ link; suffix ]
    in
    let row =
      match decoration.context_menu with
      | None -> row
      | Some menu ->
        View.context_menu
          ~key:(key_of "item")
          ~style:(style [ Grow 1.; Min_width (px 0.) ])
          ~menu
          row
    in
    let%map body =
      match Item.children value with
      | [] -> Ok row
      | children ->
        let expanded = (not compact) && is_expanded t id in
        let%bind children =
          if expanded || Content_policy.equal hidden Retain then items children else Ok []
        in
        let toggle_label = labels.toggle_item ~label ~expanded in
        let%bind metadata = Accessibility.create ~label:toggle_label () in
        let%bind trigger =
          View.with_accessibility
            (View.button
               ~style:
                 (Style.merge
                    [ style [ Padding (px 6.); Shrink 0. ]; hidden_style compact ])
               ~disabled:(t.disabled || Item.is_disabled value || compact)
               ~on_click:(fun () -> on_request (Request.Toggle id))
               (if expanded then "−" else "+"))
            metadata
        in
        View.disclosure_with_header
          ~key:(key_of "branch")
          ~label
          ~expanded
          ~hidden
          ~panel_style:(style [ Padding_left (px 12.); Gap (px 4.); Min_width (px 0.) ])
          ~header:[ row ]
          ~trigger
          children
    in
    View.column
      ~key:(key_of (Id.to_string id))
      ~style:(style [ Min_width (px 0.) ])
      [ body ]
  in
  let%bind children =
    if offcanvas && Content_policy.equal hidden Unmount
    then Ok []
    else (
      let%map groups =
        List.map t.groups ~f:(fun group ->
          let%map children = items (Group.items group) in
          let heading =
            View.column
              ~key:(key_of "heading")
              ~style:(hidden_style compact)
              (match Group.label group with
               | None -> []
               | Some label ->
                 [ View.text
                     ~style:
                       (style
                          [ Font_size 12.
                          ; Font_weight 600
                          ; Foreground (Color.token_exn "muted")
                          ; Padding (px 8.)
                          ])
                     label
                 ])
          in
          View.column
            ~key:(key_of (Id.to_string (Group.id group)))
            ~style:
              (Style.merge
                 [ style [ Gap (px 4.); Min_width (px 0.) ]; appearance.group_style ])
            [ heading
            ; View.column
                ~key:(key_of "items")
                ~style:(style [ Gap (px 4.); Min_width (px 0.) ])
                children
            ])
        |> Or_error.all
      in
      [ View.column
          ~key:(key_of "header")
          (Option.map header ~f:(fun f -> f ~compact) |> Option.to_list)
      ; View.column
          ~key:(key_of "groups")
          ~style:(style [ Grow 1.; Min_height (px 0.); Overflow_y Scroll; Gap (px 16.) ])
          groups
      ; View.column
          ~key:(key_of "footer")
          (Option.map footer ~f:(fun f -> f ~compact) |> Option.to_list)
      ])
  in
  let width =
    if offcanvas
    then 0.
    else if compact
    then appearance.compact_width
    else appearance.width
  in
  let border =
    match side with
    | Left -> Border_right_width 1.
    | Right -> Border_left_width 1.
  in
  let root_style =
    Style.merge
      [ style
          [ Width (px width)
          ; Height full
          ; Shrink 0.
          ; Min_width (px 0.)
          ; Min_height (px 0.)
          ; Padding (px 8.)
          ; Gap (px 12.)
          ; Background (Background.solid (Color.token_exn "background"))
          ; Foreground (Color.token_exn "foreground")
          ; Border_color (Color.token_exn "muted")
          ; border
          ]
      ; appearance.style
      ; (if offcanvas
         then
           style
             [ Display Hidden
             ; Width (px 0.)
             ; Min_width (px 0.)
             ; Padding (px 0.)
             ; Border_width 0.
             ]
         else Style.empty)
      ]
  in
  let panel =
    View.panel
      ~key:(key_of "content")
      ~label:labels.navigation
      ~active:(not offcanvas)
      ~hidden
      ~style:(style [ Height full; Min_height (px 0.); Gap (px 12.) ])
      children
  in
  let%bind metadata = Accessibility.create ~role:Navigation ~label:labels.navigation () in
  View.with_accessibility (View.column ?key ~style:root_style [ panel ]) metadata
;;
