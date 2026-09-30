open Core
open Gpuio
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View
module Panel = Gpuio_bonsai.Settings
module Model = Gpuio_gallery_model.Settings_state
module F = Model.Field
module Text = Gpuio_eio.Text_input
module Number = Gpuio_eio.Number_input
module Scope = Gpuio_eio.Scope

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn
let full = Length.percent_exn 100.
let key = Key.of_string_exn
let field ?help ?error label = Accessibility.Field.create ~label ?help ?error () |> ok
let regions = [ "Americas"; "Europe"; "Asia Pacific" ]

let choices prefix labels =
  List.mapi labels ~f:(fun n label ->
    ( n
    , Choice.create ~id:(Choice.Id.of_string (prefix ^ Int.to_string n) |> ok) ~label ()
      |> ok ))
  |> Settings_field.Choices.create (module Int)
  |> ok
;;

let region_choices = choices "region-" regions
let model_choices = choices "model-" (List.init 250 ~f:(fun n -> sprintf "Model %03d" n))
let domain = Numeric.Domain.create ~min:0. ~max:1000. ~step:1. |> ok

let input_style =
  style
    [ Width full
    ; Height (px 38.)
    ; Min_width (px 0.)
    ; Border_width 1.
    ; Radius 6.
    ; Padding (px 6.)
    ]
;;

