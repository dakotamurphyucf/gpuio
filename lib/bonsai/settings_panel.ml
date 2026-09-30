open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module S = Gpuio.Settings
module V = Gpuio.View
module P = Gpuio.Presentation

type view = unit E.t V.t

let ok = Or_error.ok_exn
let key = Gpuio.Key.of_string_exn
let px = Gpuio.Length.px_exn
let full = Gpuio.Length.percent_exn 100.
let style = Gpuio.Style.create_exn
let text = P.label

module Labels = struct
  type t =
    { navigation : string
    ; empty : string
    ; resize : string
    ; reset_matches : string
    ; reset_page : string
    ; reset_group : string
    ; current_page : string
    ; current_group : string
    ; expand : string -> string
    ; collapse : string -> string
    }

  let valid text =
    (not (String.is_empty (String.strip text)))
    && String.length text <= 4096
    && Stdlib.String.is_valid_utf_8 text
    && not (String.contains text '\000')
  ;;

  let create
        ~navigation
        ~empty
        ~resize
        ~reset_matches
        ~reset_page
        ~reset_group
        ~current_page
        ~current_group
        ~expand
        ~collapse
    =
    if
      List.for_all
        [ navigation
        ; empty
        ; resize
        ; reset_matches
        ; reset_page
        ; reset_group
        ; current_page
        ; current_group
        ]
        ~f:valid
    then
      Ok
        { navigation
        ; empty
        ; resize
        ; reset_matches
        ; reset_page
        ; reset_group
        ; current_page
        ; current_group
        ; expand
        ; collapse
        }
    else Or_error.error_string "invalid settings labels"
  ;;

  let english =
    create
      ~navigation:"Settings navigation"
      ~empty:"No matching settings"
      ~resize:"Settings sidebar width"
      ~reset_matches:"Reset matching settings"
      ~reset_page:"Reset entire page"
      ~reset_group:"Reset group"
      ~current_page:"Current page"
      ~current_group:"Current group"
      ~expand:(fun title -> "Expand " ^ title)
      ~collapse:(fun title -> "Collapse " ^ title)
    |> ok
  ;;
end

module Output = struct
  type t =
    { view : view
    ; active_groups : int
    ; budget_exhausted : bool
    }

  let view t = t.view
  let active_groups t = t.active_groups
  let budget_exhausted t = t.budget_exhausted
end

let selected_page model =
  Option.bind (S.selection model) ~f:(fun selected ->
    List.find (S.filtered_pages model) ~f:(fun page ->
      S.Page_id.equal selected.page (S.Page.id page)))
;;

let gap = function
  | P.Size.Small -> 8.
  | Medium -> 12.
  | Large -> 16.
;;

let heading title description =
  V.column
    ~style:(style [ Gap (px 6.) ])
    (Option.to_list
       (Option.map title ~f:(fun title -> text ~style:(style [ Font_size 17. ]) title))
     @ Option.to_list
         (Option.map description ~f:(fun description ->
            text ~style:(style [ Opacity 0.7 ]) description)))
;;

let item_view item layout size control =
  let disabled = S.Item.is_disabled item in
  let children =
    match S.Item.title item with
    | None -> [ V.column ~key:(key "field") [ control ] ]
    | Some title ->
      [ V.column
          ~key:(key "label")
          ~style:(style [ Grow 1.; Min_width (px 0.) ])
          [ heading (Some title) (S.Item.description item) ]
      ; V.column ~key:(key "field") ~style:(style [ Min_width (px 0.) ]) [ control ]
      ]
  in
  V.column
    ~key:(key (S.Item_id.to_string (S.Item.id item)))
    ~style:
      (style
         [ Direction (if S.Layout.equal layout Horizontal then Row else Column)
         ; Gap (px (gap size))
         ; Inert disabled
         ; Opacity (if disabled then 0.5 else 1.)
         ; Width full
         ])
    children
;;

let wide = Gpuio.Container_query.Branch_id.of_string "wide" |> ok
let narrow = Gpuio.Container_query.Branch_id.of_string "narrow" |> ok

let breakpoint =
  let module Q = Gpuio.Container_query in
  Q.Config.create
    ~default:narrow
    [ Q.Rule.create
        ~condition:(Q.Predicate.create ~width:(Q.Range.create ~minimum:480. () |> ok) ())
        ~branch:wide
    ]
  |> ok
;;

