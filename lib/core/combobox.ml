open Core

module Filter = struct
  type t =
    | Substring
    | Unfiltered
  [@@deriving equal, sexp_of]
end

module Config = struct
  type t =
    { choices : Choice.Config.t
    ; editor : Text_input.Config.t
    ; filter : Filter.t
    }
  [@@deriving equal, sexp_of]

  let create
        ~label
        ~options
        ~selected
        ?placeholder
        ?disabled
        ?auto_focus
        ?(filter = Filter.Substring)
        ()
    =
    let open Or_error.Let_syntax in
    let%bind choices = Choice.Config.create ~label ~options ~selected ?disabled () in
    let%map editor =
      Text_input.Config.create
        ~mode:Single_line
        ~label
        ?placeholder
        ?disabled
        ?auto_focus
        ~submit_on_enter:false
        ()
    in
    { choices; editor; filter }
  ;;

  let choices t = t.choices
  let filter t = t.filter
end

module Selection = struct
  type t =
    { id : Choice.Id.t
    ; snapshot : Text_input.Snapshot.t
    }
  [@@deriving equal, sexp_of]

  let id t = t.id
  let snapshot t = t.snapshot
end

module Event = struct
  type t =
    | Changed of Text_input.Snapshot.t
    | Selected of Selection.t
  [@@deriving equal, sexp_of]
end

module Expert = struct
  let editor_config (t : Config.t) = t.editor

  let selection config ~id ~snapshot =
    let open Or_error.Let_syntax in
    let%bind () =
      if Choice.Config.can_select (Config.choices config) id
      then Ok ()
      else Or_error.error_string "combobox choice is absent or disabled"
    in
    let%bind () =
      if Option.is_none (Text_input.Snapshot.composition snapshot)
      then Ok ()
      else Or_error.error_string "cannot select a combobox choice during composition"
    in
    let%map () =
      Text_input.validate_text ~mode:Single_line (Text_input.Snapshot.text snapshot)
    in
    { Selection.id; snapshot }
  ;;
end
