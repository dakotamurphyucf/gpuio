open Core
module Diff = Document_diff
module Preview = Document_preview
module Style = Document_style
module Activation = Document_activation
module Markdown_options = Document_markdown_options
module Actions = Document_actions
module Profile = Document_profile

module Language = struct
  type t = string [@@deriving equal, sexp_of]

  let plain_text = "txt"
  let ocaml = "ml"
  let rust = "rs"
  let shell = "sh"
  let json = "json"
  let python = "py"
  let javascript = "js"
  let typescript = "ts"
  let yaml = "yaml"
  let toml = "toml"
  let ini = "ini"

  let of_string text =
    if
      String.is_empty text
      || String.length text > 64
      || not
           (String.for_all text ~f:(fun c -> Char.is_alphanum c || String.mem "_+-.#" c))
    then Or_error.error_string "syntax token must contain 1..64 ASCII identifier bytes"
    else Ok (String.lowercase text)
  ;;

  let to_string t = t
end

module Mode = struct
  type t =
    | Markdown
    | Code of Language.t
    | Diff
    | Html
  [@@deriving equal, sexp_of]
end

module Appearance = struct
  type t =
    | Light
    | Dark
  [@@deriving equal, sexp_of]
end

module Layout = struct
  type t =
    | Flow
    | Viewport of float
  [@@deriving equal, sexp_of]
end

module Navigation = struct
  module Side = struct
    type t =
      | Before
      | After
    [@@deriving equal, sexp_of]
  end

  type t =
    | Link of
        { url : string
        ; activation : Activation.t option
        }
    | Line of
        { path : string option
        ; side : Side.t
        ; line : int
        }
  [@@deriving equal, sexp_of]
end

module Selection_format = struct
  type t =
    | Plain_text
    | Markdown
  [@@deriving equal, sexp_of]
end

module Setting = struct
  type 'a t =
    | Inherit
    | Builtin
    | Value of 'a
  [@@deriving equal, sexp_of]

  let of_option = function
    | None -> Inherit
    | Some value -> Value value
  ;;

  let resolve t ~inherited ~builtin =
    match t with
    | Inherit -> inherited
    | Builtin -> builtin
    | Value value -> value
  ;;

  let resolve_optional t ~inherited =
    match t with
    | Inherit -> inherited
    | Builtin -> None
    | Value value -> Some value
  ;;
end

module Overrides = struct
  type t =
    { appearance : Appearance.t Setting.t
    ; layout : Layout.t Setting.t
    ; line_numbers : bool Setting.t
    ; initially_collapsed : bool Setting.t
    ; selection_format : Selection_format.t Setting.t
    ; max_lines : int Setting.t
    ; text_style : Style.t Setting.t
    ; actions : Actions.Config.t Setting.t
    ; markdown_options : Markdown_options.t Setting.t
    }
  [@@deriving equal, sexp_of]

  let create
        ?appearance
        ?layout
        ?line_numbers
        ?initially_collapsed
        ?selection_format
        ?max_lines
        ?text_style
        ?actions
        ?markdown_options
        ()
    =
    { appearance = Setting.of_option appearance
    ; layout = Setting.of_option layout
    ; line_numbers = Setting.of_option line_numbers
    ; initially_collapsed = Setting.of_option initially_collapsed
    ; selection_format = Setting.of_option selection_format
    ; max_lines = Setting.of_option max_lines
    ; text_style = Setting.of_option text_style
    ; actions = Setting.of_option actions
    ; markdown_options = Setting.of_option markdown_options
    }
  ;;

  let empty = create ()
end

