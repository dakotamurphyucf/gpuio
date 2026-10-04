open Core

let validate_text ~name ~empty ~max_bytes text =
  if ((not empty) && String.is_empty text) || String.length text > max_bytes
  then
    Or_error.errorf "%s must contain %d..%d bytes" name (if empty then 0 else 1) max_bytes
  else if (not (Stdlib.String.is_valid_utf_8 text)) || String.contains text '\000'
  then Or_error.errorf "%s must be UTF-8 without NUL" name
  else Ok ()
;;

module Group = struct
  module Id = struct
    type t = string [@@deriving equal, compare, sexp_of]

    let of_string text =
      let%map.Or_error () =
        validate_text ~name:"picker group id" ~empty:false ~max_bytes:256 text
      in
      text
    ;;

    let to_string t = t
  end

  type t =
    { id : Id.t
    ; label : string
    ; items : Choice.Collection.t
    }
  [@@deriving equal, sexp_of]

  let create ~id ~label items =
    let%map.Or_error () =
      validate_text ~name:"picker group label" ~empty:false ~max_bytes:1024 label
    in
    { id; label; items }
  ;;

  let id t = t.id
  let label t = t.label
  let items t = t.items
end

module Collection = struct
  type t =
    { groups : Group.t list option
    ; items : Choice.Collection.t
    }
  [@@deriving equal, sexp_of]

  let flat items = { groups = None; items }
  let groups t = t.groups
  let items t = Choice.Collection.to_list t.items
  let find t id = Choice.Collection.find t.items id

  let grouped groups =
    if List.length groups > 256
    then Or_error.error_string "picker exceeds 256 groups"
    else
      let open Or_error.Let_syntax in
      let%bind _, _, _ =
        List.fold_result
          groups
          ~init:(String.Set.empty, 0, 0)
          ~f:(fun (seen, count, bytes) group ->
            let id = Group.Id.to_string (Group.id group) in
            let items = Choice.Collection.to_list (Group.items group) in
            let count = count + List.length items in
            let bytes =
              bytes
              + String.length id
              + String.length (Group.label group)
              + List.sum
                  (module Int)
                  items
                  ~f:(fun item ->
                    String.length (Choice.Id.to_string (Choice.id item))
                    + String.length (Choice.label item))
            in
            if Set.mem seen id
            then Or_error.errorf "duplicate picker group id: %s" id
            else if count > Choice.Collection.max_choices
            then Or_error.error_string "picker exceeds 4096 items"
            else if bytes > Choice.Collection.max_text_bytes
            then Or_error.error_string "picker exceeds 262144 catalog text bytes"
            else Ok (Set.add seen id, count, bytes))
      in
      let%map items =
        Choice.Collection.create
          (List.concat_map groups ~f:(fun group ->
             Choice.Collection.to_list (Group.items group)))
      in
      { groups = Some groups; items }
  ;;
end

module Mode = struct
  type t =
    | Single
    | Multiple
  [@@deriving equal, sexp_of]
end

module Selection = struct
  type t =
    { mode : Mode.t
    ; ids : Choice.Id.t list
    ; by_id : String.Set.t
    }

  let equal a b = Mode.equal a.mode b.mode && List.equal Choice.Id.equal a.ids b.ids
  let sexp_of_t t = [%sexp (t.mode : Mode.t), (t.ids : Choice.Id.t list)]

  let make mode ids =
    { mode; ids; by_id = String.Set.of_list (List.map ids ~f:Choice.Id.to_string) }
  ;;

  let single id = make Single (Option.to_list id)

  let multiple ids =
    if List.length ids > Choice.Collection.max_choices
    then Or_error.error_string "picker selection exceeds 4096 IDs"
    else (
      let t = make Mode.Multiple ids in
      if Set.length t.by_id <> List.length ids
      then Or_error.error_string "duplicate picker selection ID"
      else Ok t)
  ;;

  let mode t = t.mode
  let ids t = t.ids
  let mem t id = Set.mem t.by_id (Choice.Id.to_string id)
end

module Search = struct
  type t =
    | None
    | Substring
    | Application
  [@@deriving equal, sexp_of]
