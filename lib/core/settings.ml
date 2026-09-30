open Core

let valid ~empty ~limit text =
  (empty || not (String.is_empty text))
  && String.length text <= limit
  && Stdlib.String.is_valid_utf_8 text
  && not (String.contains text '\000')
;;

let lowercase text =
  let buffer = Stdlib.Buffer.create (String.length text) in
  let rec loop offset =
    if offset < String.length text
    then (
      let decoded = Stdlib.String.get_utf_8_uchar text offset in
      let scalar = Stdlib.Uchar.utf_decode_uchar decoded in
      (match Uucp.Case.Map.to_lower scalar with
       | `Self -> Stdlib.Buffer.add_utf_8_uchar buffer scalar
       | `Uchars scalars -> List.iter scalars ~f:(Stdlib.Buffer.add_utf_8_uchar buffer));
      loop (offset + Stdlib.Uchar.utf_decode_length decoded))
  in
  loop 0;
  Stdlib.Buffer.contents buffer
;;

module type Id = sig
  type t [@@deriving equal, compare, sexp_of]

  val of_string : string -> t Or_error.t
  val to_string : t -> string
end

module Make_id () : Id = struct
  type t = string [@@deriving equal, compare, sexp_of]

  let of_string text =
    if valid ~empty:false ~limit:256 text
    then Ok text
    else Or_error.error_string "settings ID requires 1..256 UTF-8 bytes without NUL"
  ;;

  let to_string t = t
end

module Page_id = Make_id ()
module Group_id = Make_id ()
module Item_id = Make_id ()

module Query = struct
  type t =
    { source : string
    ; lowered : string
    }
  [@@deriving equal, sexp_of]

  let empty = { source = ""; lowered = "" }

  let of_string source =
    if valid ~empty:true ~limit:1024 source
    then Ok { source; lowered = lowercase source }
    else
      Or_error.error_string "settings query requires up to 1024 UTF-8 bytes without NUL"
  ;;

  let to_string t = t.source
end

module Reset = struct
  type t =
    | Unavailable
    | Clean
    | Dirty
  [@@deriving equal, sexp_of]
end

module Layout = struct
  type t =
    | Horizontal
    | Vertical
  [@@deriving equal, sexp_of]
end

let bytes strings = List.sum (module Int) strings ~f:String.length
let description_valid = Option.for_all ~f:(valid ~empty:true ~limit:16384)
let title_valid = valid ~empty:false ~limit:4096

module Item = struct
  type t =
    { id : Item_id.t
    ; title : string option
    ; description : string option
    ; keywords : string list
    ; layout : Layout.t
    ; disabled : bool
    ; reset : Reset.t
    ; search : string list
    ; text_bytes : int
    }
  [@@deriving equal, sexp_of]

  let make ~id ~title ~description ~keywords ~layout ~disabled ~reset =
    if
      not
        (Option.for_all title ~f:title_valid
         && description_valid description
         && List.length keywords <= 64
         && List.for_all keywords ~f:(valid ~empty:false ~limit:256))
    then Or_error.error_string "invalid settings item text or keywords"
    else (
      let source = Option.to_list title @ Option.to_list description @ keywords in
      let search = List.map source ~f:lowercase in
      if bytes search > 65536
      then Or_error.error_string "settings search text exceeds 64 KiB"
      else
        Ok
          { id
          ; title
          ; description
          ; keywords
          ; layout
          ; disabled
          ; reset
          ; search
          ; text_bytes =
              String.length (Item_id.to_string id) + bytes source + bytes search
          })
  ;;

  let create
        ~id
        ~title
        ?description
        ?(keywords = [])
        ?(layout = Layout.Horizontal)
        ?(disabled = false)
        ?(reset = Reset.Unavailable)
        ()
    =
    make ~id ~title:(Some title) ~description ~keywords ~layout ~disabled ~reset
  ;;

  let custom ~id ?(keywords = []) ?(disabled = false) ?(reset = Reset.Unavailable) () =
    make
      ~id
      ~title:None
      ~description:None
      ~keywords
      ~layout:Layout.Vertical
      ~disabled
      ~reset
  ;;

  let id t = t.id
  let title t = t.title
  let description t = t.description
  let keywords t = t.keywords
  let layout t = t.layout
  let is_disabled t = t.disabled
  let reset t = t.reset
  let with_disabled t disabled = { t with disabled }
  let with_reset t reset = { t with reset }

  let matches t query =
    String.is_empty query.Query.lowered
    || List.exists t.search ~f:(fun text ->
      String.is_substring text ~substring:query.lowered)
  ;;