module Config = struct
  type t =
    { source : Text_source.Handle.t
    ; mode : Mode.t
    ; appearance : Appearance.t
    ; layout : Layout.t
    ; label : string
    ; path : string option
    ; line_numbers : bool
    ; initially_collapsed : bool
    ; search : string
    ; images : (string * Asset.Handle.t) list
    ; diff : Diff.Config.t option
    ; selection_format : Selection_format.t
    ; max_lines : int option
    ; text_style : Style.t option
    ; actions : Actions.Config.t
    ; markdown_options : Markdown_options.t
    ; overrides : Overrides.t
    }
  [@@deriving equal, sexp_of]

  let valid_text text ~max =
    String.length text <= max
    && Stdlib.String.is_valid_utf_8 text
    && not (String.contains text '\000')
  ;;

  let create
        ~source
        ~mode
        ?appearance
        ?layout
        ?(label = "Document")
        ?path
        ?line_numbers
        ?initially_collapsed
        ?(search = "")
        ?(images = [])
        ?diff
        ?selection_format
        ?max_lines
        ?text_style
        ?actions
        ?markdown_options
        ()
    =
    let overrides =
      Overrides.create
        ?appearance
        ?layout
        ?line_numbers
        ?initially_collapsed
        ?selection_format
        ?max_lines
        ?text_style
        ?actions
        ?markdown_options
        ()
    in
    let appearance = Option.value appearance ~default:Appearance.Light in
    let layout = Option.value layout ~default:Layout.Flow in
    let line_numbers = Option.value line_numbers ~default:true in
    let initially_collapsed = Option.value initially_collapsed ~default:false in
    let selection_format =
      Option.value selection_format ~default:Selection_format.Plain_text
    in
    let actions = Option.value actions ~default:Actions.Config.default in
    let markdown_options =
      Option.value markdown_options ~default:Markdown_options.default
    in
    let valid_layout =
      match layout with
      | Flow -> true
      | Viewport height ->
        Float.is_finite height && Float.(height >= 1. && height <= 16384.)
    in
    if
      (not (Actions.Config.equal actions Actions.Config.default))
      && not (Mode.equal mode Markdown || Mode.equal mode Html)
    then Or_error.error_string "document actions require Markdown or HTML"
    else if
      (not (Markdown_options.equal markdown_options Markdown_options.default))
      && not (Mode.equal mode Markdown)
    then Or_error.error_string "Markdown parser options require Markdown mode"
    else if
      Option.is_some text_style && not (Mode.equal mode Markdown || Mode.equal mode Html)
    then Or_error.error_string "document text style requires Markdown or HTML"
    else if
      Option.is_some max_lines
      && ((not
             ((Mode.equal mode Markdown || Mode.equal mode Html)
              && Layout.equal layout Flow))
          || not (Option.for_all max_lines ~f:(fun n -> n >= 1 && n <= 4096)))
    then
      Or_error.error_string
        "document preview requires Markdown/HTML Flow and max_lines in 1..4096"
    else if Option.is_some diff && not (Mode.equal mode Mode.Diff)
    then Or_error.error_string "diff controls require Document.Mode.Diff"
    else if not valid_layout
    then Or_error.error_string "document viewport height must be finite and in 1..16384"
    else if
      String.is_empty label
      || (not (valid_text label ~max:1024))
      || (not (Option.for_all path ~f:(fun text -> valid_text text ~max:4096)))
      || not (valid_text search ~max:4096)
    then Or_error.error_string "invalid document label, path or search text"
    else if
      List.length images > 128
      || (not
            (List.for_all images ~f:(fun (url, _) ->
               (not (String.is_empty url)) && valid_text url ~max:4096)))
      || Set.length (String.Set.of_list (List.map images ~f:fst)) <> List.length images
    then
      Or_error.error_string "document image map requires at most128 unique bounded URLs"
    else
      Ok
        { source
        ; mode
        ; appearance
        ; layout
        ; label
        ; path
        ; line_numbers
        ; initially_collapsed
        ; search
        ; images
        ; diff
        ; selection_format
        ; max_lines
        ; text_style
        ; actions
        ; markdown_options
        ; overrides
        }
  ;;

  let resolve t ~(defaults : Overrides.t) =
    let rich = Mode.equal t.mode Markdown || Mode.equal t.mode Html in
    let appearance =
      let inherited =
        if true
        then
          Setting.resolve
            defaults.appearance
            ~inherited:Appearance.Light
            ~builtin:Appearance.Light
        else Appearance.Light
      in
      Setting.resolve t.overrides.appearance ~inherited ~builtin:Appearance.Light
    in
    let layout =
      let inherited =
        if true
        then Setting.resolve defaults.layout ~inherited:Layout.Flow ~builtin:Layout.Flow
        else Layout.Flow
      in
      Setting.resolve t.overrides.layout ~inherited ~builtin:Layout.Flow
    in
    let line_numbers =
      let inherited =
        if true
        then Setting.resolve defaults.line_numbers ~inherited:true ~builtin:true
        else true
      in
      Setting.resolve t.overrides.line_numbers ~inherited ~builtin:true
    in
    let initially_collapsed =
      let inherited =
        if true
        then Setting.resolve defaults.initially_collapsed ~inherited:false ~builtin:false
        else false
      in
      Setting.resolve t.overrides.initially_collapsed ~inherited ~builtin:false
    in
    let selection_format =
      let inherited =
        if true
        then
          Setting.resolve
            defaults.selection_format
            ~inherited:Selection_format.Plain_text
            ~builtin:Selection_format.Plain_text
        else Selection_format.Plain_text
      in
      Setting.resolve
        t.overrides.selection_format
        ~inherited
        ~builtin:Selection_format.Plain_text
    in
    let max_lines =
      let inherited =
        if rich && Layout.equal layout Flow
        then Setting.resolve_optional defaults.max_lines ~inherited:None
        else None
      in
      Setting.resolve_optional t.overrides.max_lines ~inherited
    in
    let text_style =
      let inherited =
        if rich
        then Setting.resolve_optional defaults.text_style ~inherited:None
        else None
      in
      Setting.resolve_optional t.overrides.text_style ~inherited
    in
    let actions =
      let inherited =
        if rich
        then
          Setting.resolve
            defaults.actions
            ~inherited:Actions.Config.default
            ~builtin:Actions.Config.default
        else Actions.Config.default
      in
      Setting.resolve t.overrides.actions ~inherited ~builtin:Actions.Config.default
    in
    let markdown_options =
      let inherited =
        if Mode.equal t.mode Markdown
        then
          Setting.resolve
            defaults.markdown_options
            ~inherited:Markdown_options.default
            ~builtin:Markdown_options.default
        else Markdown_options.default
      in
      Setting.resolve
        t.overrides.markdown_options
        ~inherited
        ~builtin:Markdown_options.default
    in
    let%map.Or_error resolved =
      create
        ~source:t.source
        ~mode:t.mode
        ~label:t.label
        ?path:t.path
        ~search:t.search
        ~images:t.images
        ?diff:t.diff
        ~appearance
        ~layout
        ~line_numbers
        ~initially_collapsed
        ~selection_format
        ?max_lines
        ?text_style
        ~actions
        ~markdown_options
        ()
    in
    { resolved with overrides = t.overrides }
  ;;

  let with_overrides
        t
        ?appearance
        ?layout
        ?line_numbers
        ?initially_collapsed
        ?selection_format
        ?max_lines
        ?text_style
        ?actions
        ?markdown_options
        ()
    =
    let overrides =
      { Overrides.appearance = Option.value appearance ~default:t.overrides.appearance
      ; layout = Option.value layout ~default:t.overrides.layout
      ; line_numbers = Option.value line_numbers ~default:t.overrides.line_numbers
      ; initially_collapsed =
          Option.value initially_collapsed ~default:t.overrides.initially_collapsed
      ; selection_format =
          Option.value selection_format ~default:t.overrides.selection_format
      ; max_lines = Option.value max_lines ~default:t.overrides.max_lines
      ; text_style = Option.value text_style ~default:t.overrides.text_style
      ; actions = Option.value actions ~default:t.overrides.actions
      ; markdown_options =
          Option.value markdown_options ~default:t.overrides.markdown_options
      }
    in
    resolve { t with overrides } ~defaults:Overrides.empty
  ;;

  let source t = t.source
  let mode t = t.mode
  let appearance t = t.appearance
  let layout t = t.layout
  let label t = t.label
  let path t = t.path
  let line_numbers t = t.line_numbers
  let initially_collapsed t = t.initially_collapsed
  let search t = t.search
  let images t = t.images
  let diff t = t.diff
  let selection_format t = t.selection_format
  let max_lines t = t.max_lines
  let text_style t = t.text_style
  let markdown_options t = t.markdown_options
  let actions t = t.actions