let group_collection groups graph =
  let module C = Gpuio.List_collection in
  let previous, set_previous = B.state_opt ~equal:phys_equal graph in
  let open B.Let_syntax in
  let collection =
    let%arr groups = groups
    and previous = previous in
    let entries =
      List.map groups ~f:(fun group -> S.Group_id.to_string (S.Group.id group), group)
    in
    match previous with
    | Some previous
      when List.equal String.equal (C.keys previous) (List.map entries ~f:fst) ->
      List.fold entries ~init:previous ~f:(fun collection (key, data) ->
        match C.find collection key with
        | Some old when phys_equal old data || S.Group.equal old data -> collection
        | _ -> C.set collection ~key ~data |> ok)
    | _ -> C.of_alist (module String) entries |> ok
  in
  let after_display =
    let%arr collection = collection
    and previous = previous
    and set_previous = set_previous in
    if Option.exists previous ~f:(phys_equal collection)
    then E.Ignore
    else set_previous (Some collection)
  in
  B.Edge.after_display after_display graph;
  collection
;;

let component
      ?key:root_key
      ?style:(root_style = B.return Gpuio.Style.empty)
      ?(labels = Labels.english)
      ?split
      ?groups
      ?(group_variant = B.return P.Group_variant.Outline)
      ?(size = B.return P.Size.Medium)
      ?(on_resize = B.return (fun _ -> E.Ignore))
      ?(page_suffix = B.return (fun _ -> None))
      ?(page_icon = B.return (fun _ -> None))
      ~appearance
      ~model
      ~search
      ~on_request
      ~on_reset
      ~render_item
      graph
  =
  let split =
    Option.value
      split
      ~default:
        (Gpuio.Split_pane.Config.create
           ~label:labels.resize
           ~initial_first:250.
           ~minimum_first:160.
           ~maximum_first:360.
           ~minimum_second:0.
           ()
         |> ok)
  in
  let groups =
    Option.value
      groups
      ~default:
        (Gpuio.Virtual_list.Config.create
           ~height:(Estimated 200.)
           ~overscan:400.
           ~max_active:32
           ()
         |> ok)
  in
  let layout, set_layout = B.state S.Layout.Horizontal ~equal:S.Layout.equal graph in
  let pages =
    B.map model ~f:(fun model ->
      match selected_page model with
      | None -> String.Map.empty
      | Some page -> String.Map.singleton (S.Page_id.to_string (S.Page.id page)) page)
  in
  let rows =
    Managed_rows.assoc
      (module String)
      pages
      ~f:(fun _ page _ graph ->
        let groups_data =
          B.map page ~f:S.Page.groups |> B.cutoff ~equal:(List.equal S.Group.equal)
        in
        let collection = group_collection groups_data graph in
        Virtual_list.component
          (module String)
          collection
          ~row_key:key
          ~config:groups
          ~key:(key "settings-groups")
          ~style:(B.return (style [ Grow 1.; Min_height (px 0.); Width full ]))
          ~render_row:(fun ~key:_ ~data ~lifetime graph ->
            let items =
              B.map data ~f:(fun group ->
                String.Map.of_alist_exn
                  (List.map (S.Group.items group) ~f:(fun item ->
                     S.Item_id.to_string (S.Item.id item), item)))
            in
            let views =
              Managed_rows.assoc
                (module String)
                items
                ~f:(fun _ item lifetime graph ->
                  let open B.Let_syntax in
                  let effective_layout =
                    let%arr item = item
                    and layout = layout in
                    if S.Layout.equal (S.Item.layout item) Vertical
                    then S.Layout.Vertical
                    else layout
                  in
                  let control =
                    render_item ~item ~layout:effective_layout ~lifetime graph
                  in
                  let%arr item = item
                  and layout = effective_layout
                  and size = size
                  and control = control in
                  item_view item layout size control)
                graph
            in
            let open B.Let_syntax in
            let%arr data = data
            and views = views
            and appearance = appearance
            and variant = group_variant
            and size = size
            and model = model
            and on_reset = on_reset
            and lifetime = lifetime in
            let children =
              List.map (S.Group.items data) ~f:(fun item ->
                Map.find_exn views (S.Item_id.to_string (S.Item.id item)))
            in
            P.group_box
              appearance
              ~key:(key (S.Group_id.to_string (S.Group.id data)))
              ~variant
              ~style:(style [ Margin_bottom (px (gap size)) ])
              ~body_style:(style [ Gap (px (gap size)) ])
              ~header:
                (V.row
                   ~style:(style [ Gap (px 8.); Align_items Center ])
                   [ V.column
                       ~style:(style [ Grow 1.; Min_width (px 0.) ])
                       [ heading (S.Group.title data) (S.Group.description data) ]
                   ; (V.button
                        ~style:(style [ Shrink 0.; Font_size 12.; Padding (px 5.) ])
                        ~disabled:
                          (List.is_empty
                             (S.reset_targets
                                model
                                ~scope:(S.Reset_scope.Matching_group (S.Group.id data))))
                        ~on_click:(fun () ->
                          Managed_rows.Lifetime.guard
                            lifetime
                            (on_reset (S.Reset_scope.Matching_group (S.Group.id data))))
                        labels.reset_group
                      |> fun view ->
                      V.with_accessibility
                        view
                        (Gpuio.Accessibility.create
                           ~description:
                             (Option.value
                                (S.Group.title data)
                                ~default:(S.Group_id.to_string (S.Group.id data)))
                           ()
                         |> ok)
                      |> ok)
                   ])
              children)
          graph)
      graph
    |> B.map ~f:(fun results ->
      match Map.data results with
      | [] -> Ok None
      | [ result ] -> Or_error.map result ~f:Option.some
      | _ -> assert false)
  in
  let target =
    B.map model ~f:(fun model ->
      Option.bind (S.selection model) ~f:(fun selected -> selected.group))
  in
  let callback =
    B.map rows ~f:(fun rows target ->
      match rows, target with
      | Ok (Some rows), Some target ->
        Virtual_list.Controller.reveal
          (Virtual_list.Output.controller rows)
          (S.Group_id.to_string target)
      | Error _, _ | Ok None, _ | _, None -> E.Ignore)
  in
  B.Edge.on_change ~equal:(Option.equal S.Group_id.equal) target ~callback graph;
  let open B.Let_syntax in
  let%arr model = model
  and search = search
  and appearance = appearance
  and rows = rows
  and on_request = on_request
  and on_reset = on_reset
  and on_resize = on_resize
  and suffix = page_suffix
  and icon = page_icon
  and root_style = root_style
  and set_layout = set_layout in
  let open Or_error.Let_syntax in
  let%bind rows = rows in
  let selection = S.selection model in
  let navigate selected () =
    let reveal =
      match rows, selection, selected.S.Selection.group with
      | Some rows, Some current, Some group
        when S.Page_id.equal current.page selected.page ->
        Virtual_list.Controller.reveal
          (Virtual_list.Output.controller rows)
          (S.Group_id.to_string group)
      | _ -> E.Ignore
    in
    E.Many [ on_request (S.Request.Select selected); reveal ]
  in
  let link ~icon ~key:key_id ~current ~label ~target ~current_label ~current_kind =
    let open Or_error.Let_syntax in
    let%bind config = Gpuio.Link.Config.create ~label () in
    let%bind view =
      P.composed_link
        appearance
        ~key:key_id
        ~style:
          (style [ Width full; Padding (px 8.); Opacity (if current then 1. else 0.75) ])
        config
        ~on_click:(navigate target)
        [ V.row
            ~style:(style [ Gap (px 8.); Align_items Center ])
            (Option.to_list icon @ [ text label ])
        ]
    in
    if not current
    then Ok view
    else
      V.with_accessibility
        view
        (Gpuio.Accessibility.create
           ~role:Link
           ~current:current_kind
           ~description:current_label
           ()
         |> ok)
  in
  let%bind destinations =
    List.map (S.filtered_pages model) ~f:(fun page ->
      let page_id = S.Page.id page in
      let current =
        Option.exists selection ~f:(fun selected -> S.Page_id.equal selected.page page_id)
      in
      let expanded = List.mem (S.expanded_pages model) page_id ~equal:S.Page_id.equal in
      let open Or_error.Let_syntax in
      let%bind page_link =
        link
          ~icon:(icon page)
          ~key:(key "page")
          ~current:
            (current
             && Option.exists selection ~f:(fun selected -> Option.is_none selected.group)
            )
          ~current_label:labels.current_page
          ~current_kind:Page
          ~label:(S.Page.title page)
          ~target:(S.Selection.create page_id)
      in
      let%bind children =
        List.filter_map (S.Page.groups page) ~f:(fun group ->
          Option.map (S.Group.title group) ~f:(fun title ->
            link
              ~icon:None
              ~key:(key (S.Group_id.to_string (S.Group.id group)))
              ~current:
                (current
                 && Option.exists selection ~f:(fun selected ->
                   Option.equal S.Group_id.equal selected.group (Some (S.Group.id group)))
                )
              ~label:title
              ~current_label:labels.current_group
              ~current_kind:Location
              ~target:(S.Selection.create ~group:(S.Group.id group) page_id)))
        |> Or_error.combine_errors
      in
      let label =
        (if expanded then labels.collapse else labels.expand) (S.Page.title page)
      in
      if not (Labels.valid label)
      then Or_error.error_string "invalid dynamic settings navigation label"
      else (
        let%bind trigger =
          V.with_accessibility
            (V.button
               ~style:(style [ Width (px 28.); Height (px 28.); Shrink 0. ])
               ~on_click:(fun () -> on_request (Toggle_page page_id))
               (if expanded then "▾" else "▸"))
            (Gpuio.Accessibility.create ~label () |> ok)
        in
        V.disclosure_with_header
          ~key:(key (S.Page_id.to_string page_id))
          ~label:(S.Page.title page)
          ~expanded
          ~hidden:Retain
          ~header:[ page_link ]
          ~trigger
          children))
    |> Or_error.combine_errors
  in
  let%bind navigation =
    V.with_accessibility
      (V.column
         ~style:(style [ Height full; Min_height (px 0.); Padding (px 8.); Gap (px 8.) ])
         [ V.column ~key:(key "search") [ search ]
         ; V.column
             ~key:(key "pages")
             ~style:(style [ Overflow_y Scroll; Grow 1.; Min_height (px 0.) ])
             destinations
         ])
      (Gpuio.Accessibility.create ~role:Navigation ~label:labels.navigation () |> ok)
  in
  let header, content =
    match selected_page model, rows with
    | None, _ | _, None -> [], [ text labels.empty ]
    | Some page, Some rows ->
      let reset scope label =
        V.button
          ~disabled:(List.is_empty (S.reset_targets model ~scope))
          ~on_click:(fun () -> on_reset scope)
          label
      in
      ( [ V.row
            ~key:(key "heading")
            ~style:(style [ Gap (px 12.); Wrap Wrap; Align_items Center ])
            (heading (Some (S.Page.title page)) (S.Page.description page)
             :: Option.to_list (suffix page))
        ; V.row
            ~key:(key "reset")
            ~style:(style [ Gap (px 8.); Wrap Wrap ])
            [ reset (Matching_page (S.Page.id page)) labels.reset_matches
            ; reset (Whole_page (S.Page.id page)) labels.reset_page
            ]
        ]
      , [ V.column
            ~key:(key (S.Page_id.to_string (S.Page.id page)))
            ~style:(style [ Grow 1.; Min_height (px 0.); Width full ])
            [ Virtual_list.Output.view rows ]
        ] )
  in
  let%map probe =
    V.container_query
      ~key:(key "width-observer")
      ~style:
        (style
           [ Position Absolute
           ; Left (px 0.)
           ; Right (px 0.)
           ; Top (px 0.)
           ; Height (px 1.)
           ; Pointer_events false
           ])
      ~on_select:(fun selected ->
        set_layout
          (if Gpuio.Container_query.Branch_id.equal selected.branch wide
           then Horizontal
           else Vertical))
      breakpoint
      [ narrow, V.column []; wide, V.column [] ]
  in
  let second =
    V.column
      ~style:
        (style
           [ Position Relative
           ; Height full
           ; Width full
           ; Min_width (px 0.)
           ; Min_height (px 0.)
           ; Gap (px 12.)
           ; Padding (px 12.)
           ])
      [ probe
      ; V.column ~key:(key "header") header
      ; V.column
          ~key:(key "content")
          ~style:(style [ Grow 1.; Min_height (px 0.); Width full ])
          content
      ]
  in
  { Output.view =
      V.split_pane
        ?key:root_key
        ~style:(Gpuio.Style.merge [ style [ Width full; Height full ]; root_style ])
        ~config:split
        ~on_resize
        ~first:navigation
        ~second
        ()
  ; active_groups = Option.value_map rows ~default:0 ~f:Virtual_list.Output.active_rows
  ; budget_exhausted =
      Option.value_map rows ~default:false ~f:Virtual_list.Output.budget_exhausted
  }
;;