end

module Group = struct
  type t =
    { id : Group_id.t
    ; title : string option
    ; description : string option
    ; items : Item.t list
    }
  [@@deriving equal, sexp_of]

  let create ~id ?title ?description items =
    if
      Option.for_all title ~f:title_valid
      && description_valid description
      && List.length items <= 32768
    then Ok { id; title; description; items }
    else Or_error.error_string "invalid settings group metadata"
  ;;

  let id t = t.id
  let title t = t.title
  let description t = t.description
  let items t = t.items
end

module Page = struct
  type t =
    { id : Page_id.t
    ; title : string
    ; description : string option
    ; default_open : bool
    ; resettable : bool
    ; groups : Group.t list
    }
  [@@deriving equal, sexp_of]

  let create ~id ~title ?description ?(default_open = false) ?(resettable = true) groups =
    if title_valid title && description_valid description && List.length groups <= 2048
    then Ok { id; title; description; default_open; resettable; groups }
    else Or_error.error_string "invalid settings page metadata"
  ;;

  let id t = t.id
  let title t = t.title
  let description t = t.description
  let groups t = t.groups
  let is_resettable t = t.resettable
end

module Selection = struct
  type t =
    { page : Page_id.t
    ; group : Group_id.t option
    }
  [@@deriving equal, sexp_of]

  let create ?group page = { page; group }
end

module Request = struct
  type t =
    | Search of Query.t
    | Select of Selection.t
    | Toggle_page of Page_id.t
  [@@deriving sexp_of]
end

module Reset_scope = struct
  type t =
    | Matching_page of Page_id.t
    | Whole_page of Page_id.t
    | Matching_group of Group_id.t
    | Item of Item_id.t
  [@@deriving sexp_of]
end

type t =
  { pages : Page.t list
  ; filtered : Page.t list
  ; items : Item.t String.Map.t
  ; query : Query.t
  ; preferred : Selection.t option
  ; expanded : Page_id.t list
  }

let equal left right =
  phys_equal left right
  || (List.equal Page.equal left.pages right.pages
      && Query.equal left.query right.query
      && Option.equal Selection.equal left.preferred right.preferred
      && List.equal Page_id.equal left.expanded right.expanded)
;;

let sexp_of_t t =
  [%sexp
    { pages = (t.pages : Page.t list)
    ; query = (t.query : Query.t)
    ; preferred = (t.preferred : Selection.t option)
    ; expanded = (t.expanded : Page_id.t list)
    }]
;;

let filter pages query =
  List.filter_map pages ~f:(fun page ->
    let groups =
      List.filter_map page.Page.groups ~f:(fun group ->
        let items =
          List.filter group.Group.items ~f:(fun item -> Item.matches item query)
        in
        Option.some_if (not (List.is_empty items)) { group with items })
    in
    Option.some_if (not (List.is_empty groups)) { page with groups })
;;

let find_page pages id = List.find pages ~f:(fun page -> Page_id.equal page.Page.id id)

let has_group page id =
  List.exists page.Page.groups ~f:(fun group -> Group_id.equal group.Group.id id)
;;

let valid_selection pages (selection : Selection.t) =
  Option.exists (find_page pages selection.page) ~f:(fun page ->
    Option.for_all selection.group ~f:(has_group page))
;;

