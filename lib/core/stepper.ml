open Core

module Status = struct
  type t =
    | Completed
    | Current
    | Upcoming
  [@@deriving equal, sexp_of]
end

module Request = struct
  type t =
    | Select of Choice.Id.t
    | Previous
    | Next
  [@@deriving equal, sexp_of]
end

type t =
  { steps : Choice.Collection.t
  ; current : Choice.Id.t option
  ; disabled : bool
  }
[@@deriving equal, sexp_of]

let max_steps = 64

let validate_steps steps =
  if List.length (Choice.Collection.to_list steps) > max_steps
  then Or_error.error_string "workflow stepper supports at most 64 steps"
  else if
    List.exists (Choice.Collection.to_list steps) ~f:(fun item ->
      let label = Choice.label item in
      String.length label > 1024 || String.is_empty (String.strip label))
  then Or_error.error_string "workflow step labels must be nonblank and <=1024 bytes"
  else Ok ()
;;

let create ~steps ~current ?(disabled = false) () =
  let open Or_error.Let_syntax in
  let%bind () = validate_steps steps in
  let%map () = Choice.Collection.validate_selection steps current in
  { steps; current; disabled }
;;

let steps t = t.steps
let current t = t.current
let is_disabled t = t.disabled
let with_disabled t disabled = { t with disabled }

let index items id =
  List.find_mapi items ~f:(fun index item ->
    if Choice.Id.equal (Choice.id item) id then Some index else None)
;;

let status_at current index =
  match current with
  | Some current when index < current -> Status.Completed
  | Some current when index = current -> Current
  | Some _ | None -> Upcoming
;;

let status t id =
  let items = Choice.Collection.to_list t.steps in
  let current = Option.bind t.current ~f:(index items) in
  Option.map (index items id) ~f:(status_at current)
;;

let with_steps t steps =
  let current =
    Option.filter t.current ~f:(fun id ->
      Option.is_some (Choice.Collection.find steps id))
  in
  create ~steps ~current ~disabled:t.disabled ()
;;

let select t current =
  let%map.Or_error () = Choice.Collection.validate_selection t.steps current in
  { t with current }
;;

let apply_request t request =
  if t.disabled
  then t
  else (
    let items = Choice.Collection.to_list t.steps in
    let candidate =
      match request with
      | Request.Select id -> Choice.Collection.find t.steps id
      | Previous | Next ->
        let indexed = List.mapi items ~f:(fun index item -> index, item) in
        let selected = Option.bind t.current ~f:(index items) in
        let eligible =
          List.filter indexed ~f:(fun (index, item) ->
            (not (Choice.is_disabled item))
            &&
            match request, selected with
            | Previous, Some current -> index < current
            | Next, Some current -> index > current
            | (Previous | Next), None -> true
            | Select _, _ -> false)
        in
        (match request with
         | Previous -> List.last eligible
         | Next -> List.hd eligible
         | Select _ -> None)
        |> Option.map ~f:snd
    in
    match candidate with
    | Some item when not (Choice.is_disabled item) ->
      { t with current = Some (Choice.id item) }
    | Some _ | None -> t)
;;

module Axis = struct
  type t =
    | Horizontal
    | Vertical
  [@@deriving equal, sexp_of]
end

module Appearance = struct
  type t =
    { indicator_size : float
    ; connector_thickness : float
    ; gap : float
    ; item_style : Style.t
    ; indicator_style : Style.t
    ; completed_style : Style.t
    ; current_style : Style.t
    ; upcoming_style : Style.t
    ; connector_style : Style.t
    }

  let create
        ?(indicator_size = 28.)
        ?(connector_thickness = 2.)
        ?(gap = 8.)
        ?(item_style = Style.empty)
        ?(indicator_style = Style.empty)
        ?(completed_style = Style.empty)
        ?(current_style = Style.empty)
        ?(upcoming_style = Style.empty)
        ?(connector_style = Style.empty)
        ()
    =
    let valid n = Float.is_finite n && Float.(n >= 0. && n <= 4096.) in
    if
      (not (List.for_all [ indicator_size; connector_thickness; gap ] ~f:valid))
      || Float.(indicator_size = 0. || connector_thickness = 0.)
    then Or_error.error_string "invalid workflow stepper dimensions"
    else
      Ok
        { indicator_size
        ; connector_thickness
        ; gap
        ; item_style
        ; indicator_style
        ; completed_style
        ; current_style
        ; upcoming_style
        ; connector_style
        }
  ;;

  let default = create () |> Or_error.ok_exn
end

module Labels = struct
  type t = { description : index:int -> count:int -> Status.t -> string }

  let create ~description = { description }

  let english =
    create ~description:(fun ~index ~count status ->
      let status =
        match status with
        | Status.Completed -> "Completed step"
        | Current -> "Current step"
        | Upcoming -> "Upcoming step"
      in
      sprintf "Step %d of %d · %s" index count status)
  ;;
