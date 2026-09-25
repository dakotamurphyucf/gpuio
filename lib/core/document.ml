open Core

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
    | Link of string
    | Line of
        { path : string option
        ; side : Side.t
        ; line : int
        }
  [@@deriving equal, sexp_of]
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
        ?(appearance = Appearance.Light)
        ?(layout = Layout.Flow)
        ?(label = "Document")
        ?path
        ?(line_numbers = true)
        ?(initially_collapsed = false)
        ?(search = "")
        ?(images = [])
        ()
    =
    let valid_layout =
      match layout with
      | Flow -> true
      | Viewport height ->
        Float.is_finite height && Float.(height >= 1. && height <= 16384.)
    in
    if not valid_layout
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
        }
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
      then Ok (Link url)
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
