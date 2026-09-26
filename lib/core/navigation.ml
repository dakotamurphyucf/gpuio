open Core
open Style.Property

let px = Length.px_exn
let style = Style.create_exn
let key = Key.of_string_exn
let token = Color.token_exn

module Appearance = struct
  type t =
    { item_style : Style.t
    ; current_style : Style.t
    ; gap_style : Style.t
    ; separator_style : Style.t
    }

  let create
        ?(item_style = Style.empty)
        ?(current_style = Style.empty)
        ?(gap_style = Style.empty)
        ?(separator_style = Style.empty)
        ()
    =
    { item_style; current_style; gap_style; separator_style }
  ;;

  let default = create ()
end

let annotate view ?role ?label ?description ?current () =
  let%bind.Or_error metadata =
    Accessibility.create ?role ?label ?description ?current ()
  in
  View.with_accessibility view metadata
;;

let item_style (appearance : Appearance.t) ~current =
  Style.merge
    [ style
        [ Background (Background.solid (token "background"))
        ; Foreground (token "foreground")
        ; Border_color (token "muted")
        ; Radius 6.
        ; Padding (px 6.)
        ; Min_width (px 0.)
        ]
    ; appearance.item_style
    ; (if current
       then
         Style.merge
           [ style
               [ Font_weight 700; Border_color (token "accent"); Border_bottom_width 3. ]
           ; appearance.current_style
           ]
       else Style.empty)
    ]
;;

let row ?key ?(custom = Style.empty) ~label children =
  View.row
    ?key
    ~style:
      (Style.merge
         [ style [ Align_items Center; Wrap Wrap; Gap (px 6.); Min_width (px 0.) ]
         ; custom
         ])
    children
  |> fun view -> annotate view ~role:Navigation ~label ()
;;

let breadcrumbs
      items
      ?key:root_key
      ?style:custom
      ?(appearance = Appearance.default)
      ~label
      ~current_description
      ~on_navigate
      ()
  =
  let open Or_error.Let_syntax in
  (* Validate even an empty path; adding its first member must not uncover an
     invalid localization string hidden in an earlier accepted configuration. *)
  let%bind _ = Accessibility.create ~description:current_description () in
  let members = Choice.Collection.to_list items in
  let count = List.length members in
  let%bind children =
    List.mapi members ~f:(fun index item ->
      let current = index = count - 1 in
      let child_style = item_style appearance ~current in
      let%map child =
        if current
        then
          annotate
            (View.text ~key:(key "item") ~style:child_style (Choice.label item))
            ~current:Location
            ~description:current_description
            ()
        else
          annotate
            (View.button
               ~key:(key "item")
               ~style:child_style
               ~disabled:(Choice.is_disabled item)
               ~on_click:(fun () -> on_navigate (Choice.id item))
               (Choice.label item))
            ~role:Link
            ()
      in
      let separator =
        View.column
          ~key:(key "separator")
          ~style:
            (Style.merge
               [ style
                   [ Height (px 12.)
                   ; Border_left_width 1.
                   ; Border_color (token "muted")
                   ; Shrink 0.
                   ]
               ; appearance.separator_style
               ])
          []
      in
      View.row
        ~key:(Choice.id item |> Choice.Id.to_string |> key)
        ~style:(style [ Align_items Center; Gap (px 6.); Min_width (px 0.) ])
        (if index = 0 then [ child ] else [ separator; child ]))
    |> Or_error.all
  in
  row ?key:root_key ?custom ~label children
;;

module Pagination_labels = struct
  type t =
    { navigation : string
    ; first : string
    ; previous : string
    ; next : string
    ; last : string
    ; current : string
    ; page : int -> string
    ; gap : first:int -> last:int -> string
    }

  let create ~navigation ~first ~previous ~next ~last ~current ~page ~gap =
    let%map.Or_error () =
      List.map [ navigation; first; previous; next; last; current ] ~f:(fun label ->
        Accessibility.create ~label () |> Or_error.map ~f:ignore)
      |> Or_error.all_unit
    in
    { navigation; first; previous; next; last; current; page; gap }
  ;;

  let english =
    create
      ~navigation:"Pagination"
      ~first:"First"
      ~previous:"Previous"
      ~next:"Next"
      ~last:"Last"
      ~current:"Current page"
      ~page:Int.to_string
      ~gap:(fun ~first ~last -> sprintf "Pages %d to %d" first last)
    |> Or_error.ok_exn
  ;;
end

let pagination
      model
      ?key:root_key
      ?style:custom
      ?(appearance = Appearance.default)
      ?(labels = Pagination_labels.english)
      ~on_request
      ()
  =
  let open Or_error.Let_syntax in
  let disabled = Pagination.is_disabled model in
  let current = Pagination.current model in
  let button ~id ~label ~request ~disabled ~is_current =
    let%bind metadata =
      Accessibility.create
        ~label
        ?current:(if is_current then Some Page else None)
        ?description:(if is_current then Some labels.current else None)
        ()
    in
    View.with_accessibility
      (View.button
         ~key:(key id)
         ~style:(item_style appearance ~current:is_current)
         ~disabled
         ~on_click:(fun () -> on_request request)
         label)
      metadata
  in
  let can_previous = Option.exists current ~f:(fun page -> page > 1) in
  let can_next =
    Option.exists current ~f:(fun page -> page < Pagination.total_pages model)
  in
  let boundary id label request can_move =
    button ~id ~label ~request ~disabled:(disabled || not can_move) ~is_current:false
  in
  let%bind first = boundary "first" labels.first Pagination.Request.first can_previous in
  let%bind previous =
    boundary "previous" labels.previous Pagination.Request.previous can_previous
  in
  let%bind next = boundary "next" labels.next Pagination.Request.next can_next in
  let%bind last = boundary "last" labels.last Pagination.Request.last can_next in
  let%map items =
    List.map (Pagination.items model) ~f:(function
      | Page page ->
        let%bind request = Pagination.Request.page page in
        button
          ~id:(sprintf "page-%d" page)
          ~label:(labels.page page)
          ~request
          ~disabled
          ~is_current:(Option.equal Int.equal current (Some page))
      | Gap { first; last } ->
        annotate
          (View.text
             ~key:(key (sprintf "gap-%d-%d" first last))
             ~style:
               (Style.merge
                  [ style [ Foreground (token "muted") ]; appearance.gap_style ])
             "…")
          ~label:(labels.gap ~first ~last)
          ())
    |> Or_error.all
  in
  (* Labels have already passed validation in the constructor. *)
  row
    ?key:root_key
    ?custom
    ~label:labels.navigation
    ([ first; previous ] @ items @ [ next; last ])
  |> Or_error.ok_exn
;;
