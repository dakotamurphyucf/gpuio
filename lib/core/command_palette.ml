module Ui_command = Command
open Core
module Wire = Gpuio_protocol.Wire
module Options = Gpuio_protocol.Palette_options_wire
module Search = Options.Search
module Escape = Options.Escape
module Layout = Gpuio_protocol.Palette_layout_wire

let valid_text limit text =
  String.length text <= limit
  && (not (String.is_empty (String.strip text)))
  && Stdlib.String.is_valid_utf_8 text
  && not (String.contains text '\000')
;;

module Group = struct
  module Id = struct
    type t = string [@@deriving equal, compare, sexp_of]

    let of_string text =
      if valid_text 256 text
      then Ok text
      else
        Or_error.error_string
          "palette group ID must be nonblank bounded UTF-8 without NUL"
    ;;

    let to_string t = t
  end

  type t =
    { id : Id.t
    ; label : string option
    ; commands : Ui_command.Id.t list
    }
  [@@deriving equal, sexp_of]

  let create ~id ?label ~commands () =
    if Option.exists label ~f:(fun label -> not (valid_text 4096 label))
    then
      Or_error.error_string
        "palette group label must be nonblank bounded UTF-8 without NUL"
    else if List.length commands > 1024
    then Or_error.error_string "palette group exceeds 1024 commands"
    else Ok { id; label; commands }
  ;;

  let id t = t.id
  let label t = t.label
  let commands t = t.commands
end

module Entry = struct
  type t =
    | Command of Ui_command.Id.t
    | Group of Group.t
    | Separator
  [@@deriving equal, sexp_of]
end

module Config = struct
  type t =
    { label : string
    ; placeholder : string
    ; commands : Ui_command.Id.t list
    ; dismiss_on_outside_pointer : bool
    ; options : Options.t option
    ; layout : Layout.t option
    }
  [@@deriving equal, sexp_of]

  let create
        ~label
        ~commands
        ?(placeholder = "Search commands")
        ?(dismiss_on_outside_pointer = true)
        ?(search = Search.All_terms)
        ?(searchable = true)
        ?(escape = Escape.Dismiss)
        ?(keywords = [])
        ()
    =
    let text value =
      String.length value <= 4096
      && Stdlib.String.is_valid_utf_8 value
      && not (String.contains value '\000')
    in
    let ids = List.map commands ~f:Ui_command.Id.to_string in
    let options : Options.t =
      { search
      ; searchable
      ; escape
      ; keywords =
          List.map keywords ~f:(fun (command, words) ->
            { Options.Keywords.command = Ui_command.Id.to_string command; words })
      }
    in
    let known = String.Set.of_list ids in
    if (not (text label && text placeholder)) || String.is_empty (String.strip label)
    then
      Or_error.error_string
        "palette label and placeholder must be bounded UTF-8 without NUL; label must be \
         nonblank"
    else if
      List.length ids > 1024 || Set.length (String.Set.of_list ids) <> List.length ids
    then Or_error.error_string "palette requires at most 1024 unique command IDs"
    else if
      (not (Options.valid options))
      || List.exists options.keywords ~f:(fun entry -> not (Set.mem known entry.command))
    then
      Or_error.error_string
        "palette keywords require unique known IDs and bounded nonblank text"
    else if
      String.length label
      + String.length placeholder
      + List.sum (module Int) ids ~f:String.length
      + Options.text_bytes options
      > 262_144
    then Or_error.error_string "palette metadata exceeds 256 KiB"
    else
      Ok
        { label
        ; placeholder
        ; commands
        ; dismiss_on_outside_pointer
        ; options = Option.some_if (not (Options.equal options Options.default)) options
        ; layout = None
        }
  ;;

  let create_entries
        ~label
        ~entries
        ?placeholder
        ?dismiss_on_outside_pointer
        ?search
        ?searchable
        ?escape
        ?keywords
        ()
    =
    let commands =
      List.concat_map entries ~f:(function
        | Entry.Command command -> [ command ]
        | Group group -> Group.commands group
        | Separator -> [])
    in
    let groups =
      List.filter_map entries ~f:(function
        | Entry.Group group -> Some (Group.Id.to_string (Group.id group))
        | Command _ | Separator -> None)
    in
    if
      List.length entries > 1024
      || Set.length (String.Set.of_list groups) <> List.length groups
    then
      Or_error.error_string "palette requires at most 1024 entries and unique group IDs"
    else (
      let%bind.Or_error t =
        create
          ~label
          ~commands
          ?placeholder
          ?dismiss_on_outside_pointer
          ?search
          ?searchable
          ?escape
          ?keywords
          ()
      in
      let _, layout =
        List.fold_map entries ~init:0 ~f:(fun index -> function
          | Entry.Command _ -> index + 1, Layout.Entry.Command index
          | Group group ->
            let count = List.length (Group.commands group) in
            ( index + count
            , Layout.Entry.Group
                ( Group.Id.to_string (Group.id group)
                , Group.label group
                , List.init count ~f:(fun offset -> index + offset) ) )
          | Separator -> index, Layout.Entry.Separator)
      in
      let bytes =
        String.length t.label
        + String.length t.placeholder
        + List.sum
            (module Int)
            commands
            ~f:(fun id -> String.length (Ui_command.Id.to_string id))
        + Option.value_map t.options ~default:0 ~f:Options.text_bytes
        + Layout.text_bytes layout
      in
      if bytes > 262_144
      then Or_error.error_string "palette metadata exceeds 256 KiB"
      else Ok { t with layout = Some layout })
  ;;

  let commands t = t.commands