end

let view
      t
      ?key
      ?(style = Style.empty)
      ?(axis = Axis.Horizontal)
      ?(centered = false)
      ?(appearance = Appearance.default)
      ?(labels = Labels.english)
      ?indicator
      ?(content = fun item _ -> View.text (Choice.label item))
      ~label
      ~on_request
      ()
  =
  let open Or_error.Let_syntax in
  let open Style.Property in
  let px = Length.px_exn in
  let styles = Style.create_exn in
  let token = Color.token_exn in
  let node_key = Key.of_string_exn in
  let%bind navigation = Accessibility.create ~role:Navigation ~label () in
  let items = Choice.Collection.to_list t.steps in
  let count = List.length items in
  let selected = Option.bind t.current ~f:(index items) in
  let%bind children =
    List.mapi items ~f:(fun index item ->
      let status = status_at selected index in
      let is_current = Status.equal status Current in
      let is_completed = Status.equal status Completed in
      let%bind metadata =
        Accessibility.create
          ~description:(labels.description ~index:(index + 1) ~count status)
          ?current:(if is_current then Some Step else None)
          ()
      in
      let glyph =
        match indicator with
        | None -> View.text (Int.to_string (index + 1))
        | Some render -> render item status
      in
      let state_style =
        match status with
        | Completed -> appearance.completed_style
        | Current -> appearance.current_style
        | Upcoming -> appearance.upcoming_style
      in
      let badge =
        View.column
          ~key:(node_key "indicator")
          ~style:
            (Style.merge
               [ styles
                   [ Width (px appearance.indicator_size)
                   ; Height (px appearance.indicator_size)
                   ; Shrink 0.
                   ; Radius (appearance.indicator_size /. 2.)
                   ; Align_items Center
                   ; Justify_content Center
                   ; Background (Background.solid (token "background"))
                   ; Border_width (if is_current then 2. else 1.)
                   ; Border_color
                       (token (if is_current || is_completed then "accent" else "muted"))
                   ; Foreground
                       (token
                          (if is_current || is_completed then "accent" else "foreground"))
                   ]
               ; appearance.indicator_style
               ; state_style
               ])
          [ glyph ]
      in
      let body = View.column ~key:(node_key "label") [ content item status ] in
      let layout =
        match axis with
        | Horizontal -> View.column
        | Vertical -> View.row
      in
      let%bind button =
        View.button_with_content
          ~key:(node_key "trigger")
          ~style:
            (Style.merge
               [ (styles [ Padding (px 6.); Radius 6.; Min_width (px 0.) ]
                  |> fun s -> Style.with_state_exn s Disabled [ Opacity 0.5 ])
               ; appearance.item_style
               ])
          ~disabled:(t.disabled || Choice.is_disabled item)
          ~accessible_name:(Choice.label item)
          ~on_click:(fun () -> on_request (Request.Select (Choice.id item)))
          (layout
             ~style:
               (styles
                  [ Gap (px appearance.gap)
                  ; Min_width (px 0.)
                  ; Align_items
                      (if centered || Axis.equal axis Vertical then Center else Start)
                  ])
             [ badge; body ])
      in
      let%map button = View.with_accessibility button metadata in
      let connector =
        let dimensions =
          match axis with
          | Horizontal ->
            [ Grow 1.
            ; Min_width (px 16.)
            ; Height (px appearance.connector_thickness)
            ; Margin_top (px ((appearance.indicator_size /. 2.) +. 6.))
            ; Align_self Start
            ]
          | Vertical ->
            [ Width (px appearance.connector_thickness)
            ; Height (px (Float.max 16. (appearance.gap *. 2.)))
            ; Margin_left (px ((appearance.indicator_size /. 2.) +. 6.))
            ]
        in
        View.column
          ~key:(node_key "connector")
          ~style:
            (Style.merge
               [ styles
                   (Background
                      (Background.solid
                         (token (if is_completed then "accent" else "muted")))
                    :: dimensions)
               ; appearance.connector_style
               ])
          []
      in
      let layout =
        match axis with
        | Horizontal -> View.row
        | Vertical -> View.column
      in
      layout
        ~key:(Choice.Id.to_string (Choice.id item) |> node_key)
        ~style:
          (styles
             [ Min_width (px 0.)
             ; Gap (px appearance.gap)
             ; Grow (if Axis.equal axis Horizontal && index < count - 1 then 1. else 0.)
             ])
        (if index = count - 1 then [ button ] else [ button; connector ]))
    |> Or_error.all
  in
  let layout =
    match axis with
    | Horizontal -> View.row
    | Vertical -> View.column
  in
  View.with_accessibility
    (layout
       ?key
       ~style:
         (Style.merge [ styles [ Min_width (px 0.); Gap (px appearance.gap) ]; style ])
       children)
    navigation
;;