let component ~save window palette graph =
  let data = B.Expert.Var.create Model.initial in
  let status =
    B.Expert.Var.create "Changes stay in this preview until you export them."
  in
  let busy = B.Expert.Var.create false in
  let query_error = B.Expert.Var.create None in
  let data_value = B.Expert.Var.value data in
  let update action =
    match Model.apply (B.Expert.Var.get data) action with
    | Ok next ->
      if not (Model.equal next (B.Expert.Var.get data)) then B.Expert.Var.set data next
    | Error error -> B.Expert.Var.set status (Error.to_string_hum error)
  in
  let action a = E.of_thunk (fun () -> update a) in
  let name =
    Text.create
      window
      ~initial_text:(Model.name Model.initial)
      ~config:
        (B.return
           (Text_input.Config.create ~mode:Single_line ~label:"Settings workspace name" ()
            |> ok))
      graph
  in
  let number =
    Number.create
      window
      ~initial:(Model.budget Model.initial)
      ~config:
        (B.return
           (Number_input.Config.create
              ~domain
              ~label:"Settings response budget"
              ~allow_empty:false
              ()
            |> ok))
      graph
  in
  let search =
    Text.create
      window
      ~config:
        (B.return
           (Text_input.Config.create
              ~mode:Single_line
              ~label:"Search settings"
              ~placeholder:"Find a preference…"
              ()
            |> ok))
      graph
  in
  let open B.Let_syntax in
  (* Read through the controller's generation/revision-checked observation lane.
     Only text/value changes enter application data; focus/selection stays native. *)
  B.Edge.on_change
    (B.map name ~f:Text.snapshot)
    ~equal:(Option.equal Text_input.Snapshot.equal)
    ~callback:
      (B.return (function
         | None -> E.Ignore
         | Some snapshot -> action (Model.Action.Name (Text_input.Snapshot.text snapshot))))
    graph;
  B.Edge.on_change
    (B.map number ~f:Number.snapshot)
    ~equal:(Option.equal Number_input.Snapshot.equal)
    ~callback:
      (B.return (function
         | None -> E.Ignore
         | Some snapshot ->
           action
             (Model.Action.Budget
                ( Number_input.Snapshot.committed snapshot
                , Number_input.Draft.of_string (Number_input.Snapshot.draft snapshot)
                  |> ok ))))
    graph;
  B.Edge.on_change
    (B.map search ~f:Text.snapshot)
    ~equal:(Option.equal Text_input.Snapshot.equal)
    ~callback:
      (B.return (function
         | None -> E.Ignore
         | Some snapshot ->
           E.of_thunk (fun () ->
             match Settings.Query.of_string (Text_input.Snapshot.text snapshot) with
             | Error error ->
               B.Expert.Var.set query_error (Some (Error.to_string_hum error))
             | Ok query ->
               B.Expert.Var.set query_error None;
               update (Navigate (Search query)))))
    graph;
  let peek_name = B.peek name graph in
  let peek_number = B.peek number graph in
  let report message = E.of_thunk (fun () -> B.Expert.Var.set status message) in
  let reset_field =
    let%arr peek_name = peek_name
    and peek_number = peek_number in
    fun ~scope field ->
      let is_allowed () =
        List.exists (Model.reset_targets (B.Expert.Var.get data) scope) ~f:(F.equal field)
      in
      let open E.Let_syntax in
      let%bind allowed = E.of_thunk is_allowed in
      if not allowed
      then E.Ignore
      else (
        match field with
        | F.Name ->
          let%bind current = peek_name in
          (match current with
           | Inactive -> report "The name editor is no longer active."
           | Active editor ->
             let%bind observed = Text.read_snapshot editor in
             (match observed with
              | Error error ->
                report
                  ("Name reset: "
                   ^ Sexp.to_string (Text_input.Command_error.sexp_of_t error))
              | Ok expected ->
                let%bind allowed = E.of_thunk is_allowed in
                if not allowed
                then E.Ignore
                else (
                  let%bind result =
                    Text.replace_if_unchanged
                      editor
                      expected
                      ~selection:End
                      ~undo:Record
                      (Model.name Model.initial)
                  in
                  match result with
                  | Error error ->
                    report
                      ("Name reset: "
                       ^ Sexp.to_string (Text_input.Command_error.sexp_of_t error))
                  | Ok _ -> report "Workspace name reset.")))
        | Budget ->
          let%bind current = peek_number in
          (match current with
           | Inactive -> report "The budget editor is no longer active."
           | Active editor ->
             let%bind observed = Number.read_snapshot editor in
             (match observed with
              | Error error ->
                report
                  ("Budget reset: "
                   ^ Sexp.to_string (Number_input.Command_error.sexp_of_t error))
              | Ok expected ->
                let%bind allowed = E.of_thunk is_allowed in
                if not allowed
                then E.Ignore
                else (
                  let%bind result =
                    Number.replace_value_if_unchanged
                      editor
                      expected
                      ~selection:End
                      ~undo:Record
                      (Model.budget Model.initial)
                  in
                  match result with
                  | Error error ->
                    report
                      ("Budget reset: "
                       ^ Sexp.to_string (Number_input.Command_error.sexp_of_t error))
                  | Ok _ -> report "Response budget reset.")))
        | Notifications | Reports | Region | Model | Custom | Locked | Feature _ ->
          action (Reset field))
  in
  let reset =
    let%arr reset_field = reset_field in
    fun scope ->
      E.bind
        (E.of_thunk (fun () -> Model.reset_targets (B.Expert.Var.get data) scope))
        ~f:(fun fields ->
          List.fold fields ~init:E.Ignore ~f:(fun prior field ->
            E.bind prior ~f:(fun () -> reset_field ~scope field)))
  in
  let saving_scope =
    Preview_scope.acquire
      window
      ~name:"settings-export"
      ~create:(fun scope ->
        E.of_thunk (fun () ->
          Scope.on_cancel scope (fun () -> B.Expert.Var.set busy false)
          |> Or_error.map ~f:(fun _unregister -> scope)))
      graph
  in
  let save_effect scope ~fail =
    let open E.Let_syntax in
    let%bind admitted =
      E.of_thunk (fun () ->
        let admitted = Scope.is_active scope && not (B.Expert.Var.get busy) in
        if admitted then B.Expert.Var.set busy true;
        admitted)
    in
    if not admitted
    then E.Ignore
    else (
      let%bind destination =
        if fail
        then E.return (Ok None)
        else
          Gpuio_eio.File_dialog.save
            window
            ~config:
              (File_dialog.Save.create
                 ~directory:(File_path.of_string "/tmp" |> ok)
                 ~suggested_name:"gpuio-settings.sexp"
                 ~title:"Export preview settings"
                 ()
               |> ok)
      in
      let%bind active = E.of_thunk (fun () -> Scope.is_active scope) in
      if not active
      then E.Ignore
      else (
        match destination with
        | Error error ->
          E.Many
            [ E.of_thunk (fun () -> B.Expert.Var.set busy false)
            ; report
                ("Export dialog: " ^ Sexp.to_string (File_dialog.Error.sexp_of_t error))
            ]
        | Ok None when not fail -> E.of_thunk (fun () -> B.Expert.Var.set busy false)
        | Ok path ->
          E.of_thunk (fun () ->
            let contents = Model.encode (B.Expert.Var.get data) in
            let result =
              Scope.start
                scope
                ~f:(fun () ->
                  if fail
                  then
                    Or_error.error_string
                      "Preview writer is unavailable. Your changes were kept."
                  else save (Option.value_exn path) contents)
                ~on_result:(fun result ->
                  E.of_thunk (fun () ->
                    B.Expert.Var.set busy false;
                    B.Expert.Var.set
                      status
                      (match Or_error.join result with
                       | Ok () ->
                         "Preview settings exported. New edits remain independent."
                       | Error error -> "Export failed: " ^ Error.to_string_hum error)))
            in
            match result with
            | Ok _ -> ()
            | Error error ->
              B.Expert.Var.set busy false;
              B.Expert.Var.set status (Error.to_string_hum error))))
  in
  let width, next_width =
    B.state_machine0 ~default_model:false ~apply_action:(fun _ v () -> not v) graph
  in
  let variant, next_variant =
    B.state_machine0
      ~default_model:Presentation.Group_variant.Outline
      ~apply_action:(fun _ v () ->
        match v with
        | Outline -> Filled
        | Filled -> Plain
        | Plain -> Card
        | Card -> Outline)
      graph
  in
  let size, next_size =
    B.state_machine0
      ~default_model:Presentation.Size.Medium
      ~apply_action:(fun _ v () ->
        match v with
        | Small -> Medium
        | Medium -> Large
        | Large -> Small)
      graph
  in
  let appearance = B.map palette ~f:Palette.appearance in
  let model = B.map data_value ~f:Model.catalog in
  let panel =
    Panel.component
      ~appearance
      ~model
      ~group_variant:variant
      ~size
      ~style:
        (let%arr narrow = width in
         style
           [ Width (if narrow then px 610. else full)
           ; Height (px 510.)
           ; Min_width (px 0.)
           ])
      ~search:
        (let%arr search = search in
         Text.view ~style:input_style search)
      ~on_request:(B.return (fun request -> action (Navigate request)))
      ~on_reset:reset
      ~page_suffix:
        (B.return (fun page ->
           Some
             (V.text
                (if
                   String.equal
                     (Settings.Page_id.to_string (Settings.Page.id page))
                     "advanced"
                 then "48"
                 else "8"))))
      ~render_item:(fun ~item ~layout ~lifetime graph ->
        let%arr item = item
        and layout = layout
        and lifetime = lifetime
        and state = data_value
        and name = name
        and number = number
        and p = palette
        and size = size
        and reset_field = reset_field in
        let guard = Gpuio_bonsai.Managed_rows.Lifetime.guard lifetime in
        let disabled = Settings.Item.is_disabled item in
        let field_kind = F.of_id (Settings.Item.id item) |> Option.value_exn in
        let semantic label help = field ~help label in
        let view =
          match field_kind with
          | F.Name ->
            Settings_field.control
              (field
                 ~help:"Changes are kept when you switch settings pages."
                 ?error:
                   (if String.is_empty (String.strip (Model.name state))
                    then Some "Enter a workspace name."
                    else None)
                 "Settings workspace name")
              ~layout
              ~size
              ~control:
                (Text.view ~style:input_style ~initial_text:(Model.name state) name)
              ()
            |> ok
          | Budget ->
            let error =
              match
                Numeric.Draft.parse
                  domain
                  (Number_input.Draft.to_string (Model.budget_draft state))
              with
              | Empty -> Some "A response budget is required."
              | Incomplete -> Some "Finish this number before committing."
              | Invalid _ -> Some "Enter a finite number."
              | Out_of_range _ -> Some "Enter commits the nearest value from 0 to 1000."
              | Valid _ -> None
            in
            Settings_field.control
              (field
                 ?error
                 ~help:"Press Enter to commit; Escape restores the committed value."
                 "Settings response budget")
              ~layout
              ~size
              ~control:
                (Number.view
                   ~style:input_style
                   ~initial:(Model.budget state)
                   ~initial_draft:(Model.budget_draft state)
                   number)
              ()
            |> ok
          | Notifications ->
            Settings_field.switch
              (semantic "Settings notifications" "Notify when work completes.")
              ~layout
              ~size
              ~checked:(Model.notifications state)
              ~on_toggle:(fun () -> guard (action Toggle_notifications))
              ()
            |> ok
          | Reports ->
            Settings_field.checkbox
              (semantic "Settings summaries" "One summary per week.")
              ~layout
              ~size
              ~checked:(Model.reports state)
              ~on_toggle:(fun () -> guard (action Toggle_reports))
              ()
            |> ok
          | Region ->
            Settings_field.Choices.select
              region_choices
              ~field:(field "Settings region")
              ~layout
              ~size
              ~selected:(Some (Model.region state))
              ~on_select:(fun n -> guard (action (Region n)))
              ()
            |> ok
          | Model ->
            Settings_field.Choices.select
              model_choices
              ~field:(field "Settings model")
              ~layout
              ~size
              ~appearance:(Choice.Appearance.create ~max_visible_rows:7 () |> ok)
              ~selected:(Some (Model.model state))
              ~on_select:(fun n -> guard (action (Model n)))
              ()
            |> ok
          | Custom ->
            V.column
              ~style:(style [ Gap (px 6.) ])
              [ Palette.text p "A custom policy control"
              ; V.button
                  ~disabled
                  ~accessible_name:"Settings custom action"
                  ~on_click:(guard (action Custom))
                  (sprintf "Custom actions: %d" (Model.custom state))
              ]
          | Locked ->
            Settings_field.switch
              (field "Settings organization policy")
              ~layout
              ~size
              ~disabled:true
              ~checked:true
              ~on_toggle:(fun () -> E.Ignore)
              ()
            |> ok
          | Feature n ->
            Settings_field.switch
              (field (sprintf "Settings feature %02d" n))
              ~layout
              ~size
              ~checked:(Model.feature state n)
              ~on_toggle:(fun () -> guard (action (Toggle_feature n)))
              ()
            |> ok
        in
        V.column
          ~style:(style [ Width full; Min_width (px 0.); Gap (px 4.) ])
          [ view
          ; V.button
              ~disabled:(not (Model.can_reset state field_kind))
              ~style:
                (style [ Align_self Start; Padding (px 5.); Font_size 12.; Radius 5. ])
              ~accessible_name:("Reset " ^ Settings.Item_id.to_string (F.id field_kind))
              ~on_click:
                (guard
                   (reset_field
                      ~scope:(Settings.Reset_scope.Item (F.id field_kind))
                      field_kind))
              "Reset"
          ])
      graph
  in
  let%arr panel = panel
  and state = data_value
  and p = palette
  and narrow = width
  and next_width = next_width
  and variant = variant
  and next_variant = next_variant
  and size = size
  and next_size = next_size
  and status = B.Expert.Var.value status
  and busy = B.Expert.Var.value busy
  and query_error = B.Expert.Var.value query_error
  and saving_scope = saving_scope in
  let variant_label =
    match variant with
    | Outline -> "Outline groups"
    | Filled -> "Filled groups"
    | Plain -> "Plain groups"
    | Card -> "Card groups"
  in
  let size_label =
    match size with
    | Small -> "Small fields"
    | Medium -> "Medium fields"
    | Large -> "Large fields"
  in
  let save fail =
    match saving_scope with
    | Ready scope -> save_effect scope ~fail
    | Loading | Failed _ -> E.Ignore
  in
  V.column
    ~style:(style [ Width full; Min_width (px 0.); Gap (px 14.) ])
    [ V.row
        ~style:(style [ Gap (px 8.); Wrap Wrap ])
        [ Palette.button
            p
            (if narrow then "Widen settings" else "Narrow settings")
            (next_width ())
        ; Palette.button p variant_label (next_variant ())
        ; Palette.button p size_label (next_size ())
        ; V.checkbox
            ~state:(Check_state.of_bool (Model.locked state))
            ~on_toggle:(action Toggle_lock)
            "Lock custom setting"
        ]
    ; (match query_error with
       | None -> V.text "Find preferences by name, description or keyword."
       | Some error -> V.text error)
    ; (match panel with
       | Ok panel -> Panel.Output.view panel
       | Error error -> V.text (Error.to_string_hum error))
    ; Palette.text
        p
        (sprintf
           "Stored name: %s · region: %s · model: %03d · custom: %d"
           (Model.name state)
           (List.nth_exn regions (Model.region state))
           (Model.model state)
           (Model.custom state))
    ; V.row
        ~style:(style [ Gap (px 8.); Wrap Wrap ])
        [ Palette.button p ~disabled:busy "Export settings…" (save false)
        ; Palette.button p ~disabled:busy "Try failed export" (save true)
        ]
    ; Palette.text p ~muted:true status
    ]
;;