end

module Defaults = struct
  type 'action t =
    { overrides : Overrides.t
    ; on_action : (Config.t -> Actions.Event.t -> 'action) option
    ; profile :
        (Config.t
         -> Gpuio_protocol.Document_profile_wire.Instance.t
            * (Gpuio_protocol.Document_profile_wire.Event.t -> 'action option))
          option
    }

  let empty = { overrides = Overrides.empty; on_action = None; profile = None }

  let create
        ?appearance
        ?layout
        ?line_numbers
        ?initially_collapsed
        ?selection_format
        ?max_lines
        ?text_style
        ?actions
        ?markdown_options
        ?on_action
        ()
    =
    let valid_layout =
      match layout with
      | None | Some Layout.Flow -> true
      | Some (Viewport height) ->
        Float.is_finite height && Float.(height >= 1. && height <= 16384.)
    in
    if not valid_layout
    then
      Or_error.error_string
        "document default viewport height must be finite and in 1..16384"
    else if not (Option.for_all max_lines ~f:(fun n -> n >= 1 && n <= 4096))
    then Or_error.error_string "document default preview limit must be in 1..4096"
    else
      Ok
        { overrides =
            Overrides.create
              ?appearance
              ?layout
              ?line_numbers
              ?initially_collapsed
              ?selection_format
              ?max_lines
              ?text_style
              ?actions
              ?markdown_options
              ()
        ; on_action
        ; profile = None
        }
  ;;

  let with_profile t instance ~on_event =
    { t with
      profile =
        Some
          (fun config ->
            ( Profile.Expert.to_wire instance
            , fun event ->
                Profile.Expert.event instance event
                |> Result.ok
                |> Option.map ~f:(on_event config) ))
    }
  ;;

  module Expert = struct
    let resolve t config = Config.resolve config ~defaults:t.overrides

    let rich config =
      match Config.mode config with
      | Markdown | Html -> true
      | Code _ | Diff -> false
    ;;

    let action_handler t config =
      if rich config then Option.map t.on_action ~f:(fun f -> f config) else None
    ;;

    let profile t config =
      if rich config then Option.map t.profile ~f:(fun f -> f config) else None
    ;;
  end
end

module Expert = struct
  module Wire = Gpuio_protocol.Document_wire

  let to_wire t ~owner ~asset_owner : Wire.Config.t =
    let source =
      match owner with
      | Some owner when Text_source.Expert.belongs_to (Config.source t) ~owner ->
        Some (Text_source.Expert.native_id (Config.source t))
      | Some _ | None -> None
    in
    let mode : Wire.Mode.t =
      match Config.mode t with
      | Markdown -> Markdown
      | Code language -> Code (Language.to_string language)
      | Diff -> Diff
      | Html -> Html
    in
    let layout : Wire.Layout.t =
      match Config.layout t with
      | Flow -> Flow
      | Viewport height -> Viewport height
    in
    let images =
      List.map (Config.images t) ~f:(fun (url, asset) ->
        let source : Gpuio_protocol.Image_wire.Source.t =
          match asset_owner with
          | Some owner when Asset.Expert.belongs_to asset ~owner ->
            Reference (Asset.Expert.native_id asset)
          | Some _ | None -> Unavailable Wrong_application
        in
        url, source)
    in
    { source
    ; mode
    ; dark =
        (match Config.appearance t with
         | Light -> false
         | Dark -> true)
    ; layout
    ; label = Config.label t
    ; path = Config.path t
    ; line_numbers = Config.line_numbers t
    ; initially_collapsed = Config.initially_collapsed t
    ; search = Config.search t
    ; images
    }
  ;;

  let navigation : Wire.Navigation.t -> Navigation.t Or_error.t = function
    | Link url ->
      if Config.valid_text url ~max:4096 && not (String.is_empty url)
      then Ok (Link { url; activation = None })
      else Or_error.error_string "invalid document link"
    | Link_activated (url, activation) ->
      if Config.valid_text url ~max:4096 && not (String.is_empty url)
      then (
        let%map.Or_error activation = Activation.Expert.of_wire activation in
        Navigation.Link { url; activation = Some activation })
      else Or_error.error_string "invalid document link"
    | Line (path, side, line) ->
      (match Int64.to_int line with
       | Some line
         when line > 0
              && Option.for_all path ~f:(fun text -> Config.valid_text text ~max:4096) ->
         let side : Navigation.Side.t =
           match side with
           | Before -> Before
           | After -> After
         in
         Ok (Line { path; side; line })
       | Some _ | None -> Or_error.error_string "invalid document line")
  ;;
end