end

module Open_state = struct
  type t =
    | Managed of { initially_open : bool }
    | Controlled of bool
  [@@deriving equal, sexp_of]
end

module Request = struct
  type t =
    | Select of Choice.Id.t
    | Toggle of Choice.Id.t
    | Clear
  [@@deriving equal, sexp_of]
end

module Config = struct
  type t =
    { label : string
    ; options : Collection.t
    ; selected : Selection.t
    ; disabled : bool
    ; search : Search.t
    ; clearable : bool
    ; open_state : Open_state.t
    ; placeholder : string
    ; search_placeholder : string
    }
  [@@deriving equal, sexp_of]

  let create
        ~label
        ~options
        ~selected
        ?(disabled = false)
        ?(search = Search.None)
        ?(clearable = false)
        ?(open_state = Open_state.Managed { initially_open = false })
        ?(placeholder = "")
        ?(search_placeholder = "")
        ()
    =
    let open Or_error.Let_syntax in
    let%bind () = validate_text ~name:"picker label" ~empty:false ~max_bytes:1024 label in
    let%bind () =
      validate_text ~name:"picker placeholder" ~empty:true ~max_bytes:1024 placeholder
    in
    let%bind () =
      validate_text
        ~name:"picker search placeholder"
        ~empty:true
        ~max_bytes:1024
        search_placeholder
    in
    let%map () =
      List.fold_result (Selection.ids selected) ~init:() ~f:(fun () id ->
        match Collection.find options id with
        | Some _ -> Ok ()
        | None ->
          Or_error.errorf "selected picker ID is absent: %s" (Choice.Id.to_string id))
    in
    { label
    ; options
    ; selected
    ; disabled
    ; search
    ; clearable
    ; open_state
    ; placeholder
    ; search_placeholder
    }
  ;;

  let label t = t.label
  let options t = t.options
  let selected t = t.selected
  let is_disabled t = t.disabled
  let search t = t.search
  let is_clearable t = t.clearable
  let open_state t = t.open_state
  let placeholder t = t.placeholder
  let search_placeholder t = t.search_placeholder

  let can_select t id =
    Option.value_map (Collection.find t.options id) ~default:false ~f:(fun item ->
      not (Choice.is_disabled item))
  ;;

  let apply_request t request =
    if t.disabled
    then t.selected
    else (
      match request, Selection.mode t.selected with
      | Request.Select id, Single when can_select t id -> Selection.single (Some id)
      | Toggle id, Multiple when can_select t id ->
        let ids =
          if Selection.mem t.selected id
          then
            List.filter (Selection.ids t.selected) ~f:(fun other ->
              not (Choice.Id.equal id other))
          else Selection.ids t.selected @ [ id ]
        in
        Selection.make Multiple ids
      | Clear, mode when t.clearable -> Selection.make mode []
      | (Select _ | Toggle _ | Clear), (Single | Multiple) -> t.selected)
  ;;
end

module Open_reason = struct
  type t =
    | Trigger
    | Keyboard
    | Escape
    | Outside_pointer
    | Focus_left
    | Selection
  [@@deriving equal, sexp_of]
end

module Visibility_reason = struct
  type t =
    | Interaction of Open_reason.t
    | Application
    | Unavailable
  [@@deriving equal, sexp_of]
end

module Visibility = struct
  type t =
    | Snapshot of bool
    | Changed of bool * Visibility_reason.t
  [@@deriving equal, sexp_of]
end

module Selection_request = struct
  type t =
    { request : Request.t
    ; query : Text_input.Snapshot.t option
    }
  [@@deriving equal, sexp_of]

  let request t = t.request
  let query t = t.query
end

module Event = struct
  type t =
    | Selection_requested of Selection_request.t
    | Open_requested of bool * Open_reason.t
    | Visibility of Visibility.t
    | Query_changed of Text_input.Snapshot.t
  [@@deriving equal, sexp_of]
end

