module Ui_command = Command
open Core
open Gpuio_protocol
module Wire = Wire
module TW = Gpuio_protocol.Table_wire
module LI = Gpuio_protocol.List_input_wire

exception Cannot_prepare of Error.t

let fail message = raise (Cannot_prepare (Error.of_string message))

let value = function
  | Ok value -> value
  | Error error -> raise (Cannot_prepare error)
;;

module Allocator = struct
  type t =
    { next : int
    ; free : int list
    ; generations : int64 Int.Map.t
    }

  let empty = { next = 0; free = []; generations = Int.Map.empty }

  let allocate t =
    let slot, t =
      match t.free with
      | slot :: free -> slot, { t with free }
      | [] ->
        if t.next >= 100_000 then fail "native slot limit exceeded";
        t.next, { t with next = t.next + 1 }
    in
    let generation =
      Map.find t.generations slot |> Option.value ~default:0L |> Int64.succ
    in
    ( slot
    , generation
    , { t with generations = Map.set t.generations ~key:slot ~data:generation } )
  ;;

  let release t slot =
    (* A generation at the wire maximum is a permanent tombstone, never wrapped. *)
    if Int64.equal (Map.find_exn t.generations slot) 0xffff_ffffL
    then t
    else { t with free = slot :: t.free }
  ;;
end

module Identity = struct
  module T = struct
    type t =
      | Key of string
      | Position of int
      | Structural of string * string
    [@@deriving compare, sexp]
  end

  include T
  include Comparable.Make (T)

  let of_view view position =
    let description = View.Expert.describe view in
    match description.structural_key, description.key with
    | Some (namespace, identity), _ -> Structural (namespace, identity)
    | None, Some key -> Key (Key.to_string key)
    | None, None -> Position position
  ;;
end