end

module Results = struct
  type t = Gpuio_protocol.Palette_results_wire.t [@@deriving equal, sexp_of]

  let of_config (config : Config.t) : t =
    { commands = List.map config.commands ~f:Ui_command.Id.to_string
    ; layout = config.layout
    }
  ;;

  let create ~commands () =
    Config.create ~label:"Results" ~placeholder:"" ~commands ()
    |> Or_error.map ~f:of_config
  ;;

  let create_entries ~entries () =
    Config.create_entries ~label:"Results" ~placeholder:"" ~entries ()
    |> Or_error.map ~f:of_config
  ;;
end

module Dismissal = struct
  type t =
    | Escape
    | Outside_pointer
    | Selected of Ui_command.Id.t
  [@@deriving equal, sexp_of]
end

module Appearance = Choice.Appearance

module Snapshot = struct
  type t =
    { window : Gpuio_protocol.Window_id.t
    ; node : Gpuio_protocol.Node_id.t
    ; observer : Gpuio_protocol.Handler_id.t
    ; wire : Gpuio_protocol.Palette_state_wire.t
    }
  [@@deriving equal, sexp_of]

  let query t = t.wire.query
  let composing t = t.wire.composing
  let loading t = t.wire.loading

  let selected t =
    Option.map t.wire.selected ~f:(fun id ->
      Ui_command.Id.of_string id |> Or_error.ok_exn)
  ;;

  let matched_count t = t.wire.matched_count

  let same_query t other =
    Gpuio_protocol.Window_id.equal t.window other.window
    && Gpuio_protocol.Node_id.equal t.node other.node
    && Gpuio_protocol.Handler_id.equal t.observer other.observer
    && Int64.equal t.wire.query_revision other.wire.query_revision
  ;;
end

module Command = struct
  type t =
    | Read_snapshot
    | Focus
    | Set_query of string
    | Highlight of Ui_command.Id.t option
    | Set_loading of bool
    | Publish_results of Results.t
  [@@deriving equal, sexp_of]
end

module Command_error = Gpuio_protocol.Palette_command_wire.Error

module Expert = struct
  let snapshot_of_wire ~window ~node ~observer wire =
    if Gpuio_protocol.Palette_state_wire.valid wire
    then Ok Snapshot.{ window; node; observer; wire }
    else Or_error.error_string "invalid palette snapshot"
  ;;

  let window (t : Snapshot.t) = t.window
  let node (t : Snapshot.t) = t.node
  let observer (t : Snapshot.t) = t.observer
  let sequence (t : Snapshot.t) = t.wire.sequence
  let query_revision (t : Snapshot.t) = t.wire.query_revision

  let same_owner (a : Snapshot.t) (b : Snapshot.t) =
    Gpuio_protocol.Window_id.equal a.window b.window
    && Gpuio_protocol.Node_id.equal a.node b.node
    && Gpuio_protocol.Handler_id.equal a.observer b.observer
  ;;

  let command_to_wire : Command.t -> Gpuio_protocol.Palette_command_wire.Command.t =
    function
    | Read_snapshot -> Read_snapshot
    | Focus -> Focus
    | Set_query query -> Set_query query
    | Set_loading loading -> Set_loading loading
    | Publish_results results -> Publish_results results
    | Highlight selected -> Highlight (Option.map selected ~f:Ui_command.Id.to_string)
  ;;

  let options (t : Config.t) = t.options
  let layout (t : Config.t) = t.layout

  let to_wire (t : Config.t) : Wire.Palette.t =
    { label = t.label
    ; placeholder = t.placeholder
    ; commands = List.map t.commands ~f:Ui_command.Id.to_string
    ; dismiss_on_outside_pointer = t.dismiss_on_outside_pointer
    }
  ;;

  let dismissal t : Wire.Palette_dismissal.t -> Dismissal.t option = function
    | Escape -> Some Escape
    | Outside_pointer ->
      if t.Config.dismiss_on_outside_pointer then Some Outside_pointer else None
    | Selected id ->
      List.find t.commands ~f:(fun command ->
        String.equal (Ui_command.Id.to_string command) id)
      |> Option.map ~f:(fun id -> Dismissal.Selected id)
  ;;
end