module Query = struct
  type t =
    { controller : Key.t
    ; initial_text : string
    }
  [@@deriving equal, sexp_of]

  let create ~controller ?(initial_text = "") () =
    let%map.Or_error () = Text_input.validate_text ~mode:Single_line initial_text in
    { controller; initial_text }
  ;;

  let controller t = t.controller
  let initial_text t = t.initial_text
end

module Checkmark = struct
  type t =
    | Native
    | Custom
  [@@deriving equal, sexp_of]
end

module Option_content = struct
  type 'a t =
    { content : 'a
    ; checkmark : Checkmark.t
    }

  let create ?(checkmark = Checkmark.Native) content = { content; checkmark }
  let content t = t.content
  let checkmark t = t.checkmark
end

module Appearance = struct
  type t =
    { parts : Choice.Appearance.t
    ; max_height : float
    ; overscan : float
    ; header_style : Style.t
    }
  [@@deriving equal, sexp_of]

  let create
        ?popup_width
        ?(max_height = 320.)
        ?(estimated_row_height = 32.)
        ?(overscan = 64.)
        ?empty_label
        ?(popup_style = Style.empty)
        ?(option_style = Style.empty)
        ?(header_style = Style.empty)
        ?(empty_style = Style.empty)
        ()
    =
    let open Or_error.Let_syntax in
    let%bind () =
      if
        Float.is_finite max_height
        && Float.(max_height > 0. && max_height <= 1_000_000.)
        && Float.is_finite estimated_row_height
        && Float.(estimated_row_height >= 1. && estimated_row_height <= 1_000_000.)
        && Float.is_finite overscan
        && Float.(overscan >= 0. && overscan <= 4096.)
      then Ok ()
      else Or_error.error_string "invalid picker appearance geometry"
    in
    let%bind parts =
      Choice.Appearance.create
        ?popup_width
        ~row_height:estimated_row_height
        ~max_visible_rows:1
        ?empty_label
        ~popup_style
        ~option_style
        ~empty_style
        ()
    in
    let%bind (_ : Choice.Appearance.t) =
      Choice.Appearance.create ~empty_style:header_style ()
    in
    let%map () =
      if
        List.sum
          (module Int)
          [ popup_style; option_style; header_style; empty_style ]
          ~f:Style.Expert.declaration_count
        <= 128
      then Ok ()
      else Or_error.error_string "picker appearance exceeds 128 declarations"
    in
    { parts; max_height; overscan; header_style }
  ;;

  let default = create () |> Or_error.ok_exn
end