let create ?selected pages =
  let open Or_error.Let_syntax in
  let%bind () =
    if
      List.length pages <= 128
      && List.sum (module Int) pages ~f:(fun page -> List.length (Page.groups page))
         <= 2048
    then Ok ()
    else Or_error.error_string "settings catalog exceeds page/group/item/text budget"
  in
  let groups = List.concat_map pages ~f:Page.groups in
  let%bind () =
    if
      List.sum (module Int) groups ~f:(fun group -> List.length (Group.items group))
      <= 32768
    then Ok ()
    else Or_error.error_string "settings catalog exceeds page/group/item/text budget"
  in
  let items = List.concat_map groups ~f:Group.items in
  let unique keys = not (List.contains_dup keys ~compare:String.compare) in
  let text_bytes =
    List.sum
      (module Int)
      pages
      ~f:(fun p ->
        bytes (Page_id.to_string p.Page.id :: p.title :: Option.to_list p.description))
    + List.sum
        (module Int)
        groups
        ~f:(fun g ->
          bytes
            ((Group_id.to_string g.Group.id :: Option.to_list g.title)
             @ Option.to_list g.description))
    + List.sum (module Int) items ~f:(fun i -> i.Item.text_bytes)
  in
  if text_bytes > 8_388_608
  then Or_error.error_string "settings catalog exceeds page/group/item/text budget"
  else if
    not
      (unique (List.map pages ~f:(fun p -> Page_id.to_string p.Page.id))
       && unique (List.map groups ~f:(fun g -> Group_id.to_string g.Group.id))
       && unique (List.map items ~f:(fun i -> Item_id.to_string i.Item.id)))
  then Or_error.error_string "duplicate settings page, group or item ID"
  else if not (Option.for_all selected ~f:(valid_selection pages))
  then Or_error.error_string "settings selection is absent from its page"
  else
    Ok
      { pages
      ; filtered = filter pages Query.empty
      ; items =
          String.Map.of_alist_exn
            (List.map items ~f:(fun i -> Item_id.to_string i.Item.id, i))
      ; query = Query.empty
      ; preferred = selected
      ; expanded =
          List.filter_map pages ~f:(fun p -> Option.some_if p.Page.default_open p.id)
      }
;;

let pages t = t.pages
let query t = t.query
let preferred_selection t = t.preferred
let filtered_pages t = t.filtered
let expanded_pages t = t.expanded
let find_item t id = Map.find t.items (Item_id.to_string id)

let selection t =
  match
    Option.bind t.preferred ~f:(fun selected ->
      Option.map (find_page t.filtered selected.page) ~f:(fun page ->
        { selected with group = Option.filter selected.group ~f:(has_group page) }))
  with
  | Some selected -> Some selected
  | None -> Option.map (List.hd t.filtered) ~f:(fun page -> Selection.create page.id)
;;

let select t selected =
  if valid_selection t.pages selected
  then Ok { t with preferred = Some selected }
  else Or_error.error_string "settings selection is absent from its page"
;;

let with_pages t pages =
  let%map.Or_error replacement = create pages in
  let preferred =
    Option.bind t.preferred ~f:(fun selected ->
      Option.map (find_page pages selected.page) ~f:(fun page ->
        { selected with group = Option.filter selected.group ~f:(has_group page) }))
  in
  let expanded =
    List.filter_map pages ~f:(fun page ->
      let expanded =
        match find_page t.pages page.Page.id with
        | None -> page.default_open
        | Some _ -> List.mem t.expanded page.id ~equal:Page_id.equal
      in
      Option.some_if expanded page.id)
  in
  { replacement with
    query = t.query
  ; filtered = filter pages t.query
  ; preferred
  ; expanded
  }
;;

let apply_request t = function
  | Request.Search query ->
    if Query.equal t.query query
    then t
    else { t with query; filtered = filter t.pages query }
  | Select selected ->
    if valid_selection t.filtered selected
    then { t with preferred = Some selected }
    else t
  | Toggle_page id ->
    if Option.is_none (find_page t.filtered id)
    then t
    else if List.mem t.expanded id ~equal:Page_id.equal
    then
      { t with
        expanded = List.filter t.expanded ~f:(fun other -> not (Page_id.equal id other))
      }
    else { t with expanded = t.expanded @ [ id ] }
;;

let reset_targets t ~scope =
  let selected = selection t in
  let pages =
    match scope with
    | Reset_scope.Matching_page _ | Matching_group _ -> t.filtered
    | Whole_page _ | Item _ -> t.pages
  in
  List.concat_map pages ~f:(fun page ->
    let included =
      match scope with
      | Reset_scope.Matching_page id ->
        Page_id.equal id page.Page.id
        && Option.exists selected ~f:(fun selected -> Page_id.equal selected.page id)
      | Whole_page id -> Page_id.equal id page.id
      | Matching_group _ ->
        Option.exists selected ~f:(fun selected -> Page_id.equal selected.page page.id)
      | Item _ -> true
    in
    if not (included && page.resettable)
    then []
    else
      List.concat_map page.groups ~f:(fun group ->
        let included =
          match scope with
          | Matching_group id -> Group_id.equal id group.Group.id
          | _ -> true
        in
        if not included
        then []
        else
          List.filter_map group.items ~f:(fun item ->
            let included =
              match scope with
              | Item id -> Item_id.equal id item.Item.id
              | _ -> true
            in
            Option.some_if
              (included && (not item.disabled) && Reset.equal item.reset Dirty)
              item.id)))
;;