type 'a callback =
  | Container_query of
      Wire.Container_query.Config.t * int64 ref * (Container_query.Selection.t -> 'a)
  | Extension of Wire.Extension.Config.t * (Wire.Extension.Signal.t -> 'a)
  | Split_pane of Split_pane.Config.t * (Split_pane.Snapshot.t -> 'a)
  | Split_group of Split_group.Config.t * (Split_group.Snapshot.t -> 'a)
  | Canvas of Wire.Canvas_view.Config.t * (Canvas.Event.t -> 'a)
  | Chart of Wire.Chart_view.Config.t * (Chart.Event.t -> 'a)
  | Document of
      { source : Text_source.Handle.t
      ; diff_config : Document.Diff.Config.t option
      ; diff_epoch : int64
      ; on_navigate : (Document.Navigation.t -> 'a) option
      ; on_diff : (Document.Diff.Event.t -> 'a) option
      ; preview_epoch : int64
      ; preview_max_lines : int option
      ; on_preview : (Document.Preview.Event.t -> 'a) option
      ; on_action : (Document.Actions.Event.t -> 'a) option
      ; actions_epoch : int64
      ; actions_config : Document.Actions.Config.t
      ; profile :
          (int64
          * Gpuio_protocol.Document_profile_wire.Instance.t
          * (Gpuio_protocol.Document_profile_wire.Event.t -> 'a option))
            option
      }
  | Virtual_list of
      List_identity.t
      * 'a View.Expert.virtual_list
      * TW.Config.t option
      * LI.Config.t option
  | Animation_program of int64 * (int64 * int64) ref * (Animation.Program.Event.t -> 'a)
  | Animation of int64 * int64 ref * (Animation.Event.t -> 'a)
  | Image of (Image.State.t -> 'a)
  | Input_region of Gpuio_protocol.Input_wire.Config.t * (Input_region.Event.t -> 'a)
  | Command_binding_scope of
      Command_binding.Config.t * int64 ref * (Command_binding.Observation.t -> 'a)
  | Highlight_scope of
      Gpuio_protocol.Highlight_wire.Config.t * (Highlight.Observation.t -> 'a)
  | Pointer of (Pointer.Event.t -> 'a)
  | Drag_source of (Drag_and_drop.Source_event.t -> 'a)
  | Drop_target of (Drag_and_drop.Target_event.t -> 'a)
  | Toast of (Toast.Dismissal.t -> 'a)
  | Palette of
      Command_palette.Config.t
      * (Command_palette.Dismissal.t -> 'a)
      * int64 ref
      * (Command_palette.Snapshot.t -> 'a) option
  | Click of (unit -> 'a)
  | Commands of 'a Ui_command.Registry.t * Wire.Command.t list
  | Dismiss of Overlay.Config.t * (Overlay.Dismissal.t -> 'a)
  | Menu of (bool -> 'a)
  | Tooltip of Tooltip.Config.t * (bool -> 'a)
  | Editor of Text_input.Config.t * (Text_input.Event.t -> 'a)
  | Choice of Choice.Config.t * (Choice.Id.t -> 'a)
  | Carousel of Wire.Carousel.Config.t * (Carousel.Request.t -> 'a)
  | Carousel_track of Wire.Carousel_track.Config.t * (Carousel_track.Request.t -> 'a)
  | Rating of Rating.Config.t * (Rating.Request.t -> 'a)
  | Slider of Slider.Value.t * int64 ref * (Slider.Event.t -> 'a)
  | Number_input of (int64 * int64) ref * (Number_input.Event.t -> 'a)
  | Otp_input of Otp_input.Policy.t * int64 ref * (Otp_input.Event.t -> 'a)
  | Color_input of int64 ref * (Color_input.Event.t -> 'a)
  | Calendar of Calendar.Mode.t * int64 ref * (Calendar.Event.t -> 'a)
  | Combobox of Combobox.Config.t * (Combobox.Event.t -> 'a)
  | Picker_query
  | Choice_picker of
      Choice_picker.Config.t * Node_id.t option * (Choice_picker.Event.t -> 'a)

type 'a binding =
  { node : Node_id.t
  ; handler : Handler_id.t
  ; callback : 'a callback
  }

type list_input_state =
  { config : List_input.Config.t
  ; multiple : bool
  ; query_controller : Key.t option
  ; wire : LI.Config.t
  }

type 'a mounted =
  { view : 'a View.t
  ; id : Node_id.t
  ; handler : Handler_id.t option
  ; hover_handler : Handler_id.t option
  ; calendar_viewport_handler : Handler_id.t option
  ; style : Wire.Style.t list
  ; text_content : Gpuio_protocol.Text_content_wire.t option
  ; animation : Wire.Animation.Config.t option
  ; animation_seen : int64 ref
  ; animation_program : Wire.Animation_program.Config.t option
  ; program_seen : (int64 * int64) ref
  ; container_query : Wire.Container_query.Config.t option
  ; query_seen : int64 ref
  ; binding_seen : int64 ref
  ; palette_seen : int64 ref
  ; document_diff_epoch : int64
  ; document_profile_epoch : int64
  ; document_actions_epoch : int64
  ; document_preview_epoch : int64
  ; document_text_style : Wire.Document_style.t option
  ; slider_seen : int64 ref
  ; number_input_seen : (int64 * int64) ref
  ; otp_input_seen : int64 ref
  ; color_input_seen : int64 ref
  ; calendar_seen : int64 ref
  ; list_identity : List_identity.t option
  ; list_input_state : list_input_state option
  ; list_input_generation : int64
  ; table_config : TW.Config.t option
  ; table_behavior : TW.Behavior.t option
  ; table_appearance : TW.Appearance.t option
  ; table_header_style : Wire.Style.t list
  ; table_row_style : Wire.Style.t list
  ; table_serial : int64
  ; choice_picker : Wire.Choice_picker_presentation.t option
  ; split_group :
      (Gpuio_protocol.Split_group_wire.Config.t * Wire.Split_group_appearance.t) option
  ; tab_appearance : Wire.Tab_appearance.t option
  ; tab_content : Wire.Tab_content.t option
  ; tab_viewport : Wire.Tab_viewport.t option
  ; tab_motion : Wire.Tab_motion.t option
  ; tab_trailing : bool
  ; choice_menu : bool
  ; choice_appearance : Wire.Choice_appearance.t option
  ; split_button : Wire.Split_button.t option
  ; control_appearance : Wire.Control_appearance.t option
  ; scrollbar : Wire.Scrollbar.t option
  ; number_step_mode : Wire.Number_input.Step_mode.t
  ; number_presentation : Wire.Number_presentation.t option
  ; reveal : Gpuio_protocol.Reveal_wire.t option
  ; overlay_backdrop : int64 option
  ; calendar_appearance : Gpuio_protocol.Calendar_presentation_wire.t option
  ; color_presentation : Gpuio_protocol.Color_presentation_wire.t option
  ; slider_appearance : Gpuio_protocol.Slider_presentation_wire.t option
  ; otp_appearance : Gpuio_protocol.Otp_presentation_wire.t option
  ; rating_appearance : Wire.Rating.Appearance.t option
  ; children : 'a mounted list
  ; controllers : String.Set.t
  ; commands : Wire.Command.t list
  ; free_commands : String.Set.t
  ; menu : Wire.Menu.t option
  ; platform_menus : int
  }

type command_button =
  { node : Node_id.t
  ; command : Ui_command.Id.t
  ; loading : bool
  ; activation_revision : int64
  }

module Hover_binding = struct
  type 'a t =
    { node : Node_id.t
    ; handler : Handler_id.t
    ; on_change : bool -> 'a
    }
end

module Calendar_viewport_binding = struct
  type 'a t =
    { node : Node_id.t
    ; handler : Handler_id.t
    ; seen : int64 ref
    ; on_change : Calendar.Viewport.t -> 'a
    }
end

type 'a state =
  { root : 'a mounted option
  ; bindings : 'a binding Int.Map.t
  ; hover_bindings : 'a Hover_binding.t Int.Map.t
  ; calendar_viewport_bindings : 'a Calendar_viewport_binding.t Int.Map.t
  ; command_buttons : command_button Int.Map.t
  ; nodes : Allocator.t
  ; handlers : Allocator.t
  ; theme : Theme.t
  ; revision : int64
  ; epoch : int
  ; command_generation : int64
  }

type 'a t =
  { owner : unit ref
  ; document_defaults : 'a Document.Defaults.t
  ; asset_owner : Asset.Expert.Owner.t option
  ; document_owner : Text_source.Expert.Owner.t option
  ; canvas_owner : Canvas_scene.Expert.Owner.t option
  ; chart_owner : Chart_resource.Expert.Owner.t option
  ; window : Window_id.t
  ; mutable state : 'a state
  ; mutable closed : bool
  }

type 'a update =
  { owner : unit ref
  ; base_epoch : int
  ; candidate : 'a state
  ; message : Wire.Message.t option
  }

type 'a builder =
  { window : Window_id.t
  ; document_defaults : 'a Document.Defaults.t
  ; mutable nodes : Allocator.t
  ; mutable handlers : Allocator.t
  ; mutable bindings : 'a binding Int.Map.t
  ; mutable hover_bindings : 'a Hover_binding.t Int.Map.t
  ; mutable calendar_viewport_bindings : 'a Calendar_viewport_binding.t Int.Map.t
  ; mutable command_buttons : command_button Int.Map.t
  ; base_revision : int64
  ; mutable operations : Wire.Op.t list
  ; mutable operation_count : int
  ; mutable command_generation : int64
  ; theme : Theme.t
  ; theme_unchanged : bool
  ; asset_owner : Asset.Expert.Owner.t option
  ; document_owner : Text_source.Expert.Owner.t option
  ; canvas_owner : Canvas_scene.Expert.Owner.t option
  ; chart_owner : Chart_resource.Expert.Owner.t option
  }

let create
      ?(document_defaults = Document.Defaults.empty)
      ?asset_owner
      ?document_owner
      ?canvas_owner
      ?chart_owner
      window
  =
  { owner = ref ()
  ; document_defaults
  ; asset_owner
  ; document_owner
  ; canvas_owner
  ; chart_owner
  ; window
  ; closed = false
  ; state =
      { root = None
      ; bindings = Int.Map.empty
      ; hover_bindings = Int.Map.empty
      ; calendar_viewport_bindings = Int.Map.empty
      ; command_buttons = Int.Map.empty
      ; nodes = Allocator.empty
      ; handlers = Allocator.empty
      ; theme = Theme.default
      ; revision = 0L
      ; epoch = 0
      ; command_generation = 0L
      }
  }
;;

let emit builder operation =
  if builder.operation_count >= 4096
  then
    fail "atomic update exceeds 4096 operations; split the UI or use a managed component";
  builder.operation_count <- builder.operation_count + 1;
  builder.operations <- operation :: builder.operations
;;

let node_slot node = Node_id.slot node |> Int64.to_int_exn
let handler_slot handler = Handler_id.slot handler |> Int64.to_int_exn

let new_node builder =
  let slot, generation, allocator = Allocator.allocate builder.nodes in
  builder.nodes <- allocator;
  Node_id.create ~slot:(Int64.of_int slot) ~generation |> value
;;

let new_handler builder =
  let slot, generation, allocator = Allocator.allocate builder.handlers in
  builder.handlers <- allocator;
  Handler_id.create ~slot:(Int64.of_int slot) ~generation |> value
;;

let rec remove builder mounted =
  List.iter mounted.children ~f:(remove builder);
  emit builder (Remove mounted.id);
  builder.nodes <- Allocator.release builder.nodes (node_slot mounted.id);
  Option.iter mounted.handler ~f:(fun handler ->
    builder.handlers <- Allocator.release builder.handlers (handler_slot handler));
  Option.iter mounted.hover_handler ~f:(fun handler ->
    builder.handlers <- Allocator.release builder.handlers (handler_slot handler));
  builder.hover_bindings <- Map.remove builder.hover_bindings (node_slot mounted.id);
  Option.iter mounted.calendar_viewport_handler ~f:(fun handler ->
    builder.handlers <- Allocator.release builder.handlers (handler_slot handler));
  builder.calendar_viewport_bindings
  <- Map.remove builder.calendar_viewport_bindings (node_slot mounted.id);
  builder.bindings <- Map.remove builder.bindings (node_slot mounted.id);
  builder.command_buttons <- Map.remove builder.command_buttons (node_slot mounted.id)
;;

let kind = function
  | View.Expert.Kind.Container -> Wire.Kind.Container
  | Text -> Text
  | Button -> Button
  | Input -> Input
  | Textarea -> Textarea
  | Checkbox -> Checkbox
  | Switch -> Switch
  | Radio -> Radio
  | Radio_group -> Radio_group
  | Select -> Select
  | Combobox -> Combobox
  | Choice_picker -> Choice_picker
  | Focus_scope -> Focus_scope
  | Tooltip -> Tooltip
  | Hover_card -> Hover_card
  | Carousel -> Carousel
  | Carousel_track_group -> Carousel_track_group
  | Carousel_track -> Carousel_track
  | Command_scope -> Command_scope
  | Command_button -> Command_button
  | Menu -> Menu
  | Command_palette -> Command_palette
  | Progress -> Progress
  | Toast -> Toast
  | Toast_stack -> Toast_stack
  | Pointer_area -> Pointer_area
  | Input_region -> Input_region
  | Highlight_scope -> Highlight_scope
  | Link -> Link
  | Drag_source -> Drag_source
  | Drop_target -> Drop_target
  | Image -> Image
  | Icon -> Icon
  | Animated -> Animated
  | Animation_program -> Animation_program
  | Container_query -> Container_query
  | Loading -> Loading
  | Avatar -> Avatar
  | Rating -> Rating
  | Slider -> Slider
  | Number_input -> Number_input
  | Otp_input -> Otp_input
  | Color_input -> Color_input
  | Panel -> Panel
  | Disclosure -> Disclosure
  | Accordion -> Accordion
  | Navigation_stack -> Navigation_stack
  | Calendar -> Calendar
  | Virtual_list -> Virtual_list
  | Canvas_view -> Canvas_view
  | Chart_view -> Chart_view
  | Document_view -> Document_view
  | Tab_bar -> Tab_bar
  | Tab_panel -> Tab_panel
  | Split_pane -> Split_pane
  | Split_group -> Split_group
  | Extension -> Extension
;;

let compatible mounted view =
  let old = View.Expert.describe mounted.view
  and next = View.Expert.describe view in
  View.Expert.Kind.equal old.kind next.kind
  && Option.equal Key.equal old.key next.key
  && Bool.equal
       (Option.exists old.virtual_list ~f:(fun list -> Option.is_some list.table))
       (Option.exists next.virtual_list ~f:(fun list -> Option.is_some list.table))
  && Option.equal
       Key.equal
       (Option.bind old.virtual_list ~f:(fun list ->
          Option.bind list.table ~f:(fun table -> table.source_key)))
       (Option.bind next.virtual_list ~f:(fun list ->
          Option.bind list.table ~f:(fun table -> table.source_key)))
  && Bool.equal (Option.is_some old.table_header) (Option.is_some next.table_header)
  && Option.equal
       Table_column.Id.equal
       (Option.map old.table_cell ~f:Table.Cell.column)
       (Option.map next.table_cell ~f:Table.Cell.column)
  && Option.equal
       Wire.Extension.Schema.equal
       (Option.map old.extension ~f:(fun item -> item.config.schema))
       (Option.map next.extension ~f:(fun item -> item.config.schema))
;;

let splice builder id old_children new_children =
  let ids children = List.map children ~f:(fun child -> child.id) in
  let previous = ids old_children
  and next = ids new_children in
  if not (List.equal Node_id.equal previous next)
  then (
    let old = Array.of_list previous
    and next = Array.of_list next in
    let prefix = ref 0 in
    while
      !prefix < Array.length old
      && !prefix < Array.length next
      && Node_id.equal old.(!prefix) next.(!prefix)
    do
      incr prefix
    done;
    let suffix = ref 0 in
    while
      !suffix < Array.length old - !prefix
      && !suffix < Array.length next - !prefix
      && Node_id.equal
           old.(Array.length old - !suffix - 1)
           next.(Array.length next - !suffix - 1)
    do
      incr suffix
    done;
    let inserted =
      Array.sub next ~pos:!prefix ~len:(Array.length next - !prefix - !suffix)
      |> Array.to_list
    in
    emit
      builder
      (Splice
         ( id
         , Int64.of_int !prefix
         , Int64.of_int (Array.length old - !prefix - !suffix)
         , inserted )))
;;

(* A list may precede its query sibling. Finalize after the whole sibling set
   exists, including unchanged children reused by mount's fast path. Candidate
   state is immutable: rejected preparations never consume an accepted epoch. *)
let finalize_list_input builder ~query (mounted : _ mounted) =
  let description = View.Expert.describe mounted.view in
  let input = Option.bind description.virtual_list ~f:(fun list -> list.list_input) in
  let old = mounted.list_input_state in
  let old_wire = Option.map old ~f:(fun old -> old.wire) in
  match input with
  | None ->
    (match old with
     | None -> mounted
     | Some _ ->
       emit builder (Set_list_input (mounted.id, None));
       { mounted with list_input_state = None })
  | Some (config, _) ->
    let list = Option.value_exn description.virtual_list in
    if Option.is_some list.on_tree_input || list.tree_moves || Option.is_some list.table
    then fail "list input cannot share tree or table input";
    let multiple =
      match
        Option.bind description.accessibility ~f:(fun a ->
          (Accessibility.Expert.to_wire a).role)
      with
      | Some (List_box multiple) -> multiple
      | _ -> fail "list input requires List_box accessibility"
    in
    let resolved =
      Option.map (List_input.Config.query config) ~f:(fun key ->
        match query key with
        | Some query -> query
        | None -> fail "list query must name a direct sibling single-line input")
    in
    let query_node = Option.map resolved ~f:fst in
    let query_controller = Option.map resolved ~f:snd in
    let generation =
      match old with
      | Some old
        when List_input.Expert.same_interaction old.config config
             && Bool.equal old.multiple multiple
             && Option.equal Key.equal old.query_controller query_controller
             && Option.equal Node_id.equal old.wire.query query_node ->
        old.wire.generation
      | Some _ | None ->
        if Int64.equal mounted.list_input_generation Int64.max_value
        then fail "list input generation exhausted";
        Int64.succ mounted.list_input_generation
    in
    let cursor =
      Option.map (List_input.Config.cursor config) ~f:(fun key ->
        let identity = Option.value_exn mounted.list_identity in
        let id =
          match List_identity.id identity key with
          | Some id -> id
          | None -> fail "list cursor is absent from logical order"
        in
        Option.iter
          (List.find mounted.children ~f:(fun row ->
             Option.equal Key.equal (View.Expert.describe row.view).key (Some key)))
          ~f:(fun row ->
            match
              Option.bind (View.Expert.describe row.view).accessibility ~f:(fun a ->
                (Accessibility.Expert.to_wire a).role)
            with
            | Some (Option_item item) when not item.disabled -> ()
            | _ -> fail "mounted list cursor requires enabled Option_item metadata");
        id)
    in
    let wire : LI.Config.t =
      { generation
      ; cursor
      ; query = query_node
      ; selection_on_navigation = List_input.Config.selection_on_navigation config
      ; disabled = List_input.Config.disabled config
      ; busy = List_input.Config.busy config
      }
    in
    if not (Option.equal LI.Config.equal old_wire (Some wire))
    then emit builder (Set_list_input (mounted.id, Some wire));
    let slot = node_slot mounted.id in
    (match Map.find builder.bindings slot with
     | Some ({ callback = Virtual_list (identity, list, table, _); _ } as binding) ->
       builder.bindings
       <- Map.set
            builder.bindings
            ~key:slot
            ~data:
              { binding with callback = Virtual_list (identity, list, table, Some wire) }
     | Some _ | None -> fail "list input requires an input callback");
    { mounted with
      list_input_state = Some { config; multiple; query_controller; wire }
    ; list_input_generation = generation
    }
;;

let finalize_sibling_list_inputs builder children =
  let has_query =
    List.exists children ~f:(fun child ->
      Option.exists (View.Expert.describe child.view).virtual_list ~f:(fun list ->
        Option.exists list.list_input ~f:(fun (config, _) ->
          Option.is_some (List_input.Config.query config))))
  in
  let queries =
    if not has_query
    then String.Map.empty
    else
      List.filter_map children ~f:(fun child ->
        let view = View.Expert.describe child.view in
        match view.kind, view.key, view.editor with
        | Input, Some key, Some editor ->
          Some (Key.to_string key, (child.id, editor.controller))
        | _ -> None)
      |> String.Map.of_alist_exn
  in
  let used = ref String.Set.empty in
  List.map
    children
    ~f:
      (finalize_list_input builder ~query:(fun key ->
         let key = Key.to_string key in
         if Set.mem !used key then fail "query input cannot serve multiple lists";
         used := Set.add !used key;
         Map.find queries key))
;;

let document_description builder view =
  Option.map (View.Expert.describe view).document ~f:(fun document ->
    let config =
      Document.Defaults.Expert.resolve builder.document_defaults document.config |> value
    in
    let on_action =
      match document.on_action with
      | Some _ as handler -> handler
      | None -> Document.Defaults.Expert.action_handler builder.document_defaults config
    in
    let profile =
      if document.inherit_profile
      then Document.Defaults.Expert.profile builder.document_defaults config
      else document.profile
    in
    { document with config; on_action; profile })
;;

let rec mount builder ~depth previous view =
  if depth > 128 then fail "view exceeds native depth limit";
  match previous with
  | Some mounted when phys_equal mounted.view view && builder.theme_unchanged -> mounted
  | _ ->
    let description =
      { (View.Expert.describe view) with document = document_description builder view }
    in
    if String.length description.text > 262_144
    then fail "text field exceeds native byte limit";
    let previous =
      match previous with
      | Some mounted when compatible mounted view -> Some mounted
      | Some mounted ->
        remove builder mounted;
        None
      | None -> None
    in
    let id =
      match previous with
      | Some mounted -> mounted.id
      | None -> new_node builder
    in
    let color_input_seen =
      Option.value_map previous ~default:(ref (-1L)) ~f:(fun old -> old.color_input_seen)
    in
    let calendar_seen =
      Option.value_map previous ~default:(ref (-1L)) ~f:(fun old -> old.calendar_seen)
    in
    let otp_input_seen =
      Option.value_map previous ~default:(ref (-1L)) ~f:(fun old -> old.otp_input_seen)
    in
    let number_input_seen =
      Option.value_map
        previous
        ~default:(ref (-1L, 0L))
        ~f:(fun old -> old.number_input_seen)
    in
    let slider_seen =
      Option.value_map previous ~default:(ref (-1L)) ~f:(fun old -> old.slider_seen)
    in
    Option.iter description.slider ~f:(fun item ->
      Option.iter previous ~f:(fun mounted ->
        Option.iter (View.Expert.describe mounted.view).slider ~f:(fun old ->
          if
            not
              (Wire.Slider.Value.same_mode
                 (Slider.Expert.value_to_wire old.initial)
                 (Slider.Expert.value_to_wire item.initial))
          then fail "slider mode changes require a new controller key")));
    let old_handler = Option.bind previous ~f:(fun mounted -> mounted.handler) in
    let old_commands =
      Option.value_map previous ~default:[] ~f:(fun mounted -> mounted.commands)
    in
    let old_commands_by_id =
      String.Map.of_alist_exn
        (List.map old_commands ~f:(fun command -> command.Wire.Command.id, command))
    in
    let commands =
      Option.value_map description.commands ~default:[] ~f:(fun registry ->
        List.map (Ui_command.Registry.to_list registry) ~f:(fun command ->
          let id = Ui_command.Id.to_string (Ui_command.id command) in
          let old = Map.find old_commands_by_id id in
          let candidate = Ui_command.Expert.to_wire command ~generation:0L in
          let generation =
            match old with
            | Some old
              when Bool.equal old.enabled candidate.enabled
                   && Wire.Command_target.equal old.target candidate.target ->
              old.generation
            | Some _ | None ->
              if Int64.equal builder.command_generation Int64.max_value
              then fail "command generation exhausted";
              builder.command_generation <- Int64.succ builder.command_generation;
              builder.command_generation
          in
          { candidate with generation }))
    in
    let old_binding_config =
      Option.bind previous ~f:(fun old ->
        Option.map (View.Expert.describe old.view).command_binding_scope ~f:(fun item ->
          item.config))
    in
    let binding_config =
      Option.map description.command_binding_scope ~f:(fun item -> item.config)
    in
    let binding_changed =
      not (Option.equal Command_binding.Config.equal old_binding_config binding_config)
    in
    let binding_seen =
      if binding_changed
      then ref 0L
      else Option.value_map previous ~default:(ref 0L) ~f:(fun old -> old.binding_seen)
    in
    Option.iter binding_config ~f:(fun config ->
      if not (Command_binding.Expert.valid_window config builder.window)
      then fail "binding editor context belongs to another window");
    let query_seen =
      Option.value_map previous ~default:(ref 0L) ~f:(fun old -> old.query_seen)
    in
    let container_query =
      Option.map description.container_query ~f:(fun item ->
        let old = Option.bind previous ~f:(fun mounted -> mounted.container_query) in
        let old_config =
          Option.bind previous ~f:(fun mounted ->
            Option.map (View.Expert.describe mounted.view).container_query ~f:(fun old ->
              old.config))
        in
        let generation =
          match old with
          | None -> 1L
          | Some old
            when Option.equal Container_query.Config.equal old_config (Some item.config)
            -> old.generation
          | Some old ->
            if Int64.equal old.generation Int64.max_value
            then fail "container query generation exhausted";
            Int64.succ old.generation
        in
        Container_query.Expert.to_wire item.config ~generation |> value)
    in
    let program_seen =
      Option.value_map previous ~default:(ref (0L, 0L)) ~f:(fun old -> old.program_seen)
    in
    let animation_program =
      Option.map description.animation_program ~f:(fun item ->
        let old = Option.bind previous ~f:(fun mounted -> mounted.animation_program) in
        let old_config =
          Option.bind previous ~f:(fun mounted ->
            Option.map
              (View.Expert.describe mounted.view).animation_program
              ~f:(fun old -> old.config))
        in
        let generation =
          match old with
          | None -> 1L
          | Some old
            when Option.equal Animation.Program.equal old_config (Some item.config) ->
            old.generation
          | Some old ->
            if Int64.equal old.generation Int64.max_value
            then fail "animation program generation exhausted";
            Int64.succ old.generation
        in
        Animation.Expert.program_to_wire item.config ~generation |> value)
    in
    let animation_seen =
      Option.value_map previous ~default:(ref 0L) ~f:(fun old -> old.animation_seen)
    in
    let animation =
      Option.map description.animation ~f:(fun item ->
        let old = Option.bind previous ~f:(fun mounted -> mounted.animation) in
        let old_config =
          Option.bind previous ~f:(fun mounted ->
            Option.map (View.Expert.describe mounted.view).animation ~f:(fun old ->
              old.config))
        in
        let generation =
          match old with
          | None -> 1L
          | Some old
            when Option.equal Animation.Config.equal old_config (Some item.config) ->
            old.generation
          | Some old ->
            if Int64.equal old.generation Int64.max_value
            then fail "animation generation exhausted";
            Int64.succ old.generation
        in
        Animation.Expert.to_wire item.config ~generation |> value)
    in
    let callback =
      match
        ( description.on_click
        , description.editor
        , description.choice
        , description.combobox
        , description.overlay
        , description.tooltip
        , description.commands )
      with
      | Some callback, None, None, None, None, None, None -> Some (Click callback)
      | None, Some editor, None, None, None, None, None ->
        Some
          (match editor.on_event with
           | View.Expert.Editor_events callback -> Editor (editor.config, callback)
           | Picker_query -> Picker_query)
      | None, None, Some choice, None, None, None, None ->
        if Choice.Config.is_disabled choice.config
        then None
        else Some (Choice (choice.config, choice.on_select))
      | None, None, None, Some combo, None, None, None ->
        Some (Combobox (combo.config, combo.on_event))
      | None, None, None, None, Some overlay, None, None ->
        Some (Dismiss (overlay.config, overlay.on_dismiss))
      | None, None, None, None, None, Some tooltip, None ->
        Option.map tooltip.on_open_change ~f:(fun callback ->
          Tooltip (tooltip.config, callback))
      | None, None, None, None, None, None, Some registry ->
        Some (Commands (registry, commands))
      | None, None, None, None, None, None, None -> None
      | _ -> fail "a view cannot combine incompatible handler kinds"
    in
    let callback =
      match description.choice_picker, callback with
      | Some (picker, callback), None ->
        Some (Choice_picker (Choice_picker.Description.config picker, None, callback))
      | None, callback -> callback
      | Some _, Some _ -> fail "picker cannot combine another handler"
    in
    let callback =
      match description.menu, callback with
      | Some menu, None ->
        Option.map menu.on_open_change ~f:(fun callback -> Menu callback)
      | None, callback -> callback
      | Some _, Some _ -> fail "menu cannot combine another handler"
    in
    let callback =
      match description.slider, callback with
      | Some slider, None -> Some (Slider (slider.initial, slider_seen, slider.on_event))
      | None, callback -> callback
      | Some _, Some _ -> fail "slider cannot combine another handler"
    in
    let callback =
      match description.number_input, callback with
      | Some input, None -> Some (Number_input (number_input_seen, input.on_event))
      | None, callback -> callback
      | Some _, Some _ -> fail "numeric input cannot combine another handler"
    in
    let callback =
      match description.otp_input, callback with
      | Some input, None ->
        Some
          (Otp_input (Otp_input.Config.policy input.config, otp_input_seen, input.on_event))
      | None, callback -> callback
      | Some _, Some _ -> fail "OTP input cannot combine another handler"
    in
    let callback =
      match description.color_input, callback with
      | Some input, None -> Some (Color_input (color_input_seen, input.on_event))
      | None, callback -> callback
      | Some _, Some _ -> fail "color input cannot combine another handler"
    in
    let callback =
      match description.calendar, callback with
      | Some calendar, None ->
        Some
          (Calendar
             (Calendar.Config.mode calendar.config, calendar_seen, calendar.on_event))
      | None, callback -> callback
      | Some _, Some _ -> fail "calendar cannot combine another handler"
    in
    let callback =
      match description.carousel_track, callback with
      | Some (config, on_request), None -> Some (Carousel_track (config, on_request))
      | None, callback -> callback
      | Some _, Some _ -> fail "carousel track cannot combine another handler"
    in
    let callback =
      match description.carousel, callback with
      | Some (config, on_request), None -> Some (Carousel (config, on_request))
      | None, callback -> callback
      | Some _, Some _ -> fail "carousel cannot combine another handler"
    in
    let callback =
      match description.rating, callback with
      | Some rating, None ->
        if
          Rating.Config.is_disabled rating.config
          || Rating.Config.is_read_only rating.config
        then None
        else Some (Rating (rating.config, rating.on_request))
      | None, callback -> callback
      | Some _, Some _ -> fail "rating cannot combine another handler"
    in
    let observes_palette view =
      Option.exists view.View.Expert.palette ~f:(fun p -> Option.is_some p.on_change)
    in
    let palette_seen =
      match previous with
      | Some old
        when Bool.equal
               (observes_palette description)
               (observes_palette (View.Expert.describe old.view)) -> old.palette_seen
      | None | Some _ -> ref 0L
    in
    let callback =
      match description.palette, callback with
      | Some palette, None ->
        Some
          (Palette (palette.config, palette.on_dismiss, palette_seen, palette.on_change))
      | None, callback -> callback
      | Some _, Some _ -> fail "palette cannot combine another handler"
    in
    let callback =
      match description.notification, callback with
      | Some item, None -> Some (Toast item.on_dismiss)
      | None, callback -> callback
      | Some _, Some _ -> fail "toast cannot combine another handler"
    in
    let callback =
      match description.command_binding_scope, callback with
      | Some item, None ->
        Some (Command_binding_scope (item.config, binding_seen, item.on_update))
      | None, callback -> callback
      | Some _, Some _ -> fail "binding observer cannot combine another handler"
    in
    let callback =
      match description.highlight_scope, callback with
      | Some item, None ->
        Option.map item.on_update ~f:(fun callback ->
          Highlight_scope (Highlight.Expert.to_wire item.config, callback))
      | None, callback -> callback
      | Some _, Some _ -> fail "highlight scope cannot combine another handler"
    in
    let callback =
      match description.input_region, callback with
      | Some item, None ->
        Some (Input_region (Input_region.Expert.to_wire item.config, item.on_event))
      | None, callback -> callback
      | Some _, Some _ -> fail "input region cannot combine another handler"
    in
    let callback =
      match description.pointer, callback with
      | Some item, None -> Some (Pointer item.on_event)
      | None, callback -> callback
      | Some _, Some _ -> fail "pointer region cannot combine another handler"
    in
    let callback =
      match description.drag_source, callback with
      | Some item, None -> Some (Drag_source item.on_event)
      | None, callback -> callback
      | Some _, Some _ -> fail "drag_source cannot combine another handler"
    in
    let callback =
      match description.drop_target, callback with
      | Some item, None -> Some (Drop_target item.on_event)
      | None, callback -> callback
      | Some _, Some _ -> fail "drop_target cannot combine another handler"
    in
    let callback =
      match description.extension, callback with
      | Some item, None -> Some (Extension (item.config, item.on_event))
      | None, callback -> callback
      | Some _, Some _ -> fail "extension cannot combine another handler"
    in
    let callback =
      match description.split_pane, callback with
      | Some item, None ->
        Option.map item.on_resize ~f:(fun callback -> Split_pane (item.config, callback))
      | None, _ -> callback
      | Some _, Some _ -> fail "split pane cannot combine another handler"
    in
    let callback =
      match description.split_group, callback with
      | Some item, None ->
        Option.map item.on_resize ~f:(fun callback -> Split_group (item.config, callback))
      | None, _ -> callback
      | Some _, Some _ -> fail "split group cannot combine another handler"
    in
    let callback =
      match description.canvas, callback with
      | Some item, None ->
        Option.map item.on_event ~f:(fun callback ->
          Canvas (Canvas.Expert.to_wire item.config ~owner:builder.canvas_owner, callback))
      | None, callback -> callback
      | Some _, Some _ -> fail "canvas cannot combine another handler"
    in
    let callback =
      match description.chart, callback with
      | Some item, None ->
        Option.map item.on_event ~f:(fun callback ->
          Chart (Chart.Expert.to_wire item.config ~owner:builder.chart_owner, callback))
      | None, callback -> callback
      | Some _, Some _ -> fail "chart cannot combine another handler"
    in
    let old_diff =
      Option.bind previous ~f:(fun mounted ->
        Option.bind (document_description builder mounted.view) ~f:(fun item ->
          Document.Config.diff item.config))
    in
    let next_diff =
      Option.bind description.document ~f:(fun item -> Document.Config.diff item.config)
    in
    let old_diff_epoch =
      Option.value_map previous ~default:0L ~f:(fun old -> old.document_diff_epoch)
    in
    let document_diff_epoch =
      if Option.equal Document.Diff.Config.equal old_diff next_diff
      then old_diff_epoch
      else (
        if Int64.equal old_diff_epoch Int64.max_value
        then fail "document diff configuration epoch exhausted";
        Int64.succ old_diff_epoch)
    in
    let preview_settings document =
      match document with
      | None -> None, false
      | Some (item : _ View.Expert.document) ->
        Document.Config.max_lines item.config, Option.is_some item.on_preview
    in
    let previous_preview =
      Option.bind previous ~f:(fun old -> document_description builder old.view)
    in
    let old_limit, old_observe = preview_settings previous_preview in
    let next_limit, next_observe = preview_settings description.document in
    let old_preview_epoch =
      Option.value_map previous ~default:0L ~f:(fun old -> old.document_preview_epoch)
    in
    let document_preview_epoch =
      if
        Option.equal Int.equal old_limit next_limit && Bool.equal old_observe next_observe
      then old_preview_epoch
      else (
        if Int64.equal old_preview_epoch Int64.max_value
        then fail "document preview epoch exhausted";
        Int64.succ old_preview_epoch)
    in
    let profile_settings document =
      Option.bind document ~f:(fun (item : _ View.Expert.document) ->
        Option.map item.profile ~f:fst)
    in
    let old_profile = profile_settings previous_preview in
    let next_profile = profile_settings description.document in
    Option.iter next_profile ~f:(fun next ->
      if not (Gpuio_protocol.Document_profile_wire.Instance.valid next)
      then fail "invalid document profile instance";
      Option.iter old_profile ~f:(fun old ->
        if
          Gpuio_protocol.Extension_wire.Schema.equal old.schema next.schema
          && Int64.(next.generation < old.generation)
        then fail "document profile generation decreased"));
    let old_profile_epoch =
      Option.value_map previous ~default:0L ~f:(fun old -> old.document_profile_epoch)
    in
    let document_profile_epoch =
      if
        Option.equal
          Gpuio_protocol.Document_profile_wire.Instance.equal
          old_profile
          next_profile
      then old_profile_epoch
      else (
        if Int64.equal old_profile_epoch Int64.max_value
        then fail "document profile epoch exhausted";
        Int64.succ old_profile_epoch)
    in
    let actions_settings document =
      match document with
      | None -> Document.Actions.Config.default, false
      | Some (item : _ View.Expert.document) ->
        Document.Config.actions item.config, Option.is_some item.on_action
    in
    let old_actions, old_actions_observe = actions_settings previous_preview in
    let next_actions, next_actions_observe = actions_settings description.document in
    let old_actions_epoch =
      Option.value_map previous ~default:0L ~f:(fun old -> old.document_actions_epoch)
    in
    let document_actions_epoch =
      if
        Document.Actions.Config.equal old_actions next_actions
        && Bool.equal old_actions_observe next_actions_observe
      then old_actions_epoch
      else (
        if Int64.equal old_actions_epoch Int64.max_value
        then fail "document actions epoch exhausted";
        Int64.succ old_actions_epoch)
    in
    let callback =
      match description.document, callback with
      | Some item, None ->
        if
          ((not (List.is_empty (Document.Actions.Config.code next_actions)))
           || not (List.is_empty (Document.Actions.Config.table next_actions)))
          && Option.is_none item.on_action
        then fail "custom document actions require on_action";
        if
          (Option.is_some item.on_action || Option.is_some item.profile)
          && not
               (Document.Mode.equal (Document.Config.mode item.config) Markdown
                || Document.Mode.equal (Document.Config.mode item.config) Html)
        then fail "document action callback requires Markdown or HTML";
        if Option.is_some item.on_diff && Option.is_none next_diff
        then fail "document diff callback requires explicit diff configuration";
        if
          Option.is_some item.on_preview
          && not
               ((Document.Mode.equal (Document.Config.mode item.config) Markdown
                 || Document.Mode.equal (Document.Config.mode item.config) Html)
                && Document.Layout.equal (Document.Config.layout item.config) Flow)
        then fail "document preview callback requires Markdown/HTML Flow";
        if
          Option.is_none item.on_navigate
          && Option.is_none item.on_diff
          && Option.is_none item.on_preview
          && Option.is_none item.on_action
          && Option.is_none item.profile
        then None
        else
          Some
            (Document
               { source = Document.Config.source item.config
               ; diff_config = next_diff
               ; diff_epoch = document_diff_epoch
               ; on_navigate = item.on_navigate
               ; on_diff = item.on_diff
               ; preview_epoch = document_preview_epoch
               ; preview_max_lines = next_limit
               ; on_action = item.on_action
               ; actions_epoch = document_actions_epoch
               ; actions_config = next_actions
               ; profile =
                   Option.map item.profile ~f:(fun (instance, callback) ->
                     document_profile_epoch, instance, callback)
               ; on_preview = item.on_preview
               })
      | None, callback -> callback
      | Some _, Some _ -> fail "document cannot combine another handler"
    in
    let callback =
      match description.image, callback with
      | Some image, None -> Option.map image.on_change ~f:(fun callback -> Image callback)
      | None, callback -> callback
      | Some _, Some _ -> fail "image cannot combine another handler"
    in
    let callback =
      match description.animation, animation, callback with
      | Some item, Some config, None ->
        Option.map item.on_event ~f:(fun callback ->
          Animation (config.generation, animation_seen, callback))
      | None, None, callback -> callback
      | Some _, _, Some _ -> fail "animation cannot combine another handler"
      | Some _, None, None | None, Some _, _ -> fail "missing animation configuration"
    in
    let callback =
      match description.animation_program, animation_program, callback with
      | Some item, Some config, None ->
        Option.map item.on_event ~f:(fun callback ->
          Animation_program (config.generation, program_seen, callback))
      | None, None, callback -> callback
      | Some _, _, Some _ -> fail "animation program cannot combine another handler"
      | Some _, None, None | None, Some _, _ ->
        fail "missing animation program configuration"
    in
    let callback =
      match description.container_query, container_query, callback with
      | Some item, Some config, None ->
        Option.map item.on_select ~f:(fun callback ->
          Container_query (config, query_seen, callback))
      | None, None, callback -> callback
      | Some _, _, Some _ -> fail "container query cannot combine another handler"
      | Some _, None, None | None, Some _, _ ->
        fail "missing container query configuration"
    in
    let list_identity =
      Option.map description.virtual_list ~f:(fun list ->
        List_identity.prepare
          (Option.bind previous ~f:(fun old -> old.list_identity))
          list.order
        |> value)
    in
    let table = Option.bind description.virtual_list ~f:(fun list -> list.table) in
    let old_table_config = Option.bind previous ~f:(fun old -> old.table_config) in
    let old_table_behavior = Option.bind previous ~f:(fun old -> old.table_behavior) in
    let table_behavior =
      Option.bind table ~f:(fun table -> Table.Expert.behavior_to_wire table.config)
    in
    let old_table_appearance =
      Option.bind previous ~f:(fun old -> old.table_appearance)
    in
    let table_appearance =
      Option.bind table ~f:(fun table ->
        Table.Expert.appearance_to_wire table.config ~theme:builder.theme |> value)
    in
    let presentation style =
      Option.value_map style ~default:[] ~f:(fun style ->
        Style.Expert.to_wire style ~theme:builder.theme |> value)
    in
    let table_header_style =
      presentation
        (Option.map description.table_header_style ~f:Table_presentation.Header.style)
    in
    let table_row_style =
      presentation
        (Option.map description.table_row_style ~f:Table_presentation.Row.style)
    in
    let header_targets view =
      List.filter_map view.View.Expert.children ~f:(fun child ->
        let child = View.Expert.describe child in
        Option.map child.table_header ~f:(fun target ->
          Option.value_exn child.key, target))
      |> List.sort ~compare:(fun (a, _) (b, _) -> Key.compare a b)
    in
    let header_targets_changed =
      not
        (List.equal
           [%equal: Key.t * Table_header.Target.t]
           (Option.value_map previous ~default:[] ~f:(fun old ->
              header_targets (View.Expert.describe old.view)))
           (header_targets description))
    in
    let table_config =
      Option.map table ~f:(fun table ->
        let candidate =
          Table.Expert.to_wire
            table.config
            ~schema_revision:1L
            ~query_generation:table.query_generation
          |> value
        in
        match old_table_config with
        | None -> candidate
        | Some old ->
          if Int64.(candidate.query_generation < old.query_generation)
          then fail "table query generation went backwards";
          let changed =
            (not (TW.Schema.equal old.schema candidate.schema))
            || (not (Option.equal TW.Sort.equal old.sort candidate.sort))
            || (not (Option.equal TW.Behavior.equal old_table_behavior table_behavior))
            || (not (TW.Appearance.geometry_equal old_table_appearance table_appearance))
            || header_targets_changed
          in
          if changed && Int64.equal old.schema_revision Int64.max_value
          then fail "table schema revision exhausted";
          { candidate with
            schema_revision =
              (if changed then Int64.succ old.schema_revision else old.schema_revision)
          })
    in
    let callback =
      match description.virtual_list, list_identity, callback with
      | Some list, Some identity, None ->
        if
          Option.is_some list.on_viewport
          || Option.is_some list.on_retain
          || Option.is_some list.on_tree_input
          || Option.is_some list.list_input
        then Some (Virtual_list (identity, list, table_config, None))
        else None
      | None, None, callback -> callback
      | _ -> fail "incompatible virtual list callback"
    in
    let rotate_handler =
      binding_changed
      || (match description.highlight_scope, previous with
          | Some item, Some mounted ->
            Option.exists
              (View.Expert.describe mounted.view).highlight_scope
              ~f:(fun old -> not (Highlight.Config.equal item.config old.config))
          | None, _ | Some _, None -> false)
      || (match description.input_region, previous with
          | Some item, Some mounted ->
            Option.exists (View.Expert.describe mounted.view).input_region ~f:(fun old ->
              not (Input_region.Config.equal item.config old.config))
          | None, _ | Some _, None -> false)
      || (match table_config, old_table_config with
          | Some next, Some old ->
            not (Int64.equal next.query_generation old.query_generation)
          | Some _, None | None, _ -> false)
      || (match description.virtual_list, previous with
          | Some list, Some mounted ->
            let old =
              Option.exists
                (View.Expert.describe mounted.view).virtual_list
                ~f:(fun list -> Option.is_some list.on_tree_input)
            in
            (not (Bool.equal old (Option.is_some list.on_tree_input)))
            || Option.exists
                 (View.Expert.describe mounted.view).virtual_list
                 ~f:(fun previous -> not (Bool.equal previous.tree_moves list.tree_moves))
          | None, _ | Some _, None -> false)
      || (match description.extension, previous with
          | Some item, Some mounted ->
            Option.exists (View.Expert.describe mounted.view).extension ~f:(fun old ->
              not (Wire.Extension.Config.equal old.config item.config))
          | None, _ | Some _, None -> false)
      || (match description.split_pane, previous with
          | Some item, Some mounted ->
            Option.exists (View.Expert.describe mounted.view).split_pane ~f:(fun old ->
              not (Split_pane.Config.equal old.config item.config))
          | None, _ | Some _, None -> false)
      || (match description.split_group, previous with
          | Some item, Some mounted ->
            Option.exists (View.Expert.describe mounted.view).split_group ~f:(fun old ->
              not (Split_group.Config.equal old.config item.config))
          | None, _ | Some _, None -> false)
      || (match description.canvas, previous with
          | Some item, Some mounted ->
            Option.exists (View.Expert.describe mounted.view).canvas ~f:(fun old ->
              not (Canvas.Config.equal old.config item.config))
          | None, _ | Some _, None -> false)
      || (match description.chart, previous with
          | Some item, Some mounted ->
            Option.exists (View.Expert.describe mounted.view).chart ~f:(fun old ->
              not (Chart.Config.equal old.config item.config))
          | None, _ | Some _, None -> false)
      || (match description.document, previous with
          | Some document, Some mounted ->
            Option.exists (document_description builder mounted.view) ~f:(fun old ->
              not
                (Text_source.Handle.equal
                   (Document.Config.source old.config)
                   (Document.Config.source document.config)))
          | None, _ | Some _, None -> false)
      || (match description.image, previous with
          | Some image, Some mounted ->
            Option.exists (View.Expert.describe mounted.view).image ~f:(fun old ->
              not
                (Asset.Handle.equal
                   (Image.Config.asset old.config)
                   (Image.Config.asset image.config)))
          | None, _ | Some _, None -> false)
      || (match description.menu, previous with
          | Some menu, Some mounted ->
            Option.exists (View.Expert.describe mounted.view).menu ~f:(fun old ->
              (not (List.equal Menu.equal old.menus menu.menus))
              || not (Menu.Expert.equal_presentation old.presentation menu.presentation))
          | None, _ | Some _, None -> false)
      || (match description.choice_picker, previous with
          | Some (picker, _), Some mounted ->
            Option.exists
              (View.Expert.describe mounted.view).choice_picker
              ~f:(fun (old, _) ->
                not
                  (Bool.equal
                     (Choice_picker.Config.is_disabled
                        (Choice_picker.Description.config old))
                     (Choice_picker.Config.is_disabled
                        (Choice_picker.Description.config picker))))
          | None, _ | Some _, None -> false)
      || (match description.combobox, previous with
          | Some combo, Some mounted ->
            Option.exists (View.Expert.describe mounted.view).combobox ~f:(fun old ->
              not
                (Bool.equal
                   (Choice.Config.is_disabled (Combobox.Config.choices old.config))
                   (Choice.Config.is_disabled (Combobox.Config.choices combo.config))))
          | None, _ | Some _, None -> false)
      ||
      match description.tooltip, previous with
      | Some tooltip, Some mounted ->
        Option.exists (View.Expert.describe mounted.view).tooltip ~f:(fun old ->
          not
            (Bool.equal
               (Tooltip.Expert.is_disabled old.config)
               (Tooltip.Expert.is_disabled tooltip.config)))
      | None, _ | Some _, None -> false
    in
    let rotate_handler =
      rotate_handler
      ||
      match description.palette, previous with
      | Some palette, Some mounted ->
        Option.exists (View.Expert.describe mounted.view).palette ~f:(fun old ->
          not
            (Bool.equal (Option.is_some old.on_change) (Option.is_some palette.on_change)))
      | None, _ | Some _, None -> false
    in
    let handler =
      match old_handler, callback with
      | Some handler, Some _ when rotate_handler ->
        builder.handlers <- Allocator.release builder.handlers (handler_slot handler);
        Some (new_handler builder)
      | Some handler, Some _ -> Some handler
      | None, Some _ -> Some (new_handler builder)
      | Some handler, None ->
        builder.handlers <- Allocator.release builder.handlers (handler_slot handler);
        None
      | None, None -> None
    in
    (match handler, callback with
     | Some handler, Some callback ->
       builder.bindings
       <- Map.set
            builder.bindings
            ~key:(node_slot id)
            ~data:{ node = id; handler; callback }
     | None, None -> builder.bindings <- Map.remove builder.bindings (node_slot id)
     | Some _, None | None, Some _ -> assert false);
    let old_viewport =
      Option.bind previous ~f:(fun mounted -> mounted.calendar_viewport_handler)
    in
    let on_viewport_change =
      Option.bind description.calendar ~f:(fun calendar -> calendar.on_viewport_change)
    in
    let calendar_viewport_handler =
      match old_viewport, on_viewport_change with
      | Some handler, Some _ -> Some handler
      | None, Some _ -> Some (new_handler builder)
      | Some handler, None ->
        builder.handlers <- Allocator.release builder.handlers (handler_slot handler);
        None
      | None, None -> None
    in
    (match calendar_viewport_handler, on_viewport_change with
     | Some handler, Some on_change ->
       let seen =
         match Map.find builder.calendar_viewport_bindings (node_slot id) with
         | Some binding when Handler_id.equal binding.handler handler -> binding.seen
         | Some _ | None -> ref (-1L)
       in
       builder.calendar_viewport_bindings
       <- Map.set
            builder.calendar_viewport_bindings
            ~key:(node_slot id)
            ~data:Calendar_viewport_binding.{ node = id; handler; seen; on_change }
     | None, None ->
       builder.calendar_viewport_bindings
       <- Map.remove builder.calendar_viewport_bindings (node_slot id)
     | Some _, None | None, Some _ -> assert false);
    let old_hover = Option.bind previous ~f:(fun mounted -> mounted.hover_handler) in
    let hover_handler =
      match old_hover, description.on_hover with
      | Some handler, Some _ -> Some handler
      | None, Some _ -> Some (new_handler builder)
      | Some handler, None ->
        builder.handlers <- Allocator.release builder.handlers (handler_slot handler);
        None
      | None, None -> None
    in
    (match hover_handler, description.on_hover with
     | Some handler, Some on_change ->
       builder.hover_bindings
       <- Map.set
            builder.hover_bindings
            ~key:(node_slot id)
            ~data:Hover_binding.{ node = id; handler; on_change }
     | None, None ->
       builder.hover_bindings <- Map.remove builder.hover_bindings (node_slot id)
     | Some _, None | None, Some _ -> assert false);
    let text_content =
      Option.map description.text_content ~f:(fun content ->
        Text_content.Expert.to_wire content ~theme:builder.theme |> value)
    in
    let compact_table_text =
      match description.kind with
      | Text -> description.table_cell
      | _ -> None
    in
    (match previous with
     | None ->
       let text = if Option.is_some text_content then "" else description.text in
       (match compact_table_text with
        | Some cell ->
          emit builder (Create_table_text (id, Table.Expert.cell_to_wire cell))
        | None -> emit builder (Create (id, kind description.kind, text, handler)))
     | Some mounted ->
       if
         Option.is_none description.editor
         && Option.is_none description.combobox
         && Option.is_none compact_table_text
         && Option.is_none text_content
         && (Option.is_some mounted.text_content
             || not
                  (String.equal (View.Expert.describe mounted.view).text description.text)
            )
       then emit builder (Set_text (id, description.text));
       if not (Option.equal Handler_id.equal old_handler handler)
       then emit builder (Bind (id, handler)));
    if not (Option.equal Handler_id.equal old_hover hover_handler)
    then emit builder (Set_hover_observer (id, hover_handler));
    if not (Option.equal Handler_id.equal old_viewport calendar_viewport_handler)
    then emit builder (Set_calendar_viewport_observer (id, calendar_viewport_handler));
    if
      not
        (Option.equal
           Gpuio_protocol.Text_content_wire.equal
           text_content
           (Option.bind previous ~f:(fun mounted -> mounted.text_content)))
    then
      Option.iter text_content ~f:(fun content ->
        emit builder (Set_styled_text (id, content)));
    let previous_shimmer =
      Option.bind previous ~f:(fun mounted ->
        (View.Expert.describe mounted.view).text_shimmer)
    in
    if
      not
        (Option.equal Text_shimmer.Config.equal previous_shimmer description.text_shimmer)
    then
      emit
        builder
        (Set_text_shimmer
           (id, Option.map description.text_shimmer ~f:Text_shimmer.Expert.to_wire));
    Option.iter description.link ~f:(fun config ->
      let previous_config =
        Option.bind previous ~f:(fun mounted -> (View.Expert.describe mounted.view).link)
      in
      if not (Option.equal Link.Config.equal previous_config (Some config))
      then emit builder (Set_link (id, Link.Expert.to_wire config)));
    if
      Option.is_some description.commands
      && not (List.equal Wire.Command.equal old_commands commands)
    then emit builder (Set_commands (id, commands));
    (* An empty registry still needs its metadata on first mount. *)
    if
      Option.is_some description.commands
      && Option.is_none previous
      && List.is_empty commands
    then emit builder (Set_commands (id, []));
    Option.iter description.command_ref ~f:(fun command ->
      let old =
        Option.bind previous ~f:(fun mounted ->
          (View.Expert.describe mounted.view).command_ref)
      in
      if not (Option.equal Ui_command.Id.equal old (Some command))
      then emit builder (Set_command_ref (id, Ui_command.Id.to_string command)));
    let menu =
      Option.map description.menu ~f:(fun menu ->
        { Wire.Menu.presentation =
            (match menu.presentation with
             | Menu.Expert.Button -> Button
             | Context -> Context
             | Bar -> Bar
             | Platform_bar -> Platform_bar
             | Editor_context -> Editor_context)
        ; menus = List.map menu.menus ~f:Menu.Expert.to_wire
        })
    in
    if
      not
        (Option.equal
           Wire.Menu.equal
           menu
           (Option.bind previous ~f:(fun mounted -> mounted.menu)))
    then Option.iter menu ~f:(fun config -> emit builder (Set_menu (id, config)));
    Option.iter description.drag_source ~f:(fun item ->
      let old =
        Option.bind previous ~f:(fun mounted ->
          Option.map (View.Expert.describe mounted.view).drag_source ~f:(fun item ->
            item.config))
      in
      if not (Option.equal Drag_and_drop.Source.equal old (Some item.config))
      then
        emit
          builder
          (Set_drag_source (id, Drag_and_drop.Expert.source_to_wire item.config)));
    Option.iter description.drop_target ~f:(fun item ->
      let old =
        Option.bind previous ~f:(fun mounted ->
          Option.map (View.Expert.describe mounted.view).drop_target ~f:(fun item ->
            item.config))
      in
      if not (Option.equal Drag_and_drop.Target.equal old (Some item.config))
      then
        emit
          builder
          (Set_drop_target (id, Drag_and_drop.Expert.target_to_wire item.config)));
    if binding_changed
    then
      emit
        builder
        (Set_command_binding
           (id, Option.map binding_config ~f:Command_binding.Expert.to_wire));
    Option.iter description.highlight_scope ~f:(fun item ->
      let old =
        Option.bind previous ~f:(fun mounted ->
          Option.map (View.Expert.describe mounted.view).highlight_scope ~f:(fun item ->
            item.config))
      in
      if not (Option.equal Highlight.Config.equal old (Some item.config))
      then emit builder (Set_highlight_scope (id, Highlight.Expert.to_wire item.config)));
    Option.iter description.input_region ~f:(fun item ->
      let old =
        Option.bind previous ~f:(fun mounted ->
          Option.map (View.Expert.describe mounted.view).input_region ~f:(fun item ->
            item.config))
      in
      if not (Option.equal Input_region.Config.equal old (Some item.config))
      then emit builder (Set_input_region (id, Input_region.Expert.to_wire item.config)));
    Option.iter description.pointer ~f:(fun item ->
      let old =
        Option.bind previous ~f:(fun mounted ->
          Option.map (View.Expert.describe mounted.view).pointer ~f:(fun item ->
            item.config))
      in
      if not (Option.equal Pointer.Config.equal old (Some item.config))
      then emit builder (Set_pointer (id, Pointer.Expert.to_wire item.config)));
    Option.iter description.notification ~f:(fun item ->
      let old =
        Option.bind previous ~f:(fun mounted ->
          Option.map (View.Expert.describe mounted.view).notification ~f:(fun item ->
            item.config))
      in
      if not (Option.equal Toast.Config.equal old (Some item.config))
      then emit builder (Set_toast (id, Toast.Expert.to_wire item.config)));
    Option.iter description.toast_stack ~f:(fun config ->
      let old =
        Option.bind previous ~f:(fun mounted ->
          (View.Expert.describe mounted.view).toast_stack)
      in
      let next_wire = Toast.Expert.stack_to_wire config in
      if
        not
          (Option.equal
             Wire.Toast_stack.equal
             (Option.map old ~f:Toast.Expert.stack_to_wire)
             (Some next_wire))
      then emit builder (Set_toast_stack (id, next_wire));
      let placement = Toast.Expert.placement config in
      let old_placement = Option.bind old ~f:Toast.Expert.placement in
      if
        not
          (Option.equal Gpuio_protocol.Toast_placement_wire.equal old_placement placement)
      then emit builder (Set_toast_placement (id, placement));
      let layering = Toast.Expert.layering config in
      let old_layering = Option.bind old ~f:Toast.Expert.layering in
      if not (Option.equal Gpuio_protocol.Toast_layering_wire.equal old_layering layering)
      then emit builder (Set_toast_layering (id, layering));
      let motion = Toast.Expert.motion config in
      let old_motion = Option.bind old ~f:Toast.Expert.motion in
      if not (Option.equal Gpuio_protocol.Toast_motion_wire.equal old_motion motion)
      then emit builder (Set_toast_motion (id, motion)));
    let accessibility = description.accessibility in
    let old_accessibility =
      Option.bind previous ~f:(fun mounted ->
        (View.Expert.describe mounted.view).accessibility)
    in
    if not (Option.equal Accessibility.equal accessibility old_accessibility)
    then
      emit
        builder
        (Set_accessibility (id, Option.map accessibility ~f:Accessibility.Expert.to_wire));
    let old_header =
      Option.bind previous ~f:(fun old -> (View.Expert.describe old.view).table_header)
    in
    if not (Option.equal Table_header.Target.equal old_header description.table_header)
    then
      emit
        builder
        (Set_table_header
           (id, Option.map description.table_header ~f:Table_header.Expert.target_to_wire));
    if
      not
        (List.equal
           Wire.Style.equal
           table_header_style
           (Option.value_map previous ~default:[] ~f:(fun old -> old.table_header_style)))
    then emit builder (Set_table_header_style (id, table_header_style));
    if
      not
        (List.equal
           Wire.Style.equal
           table_row_style
           (Option.value_map previous ~default:[] ~f:(fun old -> old.table_row_style)))
    then emit builder (Set_table_row_style (id, table_row_style));
    Option.iter description.table_cell ~f:(fun cell ->
      let old =
        Option.bind previous ~f:(fun old -> (View.Expert.describe old.view).table_cell)
      in
      if not (Option.equal Table.Cell.equal old (Some cell))
      then (
        let wire = Table.Expert.cell_to_wire cell in
        match compact_table_text, previous with
        | Some _, Some _ -> emit builder (Set_table_text (id, wire))
        | Some _, None -> ()
        | None, _ -> emit builder (Set_table_cell (id, wire))));
    Option.iter table_config ~f:(fun config ->
      if not (Option.equal TW.Config.equal old_table_config (Some config))
      then emit builder (Set_table (id, config)));
    if not (Option.equal TW.Behavior.equal old_table_behavior table_behavior)
    then emit builder (Set_table_behavior (id, table_behavior));
    if not (Option.equal TW.Appearance.equal old_table_appearance table_appearance)
    then emit builder (Set_table_appearance (id, table_appearance));
    let tree_input description =
      Option.exists description.View.Expert.virtual_list ~f:(fun list ->
        Option.is_some list.on_tree_input)
    in
    let input = tree_input description in
    if
      input
      && not
           (Option.exists description.accessibility ~f:(fun metadata ->
              match (Accessibility.Expert.to_wire metadata).role with
              | Some (Tree _) -> true
              | None
              | Some
                  ( Group
                  | Label
                  | Link
                  | Separator
                  | Description_list
                  | Term
                  | Definition
                  | Status
                  | Alert
                  | Image
                  | Heading _
                  | Navigation
                  | Tree_item _
                  | List_box _
                  | Option_item _
                  | Table _
                  | Row_group
                  | Table_row _
                  | Table_cell _
                  | Column_header _
                  | Row_header _
                  | Caption
                  | Toolbar _
                  | Radio_group _
                  | Log ) -> false))
    then fail "native tree input requires Tree accessibility on its managed list root";
    let old_input =
      Option.exists previous ~f:(fun mounted ->
        tree_input (View.Expert.describe mounted.view))
    in
    if not (Bool.equal input old_input) then emit builder (Set_tree_input (id, input));
    let moves description =
      Option.exists description.View.Expert.virtual_list ~f:(fun list -> list.tree_moves)
    in
    let enabled_moves = moves description in
    if enabled_moves && not input then fail "native tree moves require tree input";
    let old_moves =
      Option.exists previous ~f:(fun mounted -> moves (View.Expert.describe mounted.view))
    in
    if not (Bool.equal enabled_moves old_moves)
    then emit builder (Set_tree_moves (id, enabled_moves));
    let track = Option.map description.carousel_track ~f:fst in
    let old_track =
      Option.bind previous ~f:(fun mounted ->
        Option.map (View.Expert.describe mounted.view).carousel_track ~f:fst)
    in
    Option.iter track ~f:(fun config ->
      if
        Option.exists old_track ~f:(fun old ->
          not (Wire.Carousel_track.Config.can_replace config old))
      then
        fail
          "carousel track model revision and lineage must advance when policy or \
           collection changes";
      if not (Option.equal Wire.Carousel_track.Config.equal track old_track)
      then emit builder (Set_carousel_track (id, config)));
    let track_motion = description.carousel_track_motion in
    let old_track_motion =
      Option.bind previous ~f:(fun mounted ->
        (View.Expert.describe mounted.view).carousel_track_motion)
    in
    if not (Option.equal Wire.Carousel_track.Motion.equal track_motion old_track_motion)
    then emit builder (Set_carousel_track_motion (id, track_motion));
    let carousel = Option.map description.carousel ~f:fst in
    let old_carousel =
      Option.bind previous ~f:(fun mounted ->
        Option.map (View.Expert.describe mounted.view).carousel ~f:fst)
    in
    Option.iter carousel ~f:(fun config ->
      if
        Option.exists old_carousel ~f:(fun old ->
          not (Wire.Carousel.Config.can_replace config old))
      then fail "carousel model revision must advance when selection or policy changes";
      if not (Option.equal Wire.Carousel.Config.equal carousel old_carousel)
      then emit builder (Set_carousel (id, config)));
    let navigation = description.navigation_stack in
    let old_navigation =
      Option.bind previous ~f:(fun mounted ->
        (View.Expert.describe mounted.view).navigation_stack)
    in
    if not (Option.equal Wire.Navigation_stack.Config.equal navigation old_navigation)
    then
      Option.iter navigation ~f:(fun config ->
        emit builder (Set_navigation_stack (id, config)));
    let old_query = Option.bind previous ~f:(fun mounted -> mounted.container_query) in
    if not (Option.equal Wire.Container_query.Config.equal container_query old_query)
    then
      Option.iter container_query ~f:(fun config ->
        emit builder (Set_container_query (id, config)));
    let old_program =
      Option.bind previous ~f:(fun mounted -> mounted.animation_program)
    in
    if
      not (Option.equal Wire.Animation_program.Config.equal animation_program old_program)
    then
      Option.iter animation_program ~f:(fun config ->
        emit builder (Set_animation_program (id, config)));
    let old_animation = Option.bind previous ~f:(fun mounted -> mounted.animation) in
    if not (Option.equal Wire.Animation.Config.equal animation old_animation)
    then
      Option.iter animation ~f:(fun config -> emit builder (Set_animation (id, config)));
    Option.iter description.extension ~f:(fun item ->
      let old =
        Option.bind previous ~f:(fun mounted ->
          Option.map (View.Expert.describe mounted.view).extension ~f:(fun old ->
            old.config))
      in
      if Option.exists old ~f:(fun old -> Int64.(item.config.generation < old.generation))
      then fail "extension generation must not decrease";
      if not (Option.equal Wire.Extension.Config.equal old (Some item.config))
      then emit builder (Set_extension (id, item.config)));
    let split_group =
      Option.map description.split_group ~f:(fun item ->
        ( Split_group.Expert.to_wire item.config
        , Split_group.Expert.appearance_to_wire item.appearance ~theme:builder.theme
          |> value ))
    in
    let old_group = Option.bind previous ~f:(fun mounted -> mounted.split_group) in
    Option.iter split_group ~f:(fun (config, appearance) ->
      if
        Option.exists old_group ~f:(fun (old, _) ->
          Int64.(config.reset_generation < old.reset_generation))
      then fail "split group reset_generation must not decrease";
      if
        not
          (Option.equal
             (fun (config, appearance) (other_config, other_appearance) ->
                Gpuio_protocol.Split_group_wire.Config.equal config other_config
                && Wire.Split_group_appearance.equal appearance other_appearance)
             old_group
             split_group)
      then emit builder (Set_split_group (id, config, appearance)));
    Option.iter description.split_pane ~f:(fun item ->
      let old =
        Option.bind previous ~f:(fun mounted ->
          Option.map (View.Expert.describe mounted.view).split_pane ~f:(fun old ->
            old.config))
      in
      if
        Option.exists old ~f:(fun old ->
          Int64.(
            Split_pane.Expert.generation item.config < Split_pane.Expert.generation old))
      then fail "split-pane reset_generation must not decrease";
      if not (Option.equal Split_pane.Config.equal old (Some item.config))
      then emit builder (Set_split (id, Split_pane.Expert.to_wire item.config)));
    Option.iter description.canvas ~f:(fun item ->
      let old =
        Option.bind previous ~f:(fun mounted ->
          Option.map (View.Expert.describe mounted.view).canvas ~f:(fun old -> old.config))
      in
      if not (Option.equal Canvas.Config.equal old (Some item.config))
      then
        emit
          builder
          (Set_canvas (id, Canvas.Expert.to_wire item.config ~owner:builder.canvas_owner)));
    Option.iter description.chart ~f:(fun item ->
      let old =
        Option.bind previous ~f:(fun mounted ->
          Option.map (View.Expert.describe mounted.view).chart ~f:(fun old -> old.config))
      in
      if not (Option.equal Chart.Config.equal old (Some item.config))
      then
        emit
          builder
          (Set_chart (id, Chart.Expert.to_wire item.config ~owner:builder.chart_owner)));
    let markdown_options view =
      Option.value_map
        (document_description builder view)
        ~default:Document.Markdown_options.default
        ~f:(fun document -> Document.Config.markdown_options document.config)
    in
    let document_markdown_options = markdown_options view in
    let old_markdown_options =
      Option.value_map previous ~default:Document.Markdown_options.default ~f:(fun old ->
        markdown_options old.view)
    in
    if
      not (Document.Markdown_options.equal old_markdown_options document_markdown_options)
    then
      emit
        builder
        (Set_document_markdown_options
           (id, Document.Markdown_options.Expert.to_wire document_markdown_options));
    let document_text_style =
      Option.bind description.document ~f:(fun document ->
        Document.Config.text_style document.config)
      |> Option.map ~f:(fun style ->
        Document.Style.Expert.to_wire style ~theme:builder.theme |> value)
    in
    let old_document_text_style =
      Option.bind previous ~f:(fun old -> old.document_text_style)
    in
    if
      not
        (Option.equal
           Wire.Document_style.equal
           old_document_text_style
           document_text_style)
    then emit builder (Set_document_text_style (id, document_text_style));
    Option.iter description.document ~f:(fun document ->
      let old =
        Option.bind previous ~f:(fun mounted ->
          Option.map (document_description builder mounted.view) ~f:(fun old ->
            old.config))
      in
      let to_wire config =
        Document.Expert.to_wire
          config
          ~owner:builder.document_owner
          ~asset_owner:builder.asset_owner
      in
      let wire = to_wire document.config in
      if
        not
          (Option.equal
             Gpuio_protocol.Document_wire.Config.equal
             (Option.map old ~f:to_wire)
             (Some wire))
      then emit builder (Set_document (id, wire));
      let markdown config =
        Document.Selection_format.equal (Document.Config.selection_format config) Markdown
      in
      let next = markdown document.config in
      let previous = Option.value_map old ~default:false ~f:markdown in
      if not (Bool.equal previous next)
      then emit builder (Set_document_selection_format (id, next)));
    if not (Int64.equal old_profile_epoch document_profile_epoch)
    then
      emit
        builder
        (Set_document_profile
           ( id
           , { Gpuio_protocol.Document_profile_wire.Config.epoch = document_profile_epoch
             ; instance = next_profile
             } ));
    if not (Int64.equal old_actions_epoch document_actions_epoch)
    then
      emit
        builder
        (Set_document_actions
           ( id
           , Document.Actions.Expert.to_wire
               next_actions
               ~epoch:document_actions_epoch
               ~observe:next_actions_observe ));
    if not (Int64.equal old_preview_epoch document_preview_epoch)
    then
      emit
        builder
        (Set_document_preview
           ( id
           , { Gpuio_protocol.Document_preview_wire.Config.epoch = document_preview_epoch
             ; max_lines = Option.map next_limit ~f:Int64.of_int
             ; observe = next_observe
             } ));
    if not (Int64.equal old_diff_epoch document_diff_epoch)
    then
      emit
        builder
        (Set_document_diff
           (id, document_diff_epoch, Option.map next_diff ~f:Document.Diff.Expert.to_wire));
    if Option.is_none description.avatar && Option.is_none description.spinner
    then
      Option.iter description.image ~f:(fun image ->
        let old =
          Option.bind previous ~f:(fun mounted ->
            Option.map (View.Expert.describe mounted.view).image ~f:(fun image ->
              image.config))
        in
        if not (Option.equal Image.Config.equal old (Some image.config))
        then
          emit
            builder
            (Set_image (id, Image.Expert.to_wire image.config ~owner:builder.asset_owner)));
    Option.iter description.slider ~f:(fun slider ->
      let old =
        Option.bind previous ~f:(fun mounted ->
          (View.Expert.describe mounted.view).slider)
      in
      if
        not
          (Option.exists old ~f:(fun old ->
             Slider.Config.equal old.config slider.config
             && Slider.Value.equal old.initial slider.initial))
      then
        emit
          builder
          (Set_slider
             ( id
             , Slider.Expert.config_to_wire slider.config
             , Slider.Expert.value_to_wire slider.initial )));
    Option.iter description.number_input ~f:(fun number_input ->
      let old =
        Option.bind previous ~f:(fun mounted ->
          (View.Expert.describe mounted.view).number_input)
      in
      if
        not
          (Option.exists old ~f:(fun old ->
             Wire.Number_input.Config.equal
               (Number_input.Expert.config_to_wire old.config)
               (Number_input.Expert.config_to_wire number_input.config)
             && Number_input.Value.equal old.initial number_input.initial))
      then
        emit
          builder
          (Set_number_input
             ( id
             , Number_input.Expert.config_to_wire number_input.config
             , Number_input.Expert.value_to_wire number_input.initial ));
      (* Draft seeds never produce a replacement/update for a live editor. *)
      if Option.is_none old
      then
        Option.iter number_input.initial_draft ~f:(fun draft ->
          emit
            builder
            (Set_number_input_draft (id, Some (Number_input.Draft.to_string draft)))));
    Option.iter description.otp_input ~f:(fun input ->
      let policy = Otp_input.Config.policy input.config in
      if not (Otp_input.Value.fits input.initial ~policy)
      then fail "OTP initial value is incompatible with its policy";
      let old =
        Option.bind previous ~f:(fun mounted ->
          (View.Expert.describe mounted.view).otp_input)
      in
      Option.iter old ~f:(fun old ->
        if not (Otp_input.Policy.equal (Otp_input.Config.policy old.config) policy)
        then fail "OTP policy is immutable; remount with a new controller identity");
      if
        not
          (Option.exists old ~f:(fun old ->
             Otp_input.Config.equal old.config input.config))
      then
        emit
          builder
          (Set_otp_input
             ( id
             , Otp_input.Expert.config_to_wire input.config
             , Otp_input.Expert.value_to_wire input.initial )));
    Option.iter description.color_input ~f:(fun input ->
      let old =
        Option.bind previous ~f:(fun mounted ->
          (View.Expert.describe mounted.view).color_input)
      in
      if Option.is_none old && not (Color_input.Config.allows input.config input.initial)
      then fail "color seed is disabled by its alpha/empty policy";
      if
        not
          (Option.exists old ~f:(fun old ->
             Gpuio_protocol.Color_input_wire.Config.equal
               (Color_input.Expert.config_to_wire old.config)
               (Color_input.Expert.config_to_wire input.config)))
      then
        emit
          builder
          (Set_color_input
             ( id
             , Color_input.Expert.config_to_wire input.config
             , Color_input.Expert.value_to_wire input.initial )));
    Option.iter description.calendar ~f:(fun calendar ->
      let mode = Calendar.Config.mode calendar.config in
      if not (Calendar.Selection.fits calendar.initial ~mode)
      then fail "calendar seed does not fit mode";
      let old =
        Option.bind previous ~f:(fun mounted ->
          (View.Expert.describe mounted.view).calendar)
      in
      (match old with
       | None ->
         if
           not
             (Calendar.Constraints.allows_selection
                (Calendar.Config.constraints calendar.config)
                calendar.initial
                ~mode)
         then fail "calendar seed is disabled by its constraints"
       | Some old ->
         if not (Calendar.Mode.equal (Calendar.Config.mode old.config) mode)
         then fail "calendar mode is immutable; remount with a new controller identity");
      if
        not
          (Option.exists old ~f:(fun old ->
             Calendar.Config.equal old.config calendar.config))
      then
        emit
          builder
          (Set_calendar
             ( id
             , Calendar.Expert.config_to_wire calendar.config
             , Calendar.Expert.selection_to_wire calendar.initial
             , Calendar.Expert.month_to_wire calendar.initial_month ));
      if
        not
          (Option.equal
             Gpuio_protocol.Calendar_content_wire.equal
             calendar.content
             (Option.bind old ~f:(fun old -> old.content)))
      then emit builder (Set_calendar_content (id, calendar.content)));
    Option.iter description.rating ~f:(fun rating ->
      let old =
        Option.bind previous ~f:(fun mounted ->
          Option.map (View.Expert.describe mounted.view).rating ~f:(fun item ->
            item.config))
      in
      if not (Option.equal Rating.Config.equal old (Some rating.config))
      then emit builder (Set_rating (id, Rating.Expert.to_wire rating.config)));
    let button_presentation =
      Option.map description.button_presentation ~f:Button.Expert.Presentation.to_wire
    in
    let old_button_presentation =
      Option.bind previous ~f:(fun mounted ->
        Option.map
          (View.Expert.describe mounted.view).button_presentation
          ~f:Button.Expert.Presentation.to_wire)
    in
    if
      not
        (Option.equal
           Gpuio_protocol.Button_wire.Config.equal
           old_button_presentation
           button_presentation)
    then emit builder (Set_button_presentation (id, button_presentation));
    (match description.kind, description.command_ref with
     | Command_button, Some command ->
       let loading =
         Option.exists button_presentation ~f:(fun config -> config.policy.loading)
       in
       let old = Map.find builder.command_buttons (node_slot id) in
       let activation_revision =
         match old with
         | Some old
           when Node_id.equal old.node id
                && Ui_command.Id.equal old.command command
                && Bool.equal old.loading loading -> old.activation_revision
         | Some _ | None ->
           if Int64.equal builder.base_revision Int64.max_value
           then fail "native revision exhausted";
           Int64.succ builder.base_revision
       in
       builder.command_buttons
       <- Map.set
            builder.command_buttons
            ~key:(node_slot id)
            ~data:{ node = id; command; loading; activation_revision }
     | _ -> ());
    let tab_order = Option.map description.tab_order ~f:Tab_order.Expert.to_wire in
    let old_tab_order =
      Option.bind previous ~f:(fun mounted ->
        Option.map
          (View.Expert.describe mounted.view).tab_order
          ~f:Tab_order.Expert.to_wire)
    in
    if
      not
        (Option.equal
           Gpuio_protocol.Checkable_wire.Tab_order.equal
           old_tab_order
           tab_order)
    then emit builder (Set_tab_order (id, tab_order));
    let split_button =
      Option.map description.split_button ~f:(fun (appearance, parts) ->
        Split_button.Expert.to_wire appearance ~parts ~theme:builder.theme |> value)
    in
    let old_split_button =
      Option.bind previous ~f:(fun mounted -> mounted.split_button)
    in
    if not (Option.equal Wire.Split_button.equal old_split_button split_button)
    then emit builder (Set_split_button (id, split_button));
    let scrollbar =
      Option.map description.scrollbar ~f:(fun config ->
        Scrollbar.Expert.to_wire config ~theme:builder.theme |> value)
    in
    if
      not
        (Option.equal
           Wire.Scrollbar.equal
           scrollbar
           (Option.bind previous ~f:(fun mounted -> mounted.scrollbar)))
    then emit builder (Set_scrollbar (id, scrollbar));
    let control_appearance =
      Option.map description.control_appearance ~f:(fun appearance ->
        Control_appearance.Expert.to_wire appearance ~theme:builder.theme |> value)
    in
    let old_control_appearance =
      Option.bind previous ~f:(fun mounted -> mounted.control_appearance)
    in
    if
      not
        (Option.equal
           Wire.Control_appearance.equal
           old_control_appearance
           control_appearance)
    then emit builder (Set_control_appearance (id, control_appearance));
    let number_step_mode : Wire.Number_input.Step_mode.t =
      match
        Option.map description.number_input ~f:(fun input ->
          Number_input.Config.step_mode input.config)
      with
      | None | Some Native -> Native
      | Some Application -> Application
    in
    let old_mode =
      Option.value_map previous ~default:Wire.Number_input.Step_mode.Native ~f:(fun old ->
        old.number_step_mode)
    in
    if not (Wire.Number_input.Step_mode.equal number_step_mode old_mode)
    then emit builder (Set_number_step_mode (id, number_step_mode));
    let number_presentation =
      Option.bind description.number_input ~f:(fun input ->
        Option.map input.appearance ~f:(fun appearance ->
          Number_input.Expert.appearance_to_wire appearance ~theme:builder.theme |> value))
    in
    if
      not
        (Option.equal
           Wire.Number_presentation.equal
           number_presentation
           (Option.bind previous ~f:(fun old -> old.number_presentation)))
    then emit builder (Set_number_presentation (id, number_presentation));
    let previous_popover =
      Option.value_map previous ~default:false ~f:(fun old ->
        (View.Expert.describe old.view).popover)
    in
    if not (Bool.equal description.popover previous_popover)
    then emit builder (Set_popover (id, description.popover));
    let reveal = description.reveal in
    if
      not
        (Option.equal
           Gpuio_protocol.Reveal_wire.equal
           reveal
           (Option.bind previous ~f:(fun old -> old.reveal)))
    then emit builder (Set_reveal (id, reveal));
    let color_presentation =
      Option.bind description.color_input ~f:(fun input ->
        let appearance =
          Option.value input.appearance ~default:Color_input.Appearance.default
        in
        let wire =
          Color_input.Expert.presentation_to_wire
            input.config
            ~appearance
            ~theme:builder.theme
          |> value
        in
        Option.some_if
          (not
             (Gpuio_protocol.Color_presentation_wire.equal
                wire
                Gpuio_protocol.Color_presentation_wire.default))
          wire)
    in
    if
      not
        (Option.equal
           Gpuio_protocol.Color_presentation_wire.equal
           color_presentation
           (Option.bind previous ~f:(fun old -> old.color_presentation)))
    then emit builder (Set_color_presentation (id, color_presentation));
    let calendar_appearance =
      Option.bind description.calendar ~f:(fun calendar ->
        Option.bind calendar.appearance ~f:(fun appearance ->
          let wire =
            Calendar.Expert.appearance_to_wire appearance ~theme:builder.theme |> value
          in
          Option.some_if
            (not
               (Gpuio_protocol.Calendar_presentation_wire.equal
                  wire
                  Gpuio_protocol.Calendar_presentation_wire.default))
            wire))
    in
    if
      not
        (Option.equal
           Gpuio_protocol.Calendar_presentation_wire.equal
           calendar_appearance
           (Option.bind previous ~f:(fun old -> old.calendar_appearance)))
    then emit builder (Set_calendar_appearance (id, calendar_appearance));
    let slider_appearance =
      Option.bind description.slider ~f:(fun slider ->
        Option.bind slider.appearance ~f:(fun appearance ->
          let wire =
            Slider.Expert.appearance_to_wire appearance ~theme:builder.theme |> value
          in
          Option.some_if
            (not
               (Gpuio_protocol.Slider_presentation_wire.equal
                  wire
                  Gpuio_protocol.Slider_presentation_wire.default))
            wire))
    in
    if
      not
        (Option.equal
           Gpuio_protocol.Slider_presentation_wire.equal
           slider_appearance
           (Option.bind previous ~f:(fun old -> old.slider_appearance)))
    then emit builder (Set_slider_appearance (id, slider_appearance));
    let otp_appearance =
      Option.bind description.otp_input ~f:(fun input ->
        Option.bind input.appearance ~f:(fun appearance ->
          let wire =
            Otp_input.Expert.appearance_to_wire appearance ~theme:builder.theme |> value
          in
          Option.some_if
            (not
               (Gpuio_protocol.Otp_presentation_wire.equal
                  wire
                  Gpuio_protocol.Otp_presentation_wire.default))
            wire))
    in
    let old_otp_appearance =
      Option.bind previous ~f:(fun mounted -> mounted.otp_appearance)
    in
    if
      not
        (Option.equal
           Gpuio_protocol.Otp_presentation_wire.equal
           old_otp_appearance
           otp_appearance)
    then emit builder (Set_otp_appearance (id, otp_appearance));
    let rating_appearance =
      match Option.bind description.rating ~f:(fun rating -> rating.appearance) with
      | None -> None
      | Some appearance ->
        Rating.Expert.appearance_to_wire appearance ~theme:builder.theme |> value
    in
    let old_appearance =
      Option.bind previous ~f:(fun mounted -> mounted.rating_appearance)
    in
    if not (Option.equal Wire.Rating.Appearance.equal old_appearance rating_appearance)
    then emit builder (Set_rating_appearance (id, rating_appearance));
    Option.iter description.avatar ~f:(fun config ->
      let old =
        Option.bind previous ~f:(fun mounted ->
          (View.Expert.describe mounted.view).avatar)
      in
      if not (Option.equal Avatar.Config.equal old (Some config))
      then
        emit
          builder
          (Set_avatar (id, Avatar.Expert.to_wire config ~owner:builder.asset_owner)));
    Option.iter description.spinner ~f:(fun config ->
      let old =
        Option.bind previous ~f:(fun mounted ->
          (View.Expert.describe mounted.view).spinner)
      in
      if not (Option.equal Spinner.Config.equal old (Some config))
      then
        emit
          builder
          (Set_spinner (id, Spinner.Expert.to_wire config ~owner:builder.asset_owner)));
    if Option.is_none description.spinner
    then
      Option.iter description.loading ~f:(fun config ->
        let old =
          Option.bind previous ~f:(fun mounted ->
            (View.Expert.describe mounted.view).loading)
        in
        if
          (not (Option.equal Loading.Config.equal old (Some config)))
          || Option.exists previous ~f:(fun mounted ->
            Option.is_some (View.Expert.describe mounted.view).spinner)
        then emit builder (Set_loading (id, Loading.Expert.to_wire config)));
    Option.iter description.progress_presentation ~f:(fun config ->
      let old =
        Option.bind previous ~f:(fun mounted ->
          (View.Expert.describe mounted.view).progress_presentation)
      in
      if
        not
          (Option.equal Gpuio_protocol.Progress_wire.Presentation.equal old (Some config))
      then emit builder (Set_progress_presentation (id, config)));
    if Option.is_none description.progress_presentation
    then
      Option.iter description.progress ~f:(fun progress ->
        let old =
          Option.bind previous ~f:(fun mounted ->
            (View.Expert.describe mounted.view).progress)
        in
        if
          (not (Option.equal Progress.Config.equal old (Some progress)))
          || Option.exists previous ~f:(fun mounted ->
            Option.is_some (View.Expert.describe mounted.view).progress_presentation)
        then emit builder (Set_progress (id, Progress.Expert.to_wire progress)));
    Option.iter description.palette ~f:(fun palette ->
      let old =
        Option.bind previous ~f:(fun mounted ->
          Option.map (View.Expert.describe mounted.view).palette ~f:(fun palette ->
            palette.config))
      in
      let wire = Command_palette.Expert.to_wire palette.config in
      if
        not
          (Option.equal
             Wire.Palette.equal
             (Option.map old ~f:Command_palette.Expert.to_wire)
             (Some wire))
      then emit builder (Set_palette (id, wire));
      let options = Command_palette.Expert.options palette.config in
      if
        not
          (Option.equal
             Gpuio_protocol.Palette_options_wire.equal
             (Option.bind old ~f:Command_palette.Expert.options)
             options)
      then emit builder (Set_palette_options (id, options));
      let layout = Command_palette.Expert.layout palette.config in
      if
        not
          (Option.equal
             Gpuio_protocol.Palette_layout_wire.equal
             (Option.bind old ~f:Command_palette.Expert.layout)
             layout)
      then emit builder (Set_palette_layout (id, layout));
      let observed = Option.is_some palette.on_change in
      let was_observed =
        Option.exists previous ~f:(fun mounted ->
          Option.exists (View.Expert.describe mounted.view).palette ~f:(fun old ->
            Option.is_some old.on_change))
      in
      if not (Bool.equal observed was_observed)
      then emit builder (Set_palette_observed (id, observed)));
    let editor_config (description : _ View.Expert.description) =
      match description.editor, description.combobox with
      | Some editor, None -> Some editor.config
      | None, Some combo -> Some (Combobox.Expert.editor_config combo.config)
      | None, None -> None
      | Some _, Some _ -> fail "incompatible editor descriptions"
    in
    Option.iter (editor_config description) ~f:(fun config ->
      let old =
        Option.bind previous ~f:(fun mounted ->
          editor_config (View.Expert.describe mounted.view))
      in
      let next = Text_input.Expert.config_to_wire config in
      if
        not
          (Option.equal
             Wire.Editor.Config.equal
             (Option.map old ~f:Text_input.Expert.config_to_wire)
             (Some next))
      then emit builder (Set_editor (id, next));
      let previous_privacy =
        Option.value_map
          old
          ~default:Text_input.Privacy.Plain
          ~f:Text_input.Config.privacy
      in
      let privacy = Text_input.Config.privacy config in
      if not (Text_input.Privacy.equal previous_privacy privacy)
      then
        emit builder (Set_editor_privacy (id, Text_input.Expert.privacy_to_wire privacy));
      let old_hint = Option.bind old ~f:Text_input.Config.content_hint in
      let hint = Text_input.Config.content_hint config in
      if not (Option.equal Text_input.Content_hint.equal old_hint hint)
      then
        emit
          builder
          (Set_editor_content_hint
             (id, Option.map hint ~f:Text_input.Content_hint.Expert.to_wire));
      let old_format = Option.bind old ~f:Text_input.Config.format in
      let format = Text_input.Config.format config in
      if not (Option.equal Input_format.equal old_format format)
      then
        emit
          builder
          (Set_editor_format (id, Option.map format ~f:Input_format.Expert.to_wire));
      let old_filter = Option.bind old ~f:Text_input.Config.edit_filter in
      let filter = Text_input.Config.edit_filter config in
      if not (Option.equal Input_validation.equal old_filter filter)
      then
        emit
          builder
          (Set_editor_validation (id, Option.map filter ~f:Input_validation.Expert.to_wire));
      let old_clear =
        Option.value_map old ~default:false ~f:Text_input.Config.clear_on_escape
      in
      let clear = Text_input.Config.clear_on_escape config in
      if not (Bool.equal old_clear clear)
      then emit builder (Set_editor_clear_on_escape (id, clear));
      let old_searchable =
        Option.value_map old ~default:false ~f:Text_input.Config.searchable
      in
      let searchable = Text_input.Config.searchable config in
      if not (Bool.equal old_searchable searchable)
      then emit builder (Set_editor_searchable (id, searchable));
      let old_layout = Option.bind old ~f:Text_input.Config.layout in
      let layout = Text_input.Config.layout config in
      if not (Option.equal Text_area_layout.equal old_layout layout)
      then
        emit
          builder
          (Set_text_area_layout (id, Option.map layout ~f:Text_area_layout.Expert.to_wire)));
    let editor_frame (description : _ View.Expert.description) =
      Option.bind description.editor ~f:(fun editor ->
        Option.map editor.frame ~f:Input_frame.Expert.to_wire)
    in
    let frame = editor_frame description in
    let old_frame =
      Option.bind previous ~f:(fun mounted ->
        editor_frame (View.Expert.describe mounted.view))
    in
    if not (Option.equal Gpuio_protocol.Editor_frame_wire.equal old_frame frame)
    then emit builder (Set_editor_frame (id, frame));
    Option.iter description.combobox ~f:(fun combo ->
      let filter = Combobox.Config.filter combo.config in
      let old =
        Option.bind previous ~f:(fun mounted ->
          Option.map (View.Expert.describe mounted.view).combobox ~f:(fun combo ->
            Combobox.Config.filter combo.config))
      in
      if not (Option.equal Combobox.Filter.equal old (Some filter))
      then
        emit
          builder
          (Set_combobox_filter
             ( id
             , match filter with
               | Substring -> Substring
               | Unfiltered -> Unfiltered )));
    Option.iter description.focus_scope ~f:(fun config ->
      let old =
        Option.bind previous ~f:(fun mounted ->
          (View.Expert.describe mounted.view).focus_scope)
      in
      if not (Option.equal Focus_scope.equal old (Some config))
      then emit builder (Set_focus_scope (id, Focus_scope.Expert.to_wire config)));
    Option.iter description.tooltip ~f:(fun tooltip ->
      let old =
        Option.bind previous ~f:(fun mounted ->
          Option.map (View.Expert.describe mounted.view).tooltip ~f:(fun old ->
            Tooltip.Expert.to_wire old.config))
      in
      let next = Tooltip.Expert.to_wire tooltip.config in
      if not (Option.equal Wire.Tooltip.equal old (Some next))
      then emit builder (Set_tooltip (id, next)));
    let next_region = description.View.Expert.window_region in
    let old_region =
      Option.bind previous ~f:(fun mounted ->
        (View.Expert.describe mounted.view).window_region)
    in
    if not (Option.equal Window_region.equal next_region old_region)
    then
      emit
        builder
        (Set_window_region (id, Option.map next_region ~f:Window_region.Expert.to_wire));
    let tooltip_motion description =
      Option.exists description.View.Expert.tooltip ~f:(fun tooltip ->
        Tooltip.Motion.equal (Tooltip.Expert.motion tooltip.config) Enter_and_switch)
    in
    let tooltip_entering = tooltip_motion description in
    let old_tooltip_entering =
      Option.exists previous ~f:(fun mounted ->
        tooltip_motion (View.Expert.describe mounted.view))
    in
    if not (Bool.equal tooltip_entering old_tooltip_entering)
    then emit builder (Set_tooltip_motion (id, tooltip_entering));
    let motion description =
      Option.exists description.View.Expert.overlay ~f:(fun overlay ->
        Overlay.Motion.equal overlay.motion Enter)
    in
    let entering = motion description in
    let previous_entering =
      Option.exists previous ~f:(fun mounted ->
        motion (View.Expert.describe mounted.view))
    in
    if not (Bool.equal entering previous_entering)
    then emit builder (Set_overlay_motion (id, entering));
    let overlay_backdrop =
      Option.bind description.overlay ~f:(fun overlay ->
        Option.map overlay.backdrop ~f:(fun color ->
          Theme.resolve builder.theme color |> value))
    in
    if
      not
        (Option.equal
           Int64.equal
           overlay_backdrop
           (Option.bind previous ~f:(fun old -> old.overlay_backdrop)))
    then emit builder (Set_overlay_backdrop (id, overlay_backdrop));
    let overlay =
      Option.map description.overlay ~f:(fun overlay ->
        Overlay.Expert.to_wire overlay.config ~kind:overlay.kind)
    in
    let old_overlay =
      Option.bind previous ~f:(fun mounted ->
        Option.map (View.Expert.describe mounted.view).overlay ~f:(fun overlay ->
          Overlay.Expert.to_wire overlay.config ~kind:overlay.kind))
    in
    if not (Option.equal Wire.Overlay.equal old_overlay overlay)
    then emit builder (Set_overlay (id, overlay));
    let sheet_insets description =
      Option.bind description.View.Expert.overlay ~f:(fun overlay ->
        Option.map overlay.sheet_insets ~f:Sheet.Expert.insets_to_wire)
    in
    let next_insets = sheet_insets description in
    let old_insets =
      Option.bind previous ~f:(fun mounted ->
        sheet_insets (View.Expert.describe mounted.view))
    in
    if not (Option.equal Gpuio_protocol.Sheet_insets_wire.equal old_insets next_insets)
    then emit builder (Set_sheet_insets (id, next_insets));
    let placement description =
      match description.View.Expert.overlay, description.tooltip with
      | Some overlay, None -> Some (Overlay.Expert.placement overlay.config)
      | None, Some tooltip -> Some (Tooltip.Expert.placement tooltip.config)
      | None, None -> Option.bind description.menu ~f:(fun menu -> menu.placement)
      | Some _, Some _ -> fail "incompatible overlay descriptions"
    in
    let geometry description =
      match description.View.Expert.overlay with
      | Some
          { kind =
              Dialog | Alert_dialog | Sheet_left | Sheet_right | Sheet_top | Sheet_bottom
          ; _
          } -> None
      | Some { kind = Popover; _ } | None ->
        Option.bind (placement description) ~f:Placement.Expert.geometry
    in
    let next_geometry = geometry description in
    let old_geometry =
      Option.bind previous ~f:(fun mounted ->
        geometry (View.Expert.describe mounted.view))
    in
    if
      not
        (Option.equal
           Gpuio_protocol.Placement_geometry_wire.equal
           old_geometry
           next_geometry)
    then emit builder (Set_placement_geometry (id, next_geometry));
    let next_placement = Option.map (placement description) ~f:Placement.Expert.to_wire in
    let old_placement =
      Option.bind previous ~f:(fun mounted ->
        Option.map
          (placement (View.Expert.describe mounted.view))
          ~f:Placement.Expert.to_wire)
    in
    if not (Option.equal Wire.Placement.equal old_placement next_placement)
    then emit builder (Set_placement (id, next_placement));
    Option.iter description.control ~f:(fun control ->
      let old =
        Option.bind previous ~f:(fun mounted ->
          (View.Expert.describe mounted.view).control)
      in
      if not (Option.equal View.Expert.Control.equal old (Some control))
      then emit builder (Set_control (id, View.Expert.Control.to_wire control)));
    let choice_config (description : _ View.Expert.description) =
      match description.choice, description.combobox with
      | Some choice, None -> Some choice.config
      | None, Some combo -> Some (Combobox.Config.choices combo.config)
      | None, None -> None
      | Some _, Some _ -> fail "incompatible choice descriptions"
    in
    Option.iter (choice_config description) ~f:(fun config ->
      let old =
        Option.bind previous ~f:(fun mounted ->
          choice_config (View.Expert.describe mounted.view))
      in
      if not (Option.equal Choice.Config.equal old (Some config))
      then emit builder (Set_choice (id, Choice.Expert.config_to_wire config)));
    let choice_picker =
      Option.map description.choice_picker ~f:(fun (picker, _) ->
        Choice_picker.Expert.description_to_wire picker ~theme:builder.theme |> value)
    in
    if
      not
        (Option.equal
           Wire.Choice_picker_presentation.equal
           choice_picker
           (Option.bind previous ~f:(fun mounted -> mounted.choice_picker)))
    then
      Option.iter choice_picker ~f:(fun presentation ->
        emit builder (Set_choice_picker (id, presentation)));
    let choice_menu =
      Option.exists description.choice ~f:(fun choice -> choice.choice_menu)
    in
    if not (Bool.equal choice_menu (Option.exists previous ~f:(fun m -> m.choice_menu)))
    then emit builder (Set_choice_menu (id, choice_menu));
    let tab_trailing =
      Option.exists description.choice ~f:(fun choice -> choice.tab_trailing)
    in
    if not (Bool.equal tab_trailing (Option.exists previous ~f:(fun m -> m.tab_trailing)))
    then emit builder (Set_tab_trailing (id, tab_trailing));
    let tab_motion =
      Option.bind description.choice ~f:(fun choice -> choice.tab_motion)
      |> Option.map ~f:Tab_bar.Expert.motion_to_wire
    in
    if
      not
        (Option.equal
           Wire.Tab_motion.equal
           tab_motion
           (Option.bind previous ~f:(fun m -> m.tab_motion)))
    then emit builder (Set_tab_motion (id, tab_motion));
    let tab_viewport =
      Option.bind description.choice ~f:(fun choice -> choice.tab_viewport)
      |> Option.map ~f:Tab_bar.Expert.viewport_to_wire
    in
    if
      not
        (Option.equal
           Wire.Tab_viewport.equal
           tab_viewport
           (Option.bind previous ~f:(fun mounted -> mounted.tab_viewport)))
    then emit builder (Set_tab_viewport (id, tab_viewport));
    let tab_content =
      Option.bind description.choice ~f:(fun choice -> choice.tab_content)
    in
    if
      not
        (Option.equal
           Wire.Tab_content.equal
           tab_content
           (Option.bind previous ~f:(fun mounted -> mounted.tab_content)))
    then emit builder (Set_tab_content (id, tab_content));
    let tab_appearance =
      Option.bind description.choice ~f:(fun choice -> choice.tab_appearance)
      |> Option.map ~f:(fun appearance ->
        Tab_bar.Expert.to_wire appearance ~theme:builder.theme |> value)
    in
    if
      not
        (Option.equal
           Wire.Tab_appearance.equal
           tab_appearance
           (Option.bind previous ~f:(fun mounted -> mounted.tab_appearance)))
    then emit builder (Set_tab_appearance (id, tab_appearance));
    let choice_appearance =
      (match description.palette, description.menu, description.combobox with
       | Some palette, _, _ -> Some palette.appearance
       | None, Some menu, _ -> Some menu.appearance
       | None, None, Some combo -> Some combo.appearance
       | None, None, None ->
         Option.bind description.choice ~f:(fun choice -> choice.appearance))
      |> Option.map ~f:(fun appearance ->
        Choice.Expert.appearance_to_wire appearance ~theme:builder.theme |> value)
    in
    let old_appearance =
      Option.bind previous ~f:(fun mounted -> mounted.choice_appearance)
    in
    if not (Option.equal Wire.Choice_appearance.equal old_appearance choice_appearance)
    then
      Option.iter choice_appearance ~f:(fun appearance ->
        emit builder (Set_choice_appearance (id, appearance)));
    let style = Style.Expert.to_wire description.style ~theme:builder.theme |> value in
    let old_style =
      Option.value_map previous ~default:[] ~f:(fun mounted -> mounted.style)
    in
    if not (List.equal Wire.Style.equal old_style style)
    then emit builder (Set_style (id, style));
    let old_children =
      Option.value_map previous ~default:[] ~f:(fun mounted -> mounted.children)
    in
    let old_by_key =
      List.mapi old_children ~f:(fun position mounted ->
        Identity.of_view mounted.view position, mounted)
      |> Map.of_alist_exn (module Identity)
    in
    let keys =
      List.mapi description.children ~f:(fun position child ->
        Identity.of_view child position, child)
    in
    let new_by_key =
      match Map.of_alist (module Identity) keys with
      | `Ok map -> map
      | `Duplicate_key _ -> fail "duplicate sibling view key"
    in
    Map.iteri old_by_key ~f:(fun ~key ~data ->
      if not (Map.mem new_by_key key) then remove builder data);
    let children =
      List.map keys ~f:(fun (key, child) ->
        mount builder ~depth:(depth + 1) (Map.find old_by_key key) child)
    in
    let children = finalize_sibling_list_inputs builder children in
    splice builder id old_children children;
    Option.iter description.choice_picker ~f:(fun (picker, callback) ->
      let query_node =
        Option.bind choice_picker ~f:(fun presentation ->
          List.find_mapi presentation.slots ~f:(fun index slot ->
            match slot with
            | Gpuio_protocol.Choice_picker_wire.Slot.Query ->
              Some (List.hd_exn (List.nth_exn children index).children).id
            | Trigger | Empty | Footer | Group _ | Option _ -> None))
      in
      builder.bindings
      <- Map.set
           builder.bindings
           ~key:(node_slot id)
           ~data:
             { node = id
             ; handler = Option.value_exn handler
             ; callback =
                 Choice_picker
                   (Choice_picker.Description.config picker, query_node, callback)
             });
    Option.iter description.virtual_list ~f:(fun list ->
      let identity = Option.value_exn list_identity in
      let old_list =
        Option.bind previous ~f:(fun old -> (View.Expert.describe old.view).virtual_list)
      in
      let config (list : _ View.Expert.virtual_list) =
        Virtual_list.Expert.config_to_wire list.config ~managed:list.managed
      in
      if
        not
          (Option.equal
             Gpuio_protocol.List_wire.Config.equal
             (Option.map old_list ~f:config)
             (Some (config list)))
      then emit builder (Set_list_config (id, config list));
      let axis = Virtual_list.Config.axis list.config in
      let old_axis =
        Option.value_map old_list ~default:Virtual_list.Axis.Vertical ~f:(fun old ->
          Virtual_list.Config.axis old.config)
      in
      if not (Virtual_list.Axis.equal old_axis axis)
      then emit builder (Set_list_axis (id, Virtual_list.Expert.axis_to_wire axis));
      let old_identity = Option.bind previous ~f:(fun old -> old.list_identity) in
      if not (Option.exists old_identity ~f:(fun old -> phys_equal old identity))
      then emit builder (Set_list_order (id, List_identity.order identity));
      let rows identity children =
        List.filter children ~f:(fun child ->
          Option.is_none (View.Expert.describe child.view).table_header)
        |> List.map ~f:(fun child ->
          let key = Option.value_exn (View.Expert.describe child.view).key in
          { Gpuio_protocol.List_wire.Row.id =
              Option.value_exn (List_identity.id identity key)
          ; node = child.id
          })
      in
      let old_rows =
        Option.value_map old_identity ~default:[] ~f:(fun identity ->
          rows identity old_children)
      in
      let next_rows = rows identity children in
      if not (List.equal Gpuio_protocol.List_wire.Row.equal old_rows next_rows)
      then emit builder (Set_list_rows (id, next_rows));
      let old_invalidation =
        Option.value_map old_list ~default:0L ~f:(fun old -> old.invalidation_revision)
      in
      if list.managed && Int64.(list.invalidation_revision < old_invalidation)
      then fail "virtual list invalidation revision went backwards";
      if
        Int64.equal list.invalidation_revision old_invalidation
        && (not (List.is_empty list.invalidated))
        && Option.exists old_list ~f:(fun old ->
          not (List.equal Key.equal old.invalidated list.invalidated))
      then fail "virtual list invalidation batch changed without a new revision";
      if
        Int64.(list.invalidation_revision > old_invalidation)
        && not (List.is_empty list.invalidated)
      then
        emit
          builder
          (Invalidate_list_rows
             ( id
             , List.map list.invalidated ~f:(fun key ->
                 Option.value_exn (List_identity.id identity key)) ));
      let old_scroll = Option.bind old_list ~f:(fun old -> old.scroll) in
      if not (Option.equal Virtual_list.Scroll_request.equal old_scroll list.scroll)
      then
        Option.iter list.scroll ~f:(fun request ->
          let request =
            Virtual_list.Expert.scroll_to_wire
              request
              ~find_id:(List_identity.id identity)
            |> value
          in
          (match request.target with
           | Gpuio_protocol.List_wire.Scroll_target.Focus_tree_row _ ->
             if Option.is_none list.on_tree_input
             then fail "tree row focus requires native tree input"
           | Offset _ | Reveal _ | End -> ());
          emit builder (Scroll_list (id, request))));
    let table_serial =
      let serial =
        Option.value_map previous ~default:0L ~f:(fun old -> old.table_serial)
      in
      match table with
      | None -> serial
      | Some table ->
        let old_commands =
          Option.value_map previous ~default:[] ~f:(fun old ->
            Option.bind (View.Expert.describe old.view).virtual_list ~f:(fun list ->
              list.table)
            |> Option.value_map ~default:[] ~f:(fun table -> table.commands))
        in
        if List.equal (Table.Command.equal Key.equal) old_commands table.commands
        then serial
        else
          List.fold table.commands ~init:serial ~f:(fun serial command ->
            if Int64.compare (Table.Command.serial command) serial <= 0
            then fail "table command serial did not advance";
            if
              not
                (Int64.equal
                   (Table.Command.query_generation command)
                   table.query_generation)
            then fail "table command belongs to an obsolete query";
            let request =
              Table.Expert.command_to_wire
                table.config
                command
                ~find_id:(List_identity.id (Option.value_exn list_identity))
              |> value
            in
            emit builder (Table_command (id, request));
            request.serial)
    in
    let controllers =
      let own =
        let controller =
          match
            ( description.editor
            , description.combobox
            , description.slider
            , description.number_input
            , description.otp_input
            , description.calendar
            , description.color_input )
          with
          | Some editor, None, None, None, None, None, None -> Some editor.controller
          | None, Some combo, None, None, None, None, None -> Some combo.controller
          | None, None, Some slider, None, None, None, None -> Some slider.controller
          | None, None, None, Some input, None, None, None -> Some input.controller
          | None, None, None, None, Some input, None, None -> Some input.controller
          | None, None, None, None, None, Some calendar, None -> Some calendar.controller
          | None, None, None, None, None, None, None -> None
          | None, None, None, None, None, None, Some input -> Some input.controller
          | _ -> fail "incompatible controller descriptions"
        in
        Option.value_map controller ~default:String.Set.empty ~f:(fun key ->
          String.Set.singleton (Key.to_string key))
      in
      List.fold children ~init:own ~f:(fun keys child ->
        if not (Set.is_empty (Set.inter keys child.controllers))
        then fail "native controller appears more than once in a window";
        Set.union keys child.controllers)
    in
    let menu_commands =
      Option.value_map description.menu ~default:String.Set.empty ~f:(fun menu ->
        List.concat_map menu.menus ~f:Menu.Expert.command_ids
        |> List.map ~f:Ui_command.Id.to_string
        |> String.Set.of_list)
    in
    let menu_commands =
      Option.value_map description.palette ~default:menu_commands ~f:(fun palette ->
        List.fold
          (Command_palette.Config.commands palette.config)
          ~init:menu_commands
          ~f:(fun refs id -> Set.add refs (Ui_command.Id.to_string id)))
    in
    let free_commands =
      List.fold
        children
        ~init:
          (Option.value_map description.command_ref ~default:menu_commands ~f:(fun id ->
             Set.add menu_commands (Ui_command.Id.to_string id)))
        ~f:(fun refs child -> Set.union refs child.free_commands)
    in
    let free_commands =
      List.fold commands ~init:free_commands ~f:(fun refs command ->
        Set.remove refs command.id)
    in
    let platform_menus =
      (if
         Option.exists menu ~f:(fun menu ->
           match menu.presentation with
           | Platform_bar -> true
           | Button | Context | Bar | Editor_context -> false)
       then 1
       else 0)
      + List.sum (module Int) children ~f:(fun child -> child.platform_menus)
    in
    if platform_menus > 1 then fail "only one platform menu bar may be mounted per window";
    { view
    ; id
    ; handler
    ; hover_handler
    ; calendar_viewport_handler
    ; style
    ; text_content
    ; animation
    ; animation_seen
    ; animation_program
    ; program_seen
    ; container_query
    ; query_seen
    ; binding_seen
    ; palette_seen
    ; document_diff_epoch
    ; document_profile_epoch
    ; document_actions_epoch
    ; document_preview_epoch
    ; document_text_style
    ; slider_seen
    ; number_input_seen
    ; otp_input_seen
    ; color_input_seen
    ; calendar_seen
    ; list_identity
    ; list_input_state = Option.bind previous ~f:(fun old -> old.list_input_state)
    ; list_input_generation =
        Option.value_map previous ~default:0L ~f:(fun old -> old.list_input_generation)
    ; table_config
    ; table_behavior
    ; table_appearance
    ; table_header_style
    ; table_row_style
    ; table_serial
    ; choice_picker
    ; split_group
    ; tab_appearance
    ; tab_content
    ; tab_viewport
    ; tab_motion
    ; tab_trailing
    ; choice_menu
    ; choice_appearance
    ; split_button
    ; control_appearance
    ; scrollbar
    ; number_step_mode
    ; number_presentation
    ; reveal
    ; overlay_backdrop
    ; calendar_appearance
    ; color_presentation
    ; slider_appearance
    ; otp_appearance
    ; rating_appearance
    ; children
    ; controllers
    ; commands
    ; free_commands
    ; menu
    ; platform_menus
    }
;;

let prepare t ~theme view =
  if t.closed
  then Or_error.error_string "window reconciler is closed"
  else if t.state.epoch = Int.max_value
  then Or_error.error_string "local commit epoch exhausted"
  else (
    try
      let builder =
        { window = t.window
        ; document_defaults = t.document_defaults
        ; nodes = t.state.nodes
        ; handlers = t.state.handlers
        ; bindings = t.state.bindings
        ; hover_bindings = t.state.hover_bindings
        ; calendar_viewport_bindings = t.state.calendar_viewport_bindings
        ; command_buttons = t.state.command_buttons
        ; base_revision = t.state.revision
        ; operations = []
        ; operation_count = 0
        ; command_generation = t.state.command_generation
        ; theme
        ; theme_unchanged = Theme.equal theme t.state.theme
        ; asset_owner = t.asset_owner
        ; document_owner = t.document_owner
        ; canvas_owner = t.canvas_owner
        ; chart_owner = t.chart_owner
        }
      in
      let root =
        match view with
        | None ->
          Option.iter t.state.root ~f:(remove builder);
          None
        | Some view ->
          Some
            (mount builder ~depth:0 t.state.root view
             |> finalize_list_input builder ~query:(fun _ -> None))
      in
      Option.iter root ~f:(fun root ->
        if not (Set.is_empty root.free_commands)
        then
          fail
            ("command references have no registry definition: "
             ^ String.concat ~sep:", " (Set.to_list root.free_commands)));
      let root_id = Option.map root ~f:(fun node -> node.id) in
      if
        not
          (Option.equal
             Node_id.equal
             root_id
             (Option.map t.state.root ~f:(fun node -> node.id)))
      then emit builder (Set_root root_id);
      let operations = List.rev builder.operations in
      let revision, message =
        if List.is_empty operations
        then t.state.revision, None
        else (
          if Int64.equal t.state.revision Int64.max_value
          then fail "native revision exhausted";
          let revision = Int64.succ t.state.revision in
          let message =
            Wire.Message.Apply
              { window = t.window; base = t.state.revision; revision; operations }
          in
          if Wire.Message.bin_size_t message > Wire.max_message_bytes
          then fail "atomic update exceeds native message byte limit";
          revision, Some message)
      in
      Ok
        { owner = t.owner
        ; base_epoch = t.state.epoch
        ; candidate =
            { root
            ; bindings = builder.bindings
            ; hover_bindings = builder.hover_bindings
            ; calendar_viewport_bindings = builder.calendar_viewport_bindings
            ; command_buttons = builder.command_buttons
            ; nodes = builder.nodes
            ; handlers = builder.handlers
            ; theme
            ; revision
            ; epoch = t.state.epoch + 1
            ; command_generation = builder.command_generation
            }
        ; message
        }
    with
    | Cannot_prepare error -> Error error)
;;

let message update = update.message

let accept t update =
  if t.closed
  then Or_error.error_string "window reconciler is closed"
  else if (not (phys_equal t.owner update.owner)) || t.state.epoch <> update.base_epoch
  then Or_error.error_string "stale or foreign prepared update"
  else (
    t.state <- update.candidate;
    Ok ())
;;

let retain_list_rows t notices =
  if t.closed
  then Or_error.error_string "window reconciler is closed"
  else
    let open Or_error.Let_syntax in
    let%bind () = Gpuio_protocol.List_wire.Retained.validate_all notices in
    List.map notices ~f:(fun notice ->
      match
        Map.find
          t.state.bindings
          (node_slot notice.Gpuio_protocol.List_wire.Retained.node)
      with
      | Some { node; callback = Virtual_list (identity, list, _, _); _ }
        when Node_id.equal node notice.node ->
        let%bind callback =
          Option.value_map
            list.on_retain
            ~default:(Or_error.error_string "list has no retention callback")
            ~f:Or_error.return
        in
        let%map keys =
          List.map notice.rows ~f:(fun id ->
            Option.value_map
              (List_identity.key identity id)
              ~default:
                (Or_error.error_string "retention references an absent logical row")
              ~f:Or_error.return)
          |> Or_error.all
        in
        callback keys
      | Some _ | None -> Or_error.error_string "retention references an absent list")
    |> Or_error.all
;;

let dispatch t = function
  | Wire.Event.Extension_event (window, node, handler, revision, generation, signal)
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.(revision >= 0L && revision <= t.state.revision) ->
    (match Map.find t.state.bindings (node_slot node) with
     | Some
         { node = expected
         ; handler = expected_handler
         ; callback = Extension (config, callback)
         }
       when Node_id.equal node expected
            && Handler_id.equal handler expected_handler
            && Int64.equal generation config.generation
            && Wire.Extension.Signal.valid signal ->
       (match signal with
        | Data _ when config.disabled -> None
        | Data _ | Mounted | Command_completed _ | Failed _ -> Some (callback signal))
     | Some _ | None -> None)
  | Wire.Event.Split_resized (window, node, handler, revision, generation, snapshot)
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.(revision >= 0L && revision <= t.state.revision) ->
    (match Map.find t.state.bindings (node_slot node) with
     | Some
         { node = expected
         ; handler = expected_handler
         ; callback = Split_pane (config, callback)
         }
       when Node_id.equal node expected
            && Handler_id.equal handler expected_handler
            && Int64.equal generation (Split_pane.Expert.generation config) ->
       Split_pane.Expert.snapshot_of_wire snapshot |> Result.ok |> Option.map ~f:callback
     | Some _ | None -> None)
  | Wire.Event.Split_group_resized (window, node, handler, revision, generation, snapshot)
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.(revision >= 0L && revision <= t.state.revision) ->
    (match Map.find t.state.bindings (node_slot node) with
     | Some
         { node = expected
         ; handler = expected_handler
         ; callback = Split_group (config, callback)
         }
       when Node_id.equal node expected
            && Handler_id.equal handler expected_handler
            && Int64.equal generation (Split_group.Expert.generation config) ->
       Split_group.Expert.snapshot_of_wire config snapshot
       |> Result.ok
       |> Option.map ~f:callback
     | Some _ | None -> None)
  | Wire.Event.Canvas_event
      ( window
      , node
      , handler
      , revision
      , source
      , scene_revision
      , scene_generation
      , observation )
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.(revision >= 0L && revision <= t.state.revision) ->
    (match Map.find t.state.bindings (node_slot node) with
     | Some
         { node = expected
         ; handler = expected_handler
         ; callback = Canvas (config, callback)
         }
       when Node_id.equal node expected
            && Handler_id.equal handler expected_handler
            && Option.equal Resource_id.equal source config.source
            && (Option.is_some source
                || Int64.(scene_revision = 0L && scene_generation = 0L)) ->
       Canvas.Expert.event ~scene_revision ~scene_generation observation
       |> Result.ok
       |> Option.map ~f:callback
     | Some _ | None -> None)
  | Wire.Event.Chart_event
      ( window
      , node
      , handler
      , revision
      , source
      , data_revision
      , data_generation
      , observation )
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.(revision >= 0L && revision <= t.state.revision) ->
    (match Map.find t.state.bindings (node_slot node) with
     | Some
         { node = expected
         ; handler = expected_handler
         ; callback = Chart (config, callback)
         }
       when Node_id.equal node expected
            && Handler_id.equal handler expected_handler
            && Option.equal Resource_id.equal source config.source
            && (Option.is_some source
                || Int64.(data_revision = 0L && data_generation = 0L)) ->
       Chart.Expert.event ~data_revision ~data_generation observation
       |> Result.ok
       |> Option.map ~f:callback
     | Some _ | None -> None)
  | Wire.Event.Document_profile_event (window, node, handler, revision, source, event)
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.(revision >= 0L && revision <= t.state.revision)
         && Gpuio_protocol.Document_profile_wire.Event.valid event ->
    (match Map.find t.state.bindings (node_slot node) with
     | Some
         { node = expected
         ; handler = expected_handler
         ; callback =
             Document
               { source = expected_source
               ; profile = Some (epoch, instance, callback)
               ; diff_config = _
               ; diff_epoch = _
               ; on_navigate = _
               ; on_diff = _
               ; preview_epoch = _
               ; preview_max_lines = _
               ; on_preview = _
               ; on_action = _
               ; actions_epoch = _
               ; actions_config = _
               }
         }
       when Node_id.equal node expected
            && Handler_id.equal handler expected_handler
            && Gpuio_protocol.Resource_id.equal
                 source
                 (Text_source.Expert.native_id expected_source)
            && Int64.equal epoch event.config_epoch
            && Int64.equal instance.generation event.instance_generation -> callback event
     | Some _ | None -> None)
  | Wire.Event.Document_action (window, node, handler, revision, source, event)
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.(revision >= 0L && revision <= t.state.revision) ->
    (match Map.find t.state.bindings (node_slot node) with
     | Some
         { node = expected
         ; handler = expected_handler
         ; callback =
             Document
               { source = expected_source
               ; actions_config
               ; actions_epoch
               ; profile = _
               ; on_action = Some callback
               ; on_navigate = _
               ; on_diff = _
               ; diff_config = _
               ; diff_epoch = _
               ; on_preview = _
               ; preview_epoch = _
               ; preview_max_lines = _
               }
         }
       when Node_id.equal node expected
            && Handler_id.equal handler expected_handler
            && Gpuio_protocol.Resource_id.equal
                 source
                 (Text_source.Expert.native_id expected_source)
            && Int64.equal actions_epoch event.config_epoch ->
       Document.Actions.Expert.event_of_wire ~config:actions_config event
       |> Result.ok
       |> Option.map ~f:callback
     | Some _ | None -> None)
  | Wire.Event.Document_preview_observed (window, node, handler, revision, source, event)
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.(revision >= 0L && revision <= t.state.revision) ->
    (match Map.find t.state.bindings (node_slot node) with
     | Some
         { node = expected
         ; handler = expected_handler
         ; callback =
             Document
               { source = expected_source
               ; preview_epoch
               ; preview_max_lines
               ; on_preview = Some callback
               ; diff_config = _
               ; diff_epoch = _
               ; on_diff = _
               ; on_navigate = _
               ; profile = _
               ; on_action = _
               ; actions_epoch = _
               ; actions_config = _
               }
         }
       when Node_id.equal node expected
            && Handler_id.equal handler expected_handler
            && Gpuio_protocol.Resource_id.equal
                 source
                 (Text_source.Expert.native_id expected_source)
            && Int64.equal preview_epoch event.config_epoch
            && (Option.is_some preview_max_lines
                || not
                     (Gpuio_protocol.Document_preview_wire.State.equal
                        event.state
                        (Rich true))) ->
       Document.Preview.Expert.event_of_wire event |> Result.ok |> Option.map ~f:callback
     | Some _ | None -> None)
  | Wire.Event.Document_diff_event (window, node, handler, revision, source, event)
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.(revision >= 0L && revision <= t.state.revision) ->
    (match Map.find t.state.bindings (node_slot node) with
     | Some
         { node = expected
         ; handler = expected_handler
         ; callback =
             Document
               { source = expected_source
               ; diff_config = Some config
               ; diff_epoch
               ; on_diff = Some callback
               ; on_navigate = _
               ; profile = _
               ; on_action = _
               ; actions_epoch = _
               ; actions_config = _
               ; preview_epoch = _
               ; preview_max_lines = _
               ; on_preview = _
               }
         }
       when Node_id.equal node expected
            && Handler_id.equal handler expected_handler
            && Gpuio_protocol.Resource_id.equal
                 source
                 (Text_source.Expert.native_id expected_source)
            && Int64.equal event.config_epoch diff_epoch ->
       Document.Diff.Expert.event_of_wire ~config event
       |> Result.ok
       |> Option.map ~f:callback
     | Some _ | None -> None)
  | Wire.Event.Document_navigation (window, node, handler, revision, source, _, navigation)
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.(revision >= 0L && revision <= t.state.revision) ->
    (match Map.find t.state.bindings (node_slot node) with
     | Some
         { node = expected
         ; handler = expected_handler
         ; callback =
             Document
               { source = expected_source
               ; on_navigate = Some callback
               ; profile = _
               ; on_action = _
               ; actions_epoch = _
               ; actions_config = _
               ; diff_config = _
               ; diff_epoch = _
               ; on_diff = _
               ; preview_epoch = _
               ; preview_max_lines = _
               ; on_preview = _
               }
         }
       when Node_id.equal node expected
            && Handler_id.equal handler expected_handler
            && Gpuio_protocol.Resource_id.equal
                 source
                 (Text_source.Expert.native_id expected_source) ->
       Document.Expert.navigation navigation |> Result.ok |> Option.map ~f:callback
     | Some _ | None -> None)
  | Wire.Event.Table_columns_observed (window, node, handler, revision, viewport)
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.equal revision t.state.revision ->
    (match Map.find t.state.bindings (node_slot node) with
     | Some
         { node = expected
         ; handler = expected_handler
         ; callback = Virtual_list (_, list, Some config, _)
         }
       when Node_id.equal node expected
            && Handler_id.equal handler expected_handler
            && Int64.equal viewport.schema_revision config.schema_revision
            && Int64.equal viewport.query_generation config.query_generation ->
       Option.bind list.table ~f:(fun table ->
         Option.bind table.on_column_viewport ~f:(fun callback ->
           Table.Expert.column_viewport_of_wire table.config viewport
           |> Option.map ~f:callback))
     | Some _ | None -> None)
  | Wire.Event.Table_input (window, node, handler, revision, input)
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.(revision >= 0L && revision <= t.state.revision) ->
    (match Map.find t.state.bindings (node_slot node) with
     | Some
         { node = expected
         ; handler = expected_handler
         ; callback = Virtual_list (identity, list, Some config, _)
         }
       when Node_id.equal node expected
            && Handler_id.equal handler expected_handler
            && Int64.equal input.schema_revision config.schema_revision
            && Int64.equal input.query_generation config.query_generation ->
       Option.bind list.table ~f:(fun table ->
         Table.Expert.request_of_wire
           table.config
           input.request
           ~find_key:(List_identity.key identity)
         |> Option.map ~f:table.on_input)
     | Some _ | None -> None)
  | Wire.Event.List_input (window, node, handler, revision, generation, request)
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.(revision >= 0L && revision <= t.state.revision) ->
    (match Map.find t.state.bindings (node_slot node) with
     | Some
         { node = expected
         ; handler = expected_handler
         ; callback = Virtual_list (identity, list, _, Some config)
         }
       when Node_id.equal node expected
            && Handler_id.equal handler expected_handler
            && Int64.equal generation config.generation
            && not config.disabled ->
       List_input.Expert.of_wire request ~find_key:(List_identity.key identity)
       |> Option.bind ~f:(fun request ->
         Option.map list.list_input ~f:(fun (_, callback) -> callback request))
     | Some _ | None -> None)
  | Wire.Event.Tree_input (window, node, handler, revision, request)
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.(revision >= 0L && revision <= t.state.revision) ->
    (match Map.find t.state.bindings (node_slot node) with
     | Some
         { node = expected
         ; handler = expected_handler
         ; callback = Virtual_list (identity, list, _, _)
         }
       when Node_id.equal node expected && Handler_id.equal handler expected_handler ->
       Tree_input.Expert.of_wire request ~find_key:(List_identity.key identity)
       |> Option.bind ~f:(fun input ->
         let allowed =
           match input with
           | Tree_input.Move _ -> list.tree_moves
           | Navigate _
           | Select _
           | Focus _
           | Set_expanded _
           | Activate _
           | Select_active _
           | Activate_active
           | Typeahead _
           | Set_selected _ -> true
         in
         if allowed
         then Option.map list.on_tree_input ~f:(fun callback -> callback input)
         else None)
     | Some _ | None -> None)
  | Wire.Event.List_viewport (window, node, handler, revision, viewport)
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.equal revision t.state.revision ->
    (match Map.find t.state.bindings (node_slot node) with
     | Some
         { node = expected
         ; handler = expected_handler
         ; callback = Virtual_list (identity, list, _, _)
         }
       when Node_id.equal node expected
            && Handler_id.equal handler expected_handler
            && Int64.equal viewport.order_revision (List_identity.order identity).revision
       ->
       Virtual_list.Expert.viewport_of_wire
         viewport
         ~find_key:(List_identity.key identity)
       |> Result.ok
       |> Option.bind ~f:(fun viewport ->
         Option.map list.on_viewport ~f:(fun callback -> callback viewport))
     | Some _ | None -> None)
  | Wire.Event.Container_selected (window, node, handler, revision, snapshot)
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.(revision >= 0L && revision <= t.state.revision) ->
    (match Map.find t.state.bindings (node_slot node) with
     | Some
         { node = expected
         ; handler = expected_handler
         ; callback = Container_query (config, seen, callback)
         }
       when Node_id.equal node expected
            && Handler_id.equal handler expected_handler
            && Int64.(snapshot.sequence > !seen) ->
       (match Container_query.Expert.selection_of_wire config snapshot with
        | Error _ -> None
        | Ok selection ->
          seen := snapshot.sequence;
          Some (callback selection))
     | Some _ | None -> None)
  | Wire.Event.Animation_program_event (window, node, handler, revision, signals)
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.(revision >= 0L && revision <= t.state.revision)
         && Wire.Animation_program.Signal.valid_batch signals ->
    (match Map.find t.state.bindings (node_slot node) with
     | Some
         { node = expected
         ; handler = expected_handler
         ; callback = Animation_program (generation, seen, callback)
         }
       when Node_id.equal node expected && Handler_id.equal handler expected_handler ->
       let first = List.hd_exn signals in
       let seen_run, seen_index = !seen in
       if Int64.(first.generation > generation || first.generation < seen_run)
       then None
       else (
         let fresh =
           List.filter signals ~f:(fun signal ->
             Int64.(signal.generation > seen_run || signal.index > seen_index))
         in
         match Animation.Expert.program_event_of_wire fresh with
         | Error _ -> None
         | Ok event ->
           seen := first.generation, (List.last_exn fresh).index;
           Some (callback event))
     | Some _ | None -> None)
  | Wire.Event.Animation_endpoint (window, node, handler, revision, endpoint)
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.(revision >= 0L && revision <= t.state.revision) ->
    (match
       Map.find t.state.bindings (node_slot node), Animation.Expert.event_of_wire endpoint
     with
     | ( Some
           { node = expected
           ; handler = expected_handler
           ; callback = Animation (generation, seen, callback)
           }
       , Ok event )
       when Node_id.equal node expected
            && Handler_id.equal handler expected_handler
            && Int64.(endpoint.generation > !seen && endpoint.generation <= generation) ->
       seen := endpoint.generation;
       Some (callback event)
     | _ -> None)
  | Wire.Event.Image_state (window, node, handler, revision, state)
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.(revision >= 0L && revision <= t.state.revision) ->
    (match
       Map.find t.state.bindings (node_slot node), Image.Expert.state_of_wire state
     with
     | ( Some { node = expected; handler = expected_handler; callback = Image callback }
       , Ok state )
       when Node_id.equal node expected && Handler_id.equal handler expected_handler ->
       Some (callback state)
     | _ -> None)
  | Wire.Event.Press (window, node, handler, revision)
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.(revision >= 0L && revision <= t.state.revision) ->
    (match Map.find t.state.bindings (node_slot node) with
     | Some binding
       when Node_id.equal node binding.node && Handler_id.equal handler binding.handler ->
       (match binding.callback with
        | Click callback -> Some (callback ())
        | Editor _
        | Rating _
        | Carousel _
        | Carousel_track _
        | Slider _
        | Number_input _
        | Otp_input _
        | Color_input _
        | Calendar _
        | Choice _
        | Combobox _
        | Choice_picker _
        | Picker_query
        | Dismiss _
        | Menu _
        | Tooltip _
        | Commands _
        | Palette _
        | Toast _
        | Input_region _
        | Command_binding_scope _
        | Highlight_scope _
        | Pointer _
        | Drag_source _
        | Drop_target _
        | Animation _
        | Animation_program _
        | Container_query _
        | Image _
        | Virtual_list _
        | Extension _
        | Split_pane _
        | Split_group _
        | Canvas _
        | Chart _
        | Document _ -> None)
     | Some _ | None -> None)
  | Slider_event (window, node, handler, revision, event)
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.(revision >= 0L && revision <= t.state.revision) ->
    (match Map.find t.state.bindings (node_slot node) with
     | Some
         { node = expected
         ; handler = expected_handler
         ; callback = Slider (initial, seen, callback)
         }
       when Node_id.equal node expected && Handler_id.equal handler expected_handler ->
       let snapshot = Wire.Slider.Event.snapshot event in
       if
         Int64.(snapshot.revision <= !seen)
         || not
              (Wire.Slider.Value.same_mode
                 (Slider.Expert.value_to_wire initial)
                 snapshot.value)
       then None
       else (
         match Slider.Expert.event_of_wire ~window ~node event with
         | Error _ -> None
         | Ok event ->
           seen := snapshot.revision;
           Some (callback event))
     | Some _ | None -> None)
  | Number_input_event (window, node, handler, revision, event)
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.(revision >= 0L && revision <= t.state.revision) ->
    (match Map.find t.state.bindings (node_slot node) with
     | Some
         { node = expected
         ; handler = expected_handler
         ; callback = Number_input (seen, callback)
         }
       when Node_id.equal node expected && Handler_id.equal handler expected_handler ->
       let snapshot = Wire.Number_input.Event.snapshot event in
       let seen_revision, seen_request = !seen in
       let request_id =
         match event with
         | Wire.Number_input.Event.Step_requested request -> Some request.id
         | Observed _ | Changed _ | Committed _ | Rejected _ | Cancelled _ -> None
       in
       let stale =
         match request_id with
         | None -> Int64.(snapshot.revision <= seen_revision)
         | Some id -> Int64.(snapshot.revision < seen_revision || id <= seen_request)
       in
       if stale
       then None
       else (
         match Number_input.Expert.event_of_wire ~window ~node event with
         | Error _ -> None
         | Ok event ->
           seen := snapshot.revision, Option.value request_id ~default:seen_request;
           Some (callback event))
     | Some _ | None -> None)
  | Otp_input_event (window, node, handler, revision, event)
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.(revision >= 0L && revision <= t.state.revision) ->
    (match Map.find t.state.bindings (node_slot node) with
     | Some
         { node = expected
         ; handler = expected_handler
         ; callback = Otp_input (policy, seen, callback)
         }
       when Node_id.equal node expected && Handler_id.equal handler expected_handler ->
       let snapshot = Wire.Otp_input.Event.snapshot event in
       if
         Int64.(snapshot.revision <= !seen)
         || not
              (Wire.Otp_input.Policy.equal
                 snapshot.policy
                 (Otp_input.Expert.policy_to_wire policy))
       then None
       else (
         match Otp_input.Expert.event_of_wire ~window ~node event with
         | Error _ -> None
         | Ok event ->
           seen := snapshot.revision;
           Some (callback event))
     | Some _ | None -> None)
  | Color_input_event (window, node, handler, revision, event)
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.(revision >= 0L && revision <= t.state.revision) ->
    (match Map.find t.state.bindings (node_slot node) with
     | Some
         { node = expected
         ; handler = expected_handler
         ; callback = Color_input (seen, callback)
         }
       when Node_id.equal node expected && Handler_id.equal handler expected_handler ->
       let snapshot = Wire.Color_input.Event.snapshot event in
       if Int64.(snapshot.revision <= !seen)
       then None
       else (
         match Color_input.Expert.event_of_wire ~window ~node event with
         | Error _ -> None
         | Ok event ->
           seen := snapshot.revision;
           Some (callback event))
     | Some _ | None -> None)
  | Calendar_event (window, node, handler, revision, event)
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.(revision >= 0L && revision <= t.state.revision) ->
    (match Map.find t.state.bindings (node_slot node) with
     | Some
         { node = expected
         ; handler = expected_handler
         ; callback = Calendar (mode, seen, callback)
         }
       when Node_id.equal node expected && Handler_id.equal handler expected_handler ->
       let snapshot = Wire.Calendar.Event.snapshot event in
       let event_mode =
         match snapshot.mode with
         | Single -> Calendar.Mode.Single
         | Range -> Calendar.Mode.Range
       in
       if Int64.(snapshot.revision <= !seen) || not (Calendar.Mode.equal event_mode mode)
       then None
       else (
         match Calendar.Expert.event_of_wire ~window ~node event with
         | Error _ -> None
         | Ok event ->
           seen := snapshot.revision;
           Some (callback event))
     | Some _ | None -> None)
  | Carousel_track_requested (window, node, handler, revision, request)
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.(revision >= 0L && revision <= t.state.revision) ->
    (match Map.find t.state.bindings (node_slot node) with
     | Some
         { node = expected
         ; handler = expected_handler
         ; callback = Carousel_track (config, callback)
         }
       when Node_id.equal node expected
            && Handler_id.equal handler expected_handler
            && Wire.Carousel_track.accepts_request config request ->
       Carousel_track.Expert.request_of_wire ~window ~node ~handler request
       |> Result.ok
       |> Option.map ~f:callback
     | Some _ | None -> None)
  | Carousel_requested (window, node, handler, revision, request)
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.(revision >= 0L && revision <= t.state.revision) ->
    (match
       Map.find t.state.bindings (node_slot node), Carousel.Expert.request_of_wire request
     with
     | ( Some
           { node = expected
           ; handler = expected_handler
           ; callback = Carousel (config, callback)
           }
       , Ok decoded )
       when Node_id.equal node expected
            && Handler_id.equal handler expected_handler
            && Wire.Carousel.accepts_request config request -> Some (callback decoded)
     | _ -> None)
  | Rating_requested (window, node, handler, revision, request)
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.(revision >= 0L && revision <= t.state.revision) ->
    (match
       Map.find t.state.bindings (node_slot node), Rating.Expert.request_of_wire request
     with
     | ( Some
           { node = expected
           ; handler = expected_handler
           ; callback = Rating (config, callback)
           }
       , Some request )
       when Node_id.equal node expected
            && Handler_id.equal handler expected_handler
            && Rating.Expert.can_apply config request -> Some (callback request)
     | _ -> None)
  | Choice (window, node, handler, revision, selected)
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.(revision >= 0L && revision <= t.state.revision) ->
    (match Map.find t.state.bindings (node_slot node), Choice.Id.of_string selected with
     | ( Some
           { node = expected
           ; handler = expected_handler
           ; callback = Choice (config, callback)
           }
       , Ok selected )
       when Node_id.equal node expected
            && Handler_id.equal handler expected_handler
            && Choice.Config.can_select config selected -> Some (callback selected)
     | _ -> None)
  | Editor_event (window, node, handler, revision, event, snapshot)
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.(revision >= 0L && revision <= t.state.revision) ->
    (match Map.find t.state.bindings (node_slot node) with
     | Some
         { node = expected
         ; handler = expected_handler
         ; callback = Combobox (_, callback)
         }
       when Node_id.equal node expected && Handler_id.equal handler expected_handler ->
       (match event with
        | Submitted -> None
        | Changed ->
          Text_input.Expert.snapshot_of_wire ~window ~node snapshot
          |> Result.ok
          |> Option.map ~f:(fun snapshot -> callback (Changed snapshot)))
     | Some { callback = Picker_query; _ } -> None
     | Some
         { node = expected; handler = expected_handler; callback = Editor (_, callback) }
       when Node_id.equal node expected && Handler_id.equal handler expected_handler ->
       let snapshot = Text_input.Expert.snapshot_of_wire ~window ~node snapshot in
       (match snapshot with
        | Error _ -> None
        | Ok snapshot ->
          (match event with
           | Changed -> Some (callback (Changed snapshot))
           | Submitted ->
             Text_input.Expert.submission snapshot
             |> Result.ok
             |> Option.map ~f:(fun submission -> callback (Submitted submission))))
     | Some _ | None -> None)
  | Editor_search_observed (window, node, handler, revision, search)
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.(revision >= 0L && revision <= t.state.revision) ->
    (match Map.find t.state.bindings (node_slot node) with
     | Some
         { node = expected
         ; handler = expected_handler
         ; callback = Editor (config, callback)
         }
       when Node_id.equal node expected
            && Handler_id.equal handler expected_handler
            && Text_input.Mode.equal (Text_input.Config.mode config) Multiline
            && (Text_input.Config.searchable config
                || Gpuio_protocol.Editor_search_wire.Mode.equal search.mode Closed) ->
       Text_input.Search.Expert.snapshot_of_wire ~window ~node search
       |> Result.ok
       |> Option.map ~f:(fun snapshot -> callback (Search_changed snapshot))
     | Some _ | None -> None)
  | Choice_picker_event (window, node, handler, revision, event)
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.(revision >= 0L && revision <= t.state.revision) ->
    (match Map.find t.state.bindings (node_slot node) with
     | Some
         { node = expected
         ; handler = expected_handler
         ; callback = Choice_picker (config, query_node, callback)
         }
       when Node_id.equal node expected && Handler_id.equal handler expected_handler ->
       Choice_picker.Expert.event_of_wire event ~window ~query_node
       |> Result.ok
       |> Option.filter ~f:(Choice_picker.Expert.accepts_event config)
       |> Option.map ~f:callback
     | Some _ | None -> None)
  | Combobox_selected (window, node, handler, revision, selected, snapshot)
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.(revision >= 0L && revision <= t.state.revision) ->
    (match Map.find t.state.bindings (node_slot node) with
     | Some
         { node = expected
         ; handler = expected_handler
         ; callback = Combobox (config, callback)
         }
       when Node_id.equal node expected && Handler_id.equal handler expected_handler ->
       let selection =
         let open Or_error.Let_syntax in
         let%bind id = Choice.Id.of_string selected in
         let%bind snapshot = Text_input.Expert.snapshot_of_wire ~window ~node snapshot in
         Combobox.Expert.selection config ~id ~snapshot
       in
       Result.ok selection |> Option.map ~f:(fun selected -> callback (Selected selected))
     | Some _ | None -> None)
  | Overlay_dismissed (window, node, handler, revision, reason)
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.(revision >= 0L && revision <= t.state.revision) ->
    (match Map.find t.state.bindings (node_slot node) with
     | Some
         { node = expected
         ; handler = expected_handler
         ; callback = Dismiss (config, callback)
         }
       when Node_id.equal node expected && Handler_id.equal handler expected_handler ->
       let reason =
         match reason with
         | Wire.Dismissal.Escape -> Overlay.Dismissal.Escape
         | Outside_pointer -> Outside_pointer
       in
       if Overlay.Expert.allows config reason then Some (callback reason) else None
     | Some _ | None -> None)
  | Calendar_viewport_changed (window, node, handler, revision, viewport)
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.(revision >= 0L && revision <= t.state.revision) ->
    (match Map.find t.state.calendar_viewport_bindings (node_slot node) with
     | Some { node = expected; handler = expected_handler; seen; on_change }
       when Node_id.equal node expected
            && Handler_id.equal handler expected_handler
            && Int64.(viewport.sequence > !seen) ->
       (match
          Calendar.Expert.viewport_of_wire ~window ~node ~observer:handler viewport
        with
        | Error _ -> None
        | Ok viewport_value ->
          seen := viewport.sequence;
          Some (on_change viewport_value))
     | Some _ | None -> None)
  | Hover_changed (window, node, handler, revision, hovered)
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.(revision >= 0L && revision <= t.state.revision) ->
    (match Map.find t.state.hover_bindings (node_slot node) with
     | Some { node = expected; handler = expected_handler; on_change }
       when Node_id.equal node expected && Handler_id.equal handler expected_handler ->
       Some (on_change hovered)
     | Some _ | None -> None)
  | Menu_open_changed (window, node, handler, revision, open_)
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.(revision >= 0L && revision <= t.state.revision) ->
    (match Map.find t.state.bindings (node_slot node) with
     | Some { node = expected; handler = expected_handler; callback = Menu callback }
       when Node_id.equal node expected && Handler_id.equal handler expected_handler ->
       Some (callback open_)
     | Some _ | None -> None)
  | Tooltip_open_changed (window, node, handler, revision, open_)
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.(revision >= 0L && revision <= t.state.revision) ->
    (match Map.find t.state.bindings (node_slot node) with
     | Some
         { node = expected
         ; handler = expected_handler
         ; callback = Tooltip (config, callback)
         }
       when Node_id.equal node expected
            && Handler_id.equal handler expected_handler
            && ((not open_) || not (Tooltip.Expert.is_disabled config)) ->
       Some (callback open_)
     | Some _ | None -> None)
  | Drag_source_event (window, node, handler, revision, sample)
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.(revision >= 0L && revision <= t.state.revision) ->
    (match Map.find t.state.bindings (node_slot node) with
     | Some
         { node = expected; handler = expected_handler; callback = Drag_source callback }
       when Node_id.equal node expected && Handler_id.equal handler expected_handler ->
       Drag_and_drop.Expert.source_event_of_wire sample
       |> Result.ok
       |> Option.map ~f:callback
     | Some _ | None -> None)
  | Drop_target_event (window, node, handler, revision, sample)
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.(revision >= 0L && revision <= t.state.revision) ->
    (match Map.find t.state.bindings (node_slot node) with
     | Some
         { node = expected; handler = expected_handler; callback = Drop_target callback }
       when Node_id.equal node expected && Handler_id.equal handler expected_handler ->
       Drag_and_drop.Expert.target_event_of_wire sample
       |> Result.ok
       |> Option.map ~f:callback
     | Some _ | None -> None)
  | Command_binding_observed (window, node, handler, revision, sample)
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.(revision >= 0L && revision <= t.state.revision) ->
    (match Map.find t.state.bindings (node_slot node) with
     | Some
         { node = expected
         ; handler = expected_handler
         ; callback = Command_binding_scope (config, seen, callback)
         }
       when Node_id.equal node expected
            && Handler_id.equal handler expected_handler
            && Int64.(sample.epoch > !seen) ->
       Option.map (Command_binding.Expert.of_wire config sample) ~f:(fun observation ->
         seen := sample.epoch;
         callback observation)
     | Some _ | None -> None)
  | Highlight_observed (window, node, handler, revision, sample)
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.(revision >= 0L && revision <= t.state.revision) ->
    (match Map.find t.state.bindings (node_slot node) with
     | Some
         { node = expected
         ; handler = expected_handler
         ; callback = Highlight_scope (config, callback)
         }
       when Node_id.equal node expected
            && Handler_id.equal handler expected_handler
            && Gpuio_protocol.Highlight_wire.Observation.valid_for sample config ->
       Highlight.Expert.observation_of_wire sample |> Result.ok |> Option.map ~f:callback
     | Some _ | None -> None)
  | Input_observed (window, node, handler, revision, sample)
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.(revision >= 0L && revision <= t.state.revision) ->
    (match Map.find t.state.bindings (node_slot node) with
     | Some
         { node = expected
         ; handler = expected_handler
         ; callback = Input_region (config, callback)
         }
       when Node_id.equal node expected
            && Handler_id.equal handler expected_handler
            && (not config.disabled)
            && List.exists config.subscriptions ~f:(fun subscription ->
              Gpuio_protocol.Input_wire.Kind.equal
                subscription.kind
                (Gpuio_protocol.Input_wire.Event.kind sample)) ->
       Input_region.Expert.event_of_wire sample |> Result.ok |> Option.map ~f:callback
     | Some _ | None -> None)
  | Pointer_event (window, node, handler, revision, sample)
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.(revision >= 0L && revision <= t.state.revision) ->
    (match Map.find t.state.bindings (node_slot node) with
     | Some { node = expected; handler = expected_handler; callback = Pointer callback }
       when Node_id.equal node expected && Handler_id.equal handler expected_handler ->
       Pointer.Expert.event_of_wire sample |> Result.ok |> Option.map ~f:callback
     | Some _ | None -> None)
  | Toast_dismissed (window, node, handler, revision, reason)
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.(revision >= 0L && revision <= t.state.revision) ->
    (match Map.find t.state.bindings (node_slot node) with
     | Some { node = expected; handler = expected_handler; callback = Toast callback }
       when Node_id.equal node expected && Handler_id.equal handler expected_handler ->
       Some (callback (Toast.Expert.dismissal reason))
     | Some _ | None -> None)
  | Palette_observed (window, node, handler, revision, snapshot)
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.(revision >= 0L && revision <= t.state.revision) ->
    (match Map.find t.state.bindings (node_slot node) with
     | Some
         { node = expected
         ; handler = expected_handler
         ; callback = Palette (_, _, seen, Some callback)
         }
       when Node_id.equal node expected
            && Handler_id.equal handler expected_handler
            && Int64.(snapshot.sequence > !seen) ->
       (match
          Command_palette.Expert.snapshot_of_wire ~window ~node ~observer:handler snapshot
        with
        | Error _ -> None
        | Ok value ->
          seen := snapshot.sequence;
          Some (callback value))
     | Some _ | None -> None)
  | Palette_dismissed (window, node, handler, revision, reason)
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.(revision >= 0L && revision <= t.state.revision) ->
    (match Map.find t.state.bindings (node_slot node) with
     | Some
         { node = expected
         ; handler = expected_handler
         ; callback = Palette (config, callback, _, _)
         }
       when Node_id.equal node expected && Handler_id.equal handler expected_handler ->
       Command_palette.Expert.dismissal config reason |> Option.map ~f:callback
     | Some _ | None -> None)
  | Command_invoked (window, node, handler, revision, id, generation, source)
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.(revision >= 0L && revision <= t.state.revision) ->
    (match Map.find t.state.bindings (node_slot node) with
     | Some
         { node = expected
         ; handler = expected_handler
         ; callback = Commands (registry, commands)
         }
       when Node_id.equal node expected && Handler_id.equal handler expected_handler ->
       let source_available =
         match source with
         | Wire.Command_source.Button source ->
           Option.exists
             (Map.find t.state.command_buttons (node_slot source))
             ~f:(fun button ->
               Node_id.equal button.node source
               && String.equal (Ui_command.Id.to_string button.command) id
               && (not button.loading)
               && Int64.(revision >= button.activation_revision))
         | Menu _ | Palette _ | Shortcut -> true
       in
       if
         source_available
         && List.exists commands ~f:(fun command ->
           String.equal command.id id && Int64.equal command.generation generation)
       then
         Result.ok (Ui_command.Id.of_string id)
         |> Option.bind ~f:(Ui_command.Registry.find registry)
         |> Option.bind ~f:Ui_command.Expert.invoke
       else None
     | Some _ | None -> None)
  | Drag_source_event _
  | Drop_target_event _
  | Input_observed _
  | Highlight_observed _
  | Command_binding_observed _
  | Pointer_event _
  | Toast_dismissed _
  | Palette_dismissed _
  | Palette_result _
  | Palette_observed _
  | Command_invoked _
  | Calendar_viewport_changed _
  | Hover_changed _
  | Menu_open_changed _
  | Tooltip_open_changed _
  | Overlay_dismissed _
  | Welcome _
  | Opened _
  | Closed _
  | Accepted _
  | Rejected _
  | Rendered _
  | Frame_requested _
  | Press _
  | Choice _
  | Combobox_selected _
  | Choice_picker_event _
  | Editor_search_observed _
  | Editor_event _
  | File_dialog_result _
  | Image_state _
  | Animation_endpoint _
  | Animation_program_event _
  | Rating_requested _
  | Carousel_requested _
  | Carousel_track_requested _
  | Tree_input _
  | List_input _
  | Table_input _
  | Table_columns_observed _
  | Slider_result _
  | Slider_event _
  | Color_input_result _
  | Calendar_result _
  | Otp_input_result _
  | Number_input_result _
  | Number_input_event _
  | Otp_input_event _
  | Color_input_event _
  | Calendar_event _
  | Container_selected _
  | List_retained _
  | List_viewport _
  | Asset_response _
  | Close_requested _
  | Quit_requested
  | Reopen_requested
  | Notification_pending
  | Notification_response _
  | Desktop_pending
  | Desktop_response _
  | Window_changed _
  | Window_response _
  | Window_capabilities _
  | Extension_event _
  | Split_group_resized _
  | Split_resized _
  | Canvas_event _
  | Chart_event _
  | Chart_response _
  | Canvas_response _
  | Document_response _
  | Document_profile_event _
  | Document_action _
  | Document_navigation _
  | Document_diff_event _
  | Document_preview_observed _
  | Editor_result _
  | Failed _
  | Stopped
  | Overloaded _ -> None
;;

let revision t = t.state.revision

let close t =
  t.closed <- true;
  t.state
  <- { t.state with
       root = None
     ; bindings = Int.Map.empty
     ; hover_bindings = Int.Map.empty
     ; calendar_viewport_bindings = Int.Map.empty
     ; command_buttons = Int.Map.empty
     ; nodes = Allocator.empty
     ; handlers = Allocator.empty
     }
;;