module Description = struct
  type 'a t =
    { config : Config.t
    ; appearance : Appearance.t
    ; query : Query.t option
    ; trigger : 'a option
    ; empty : 'a option
    ; footer : 'a option
    ; groups : (Group.Id.t * 'a) list
    ; options : (Choice.Id.t * 'a Option_content.t) list
    }

  let create
        ~config
        ?(appearance = Appearance.default)
        ?query
        ?trigger
        ?empty
        ?footer
        ?(groups = [])
        ?(options = [])
        ()
    =
    let open Or_error.Let_syntax in
    let%bind () =
      if
        Bool.equal (Option.is_some query) (not (Search.equal (Config.search config) None))
      then Ok ()
      else Or_error.error_string "picker query placement must match search mode"
    in
    let%bind () =
      if List.length groups <= 256 && List.length options <= 4096
      then Ok ()
      else Or_error.error_string "too many picker content overrides"
    in
    let group_ids =
      Collection.groups (Config.options config)
      |> Option.value ~default:[]
      |> List.map ~f:(fun group -> Group.Id.to_string (Group.id group))
      |> String.Set.of_list
    in
    let%bind (_ : String.Set.t) =
      List.fold_result groups ~init:String.Set.empty ~f:(fun seen (id, _) ->
        let id = Group.Id.to_string id in
        if Set.mem seen id || not (Set.mem group_ids id)
        then Or_error.error_string "picker group override is duplicate or absent"
        else Ok (Set.add seen id))
    in
    let%map (_ : String.Set.t) =
      List.fold_result options ~init:String.Set.empty ~f:(fun seen (id, _) ->
        let name = Choice.Id.to_string id in
        if
          Set.mem seen name || Option.is_none (Collection.find (Config.options config) id)
        then Or_error.error_string "picker option override is duplicate or absent"
        else Ok (Set.add seen name))
    in
    { config; appearance; query; trigger; empty; footer; groups; options }
  ;;

  let config t = t.config
  let appearance t = t.appearance
  let query t = t.query
  let trigger t = t.trigger
  let empty t = t.empty
  let footer t = t.footer
  let groups t = t.groups
  let options t = t.options
end

module Expert = struct
  let accepts_event config = function
    | Event.Query_changed _ -> true
    | Visibility (Snapshot open_ | Changed (open_, _)) ->
      (not open_) || not (Config.is_disabled config)
    | Open_requested _ -> not (Config.is_disabled config)
    | Selection_requested selection ->
      (not (Config.is_disabled config))
      &&
        (match
           Selection_request.request selection, Selection.mode (Config.selected config)
         with
        | Request.Clear, _ -> Config.is_clearable config
        | Select id, Single | Toggle id, Multiple -> Config.can_select config id
        | Select _, Multiple | Toggle _, Single -> false)
  ;;

  module W = Gpuio_protocol.Choice_picker_wire

  let open_reason_of_wire : W.Open_reason.t -> Open_reason.t = function
    | Trigger -> Trigger
    | Keyboard -> Keyboard
    | Escape -> Escape
    | Outside_pointer -> Outside_pointer
    | Focus_left -> Focus_left
    | Selection -> Selection
  ;;

  let visibility_reason_of_wire : W.Visibility_reason.t -> Visibility_reason.t = function
    | Interaction reason -> Interaction (open_reason_of_wire reason)
    | Application -> Application
    | Unavailable -> Unavailable
  ;;

  let event_of_wire (wire : W.Event.t) ~window ~query_node =
    if not (W.Event.valid wire)
    then Or_error.error_string "invalid choice picker event"
    else
      let open Or_error.Let_syntax in
      let query_of_wire (query : W.Query.t) =
        if not (Option.equal Gpuio_protocol.Node_id.equal query_node (Some query.node))
        then Or_error.error_string "stale picker query editor"
        else Text_input.Expert.snapshot_of_wire ~window ~node:query.node query.snapshot
      in
      match wire with
      | Selection_requested (request, query) ->
        let%bind query =
          match query, query_node with
          | None, None -> Ok None
          | None, Some _ -> Or_error.error_string "missing picker query snapshot"
          | Some query, (None | Some _) ->
            let%map query = query_of_wire query in
            Some query
        in
        let%map request =
          match request with
          | Select id ->
            let%map id = Choice.Id.of_string id in
            Request.Select id
          | Toggle id ->
            let%map id = Choice.Id.of_string id in
            Request.Toggle id
          | Clear -> Ok Request.Clear
        in
        Event.Selection_requested { Selection_request.request; query }
      | Open_requested (open_, reason) ->
        Ok (Event.Open_requested (open_, open_reason_of_wire reason))
      | Visibility (Snapshot open_) -> Ok (Event.Visibility (Snapshot open_))
      | Visibility (Changed (open_, reason)) ->
        Ok (Event.Visibility (Changed (open_, visibility_reason_of_wire reason)))
      | Query_changed query ->
        let%map query = query_of_wire query in
        Event.Query_changed query
  ;;

  let item_to_wire item : W.Item.t =
    { id = Choice.Id.to_string (Choice.id item)
    ; label = Choice.label item
    ; disabled = Choice.is_disabled item
    }
  ;;

  let to_wire (t : Config.t) : W.Config.t =
    let options =
      match Collection.groups t.options with
      | None -> W.Collection.Flat (List.map (Collection.items t.options) ~f:item_to_wire)
      | Some groups ->
        Grouped
          (List.map groups ~f:(fun group ->
             { W.Group.id = Group.Id.to_string (Group.id group)
             ; label = Group.label group
             ; items =
                 List.map (Choice.Collection.to_list (Group.items group)) ~f:item_to_wire
             }))
    in
    let ids = List.map (Selection.ids t.selected) ~f:Choice.Id.to_string in
    { label = t.label
    ; options
    ; selected =
        (match Selection.mode t.selected with
         | Single -> W.Selection.Single (List.hd ids)
         | Multiple -> Multiple ids)
    ; disabled = t.disabled
    ; search =
        (match t.search with
         | None -> W.Search.None
         | Substring -> Substring
         | Application -> Application)
    ; clearable = t.clearable
    ; open_state =
        (match t.open_state with
         | Managed { initially_open } -> W.Open_state.Managed initially_open
         | Controlled value -> Controlled value)
    ; placeholder = t.placeholder
    ; search_placeholder = t.search_placeholder
    }
  ;;

  let description_to_wire (t : 'a Description.t) ~theme =
    let open Or_error.Let_syntax in
    let%bind parts = Choice.Expert.appearance_to_wire t.appearance.parts ~theme in
    let%map header_style = Style.Expert.to_wire t.appearance.header_style ~theme in
    let slots =
      Option.to_list (Option.map t.trigger ~f:(fun _ -> W.Slot.Trigger))
      @ Option.to_list (Option.map t.query ~f:(fun _ -> W.Slot.Query))
      @ Option.to_list (Option.map t.empty ~f:(fun _ -> W.Slot.Empty))
      @ Option.to_list (Option.map t.footer ~f:(fun _ -> W.Slot.Footer))
      @ List.map t.groups ~f:(fun (id, _) -> W.Slot.Group (Group.Id.to_string id))
      @ List.map t.options ~f:(fun (id, content) ->
        W.Slot.Option
          ( Choice.Id.to_string id
          , match Option_content.checkmark content with
            | Native -> W.Checkmark.Native
            | Custom -> Custom ))
    in
    ({ config = to_wire t.config
     ; popup_width = parts.popup_width
     ; max_height = t.appearance.max_height
     ; estimated_row_height = parts.row_height
     ; overscan = t.appearance.overscan
     ; empty_label = parts.empty_label
     ; popup_style = parts.popup_style
     ; option_style = parts.option_style
     ; header_style
     ; empty_style = parts.empty_style
     ; slots
     }
     : Gpuio_protocol.Wire.Choice_picker_presentation.t)
  ;;

  let items_of_wire items =
    let open Or_error.Let_syntax in
    let%bind items =
      List.map items ~f:(fun (item : W.Item.t) ->
        let%bind id = Choice.Id.of_string item.id in
        Choice.create ~id ~label:item.label ~disabled:item.disabled ())
      |> Or_error.all
    in
    Choice.Collection.create items
  ;;

  let of_wire (wire : W.Config.t) =
    if not (W.Config.valid wire)
    then Or_error.error_string "invalid choice picker configuration"
    else
      let open Or_error.Let_syntax in
      let%bind options =
        match wire.options with
        | Flat items ->
          let%map items = items_of_wire items in
          Collection.flat items
        | Grouped groups ->
          let%bind groups =
            List.map groups ~f:(fun (group : W.Group.t) ->
              let%bind id = Group.Id.of_string group.id in
              let%bind items = items_of_wire group.items in
              Group.create ~id ~label:group.label items)
            |> Or_error.all
          in
          Collection.grouped groups
      in
      let%bind selected =
        match wire.selected with
        | Single None -> Ok (Selection.single None)
        | Single (Some id) ->
          let%map id = Choice.Id.of_string id in
          Selection.single (Some id)
        | Multiple ids ->
          let%bind ids = List.map ids ~f:Choice.Id.of_string |> Or_error.all in
          Selection.multiple ids
      in
      Config.create
        ~label:wire.label
        ~options
        ~selected
        ~disabled:wire.disabled
        ~search:
          (match wire.search with
           | None -> Search.None
           | Substring -> Substring
           | Application -> Application)
        ~clearable:wire.clearable
        ~open_state:
          (match wire.open_state with
           | Managed initially_open -> Open_state.Managed { initially_open }
           | Controlled value -> Controlled value)
        ~placeholder:wire.placeholder
        ~search_placeholder:wire.search_placeholder
        ()
  ;;
end
