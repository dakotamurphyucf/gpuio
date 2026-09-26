module Ui_command = Command
open Core
open Gpuio_protocol
module Wire = Wire

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
    [@@deriving compare, sexp]
  end

  include T
  include Comparable.Make (T)

  let of_view view position =
    match (View.Expert.describe view).key with
    | Some key -> Key (Key.to_string key)
    | None -> Position position
  ;;
end

type 'a callback =
  | Container_query of
      Wire.Container_query.Config.t * int64 ref * (Container_query.Selection.t -> 'a)
  | Extension of Wire.Extension.Config.t * (Wire.Extension.Signal.t -> 'a)
  | Split_pane of Split_pane.Config.t * (Split_pane.Snapshot.t -> 'a)
  | Canvas of Wire.Canvas_view.Config.t * (Canvas.Event.t -> 'a)
  | Document of Text_source.Handle.t * (Document.Navigation.t -> 'a)
  | Virtual_list of List_identity.t * 'a View.Expert.virtual_list
  | Animation_program of int64 * (int64 * int64) ref * (Animation.Program.Event.t -> 'a)
  | Animation of int64 * int64 ref * (Animation.Event.t -> 'a)
  | Image of (Image.State.t -> 'a)
  | Pointer of (Pointer.Event.t -> 'a)
  | Drag_source of (Drag_and_drop.Source_event.t -> 'a)
  | Drop_target of (Drag_and_drop.Target_event.t -> 'a)
  | Toast of (Toast.Dismissal.t -> 'a)
  | Palette of Command_palette.Config.t * (Command_palette.Dismissal.t -> 'a)
  | Click of (unit -> 'a)
  | Commands of 'a Ui_command.Registry.t * Wire.Command.t list
  | Dismiss of Overlay.Config.t * (Overlay.Dismissal.t -> 'a)
  | Tooltip of Tooltip.Config.t * (bool -> 'a)
  | Editor of (Text_input.Event.t -> 'a)
  | Choice of Choice.Config.t * (Choice.Id.t -> 'a)
  | Rating of Rating.Config.t * (Rating.Request.t -> 'a)
  | Slider of Slider.Value.t * int64 ref * (Slider.Event.t -> 'a)
  | Number_input of int64 ref * (Number_input.Event.t -> 'a)
  | Otp_input of Otp_input.Policy.t * int64 ref * (Otp_input.Event.t -> 'a)
  | Color_input of int64 ref * (Color_input.Event.t -> 'a)
  | Calendar of Calendar.Mode.t * int64 ref * (Calendar.Event.t -> 'a)
  | Combobox of Combobox.Config.t * (Combobox.Event.t -> 'a)

type 'a binding =
  { node : Node_id.t
  ; handler : Handler_id.t
  ; callback : 'a callback
  }

type 'a mounted =
  { view : 'a View.t
  ; id : Node_id.t
  ; handler : Handler_id.t option
  ; style : Wire.Style.t list
  ; animation : Wire.Animation.Config.t option
  ; animation_seen : int64 ref
  ; animation_program : Wire.Animation_program.Config.t option
  ; program_seen : (int64 * int64) ref
  ; container_query : Wire.Container_query.Config.t option
  ; query_seen : int64 ref
  ; slider_seen : int64 ref
  ; number_input_seen : int64 ref
  ; otp_input_seen : int64 ref
  ; color_input_seen : int64 ref
  ; calendar_seen : int64 ref
  ; list_identity : List_identity.t option
  ; choice_appearance : Wire.Choice_appearance.t option
  ; children : 'a mounted list
  ; controllers : String.Set.t
  ; commands : Wire.Command.t list
  ; free_commands : String.Set.t
  ; menu : Wire.Menu.t option
  ; platform_menus : int
  }

type 'a state =
  { root : 'a mounted option
  ; bindings : 'a binding Int.Map.t
  ; nodes : Allocator.t
  ; handlers : Allocator.t
  ; theme : Theme.t
  ; revision : int64
  ; epoch : int
  ; command_generation : int64
  }

type 'a t =
  { owner : unit ref
  ; asset_owner : Asset.Expert.Owner.t option
  ; document_owner : Text_source.Expert.Owner.t option
  ; canvas_owner : Canvas_scene.Expert.Owner.t option
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
  { mutable nodes : Allocator.t
  ; mutable handlers : Allocator.t
  ; mutable bindings : 'a binding Int.Map.t
  ; mutable operations : Wire.Op.t list
  ; mutable operation_count : int
  ; mutable command_generation : int64
  ; theme : Theme.t
  ; theme_unchanged : bool
  ; asset_owner : Asset.Expert.Owner.t option
  ; document_owner : Text_source.Expert.Owner.t option
  ; canvas_owner : Canvas_scene.Expert.Owner.t option
  }

let create ?asset_owner ?document_owner ?canvas_owner window =
  { owner = ref ()
  ; asset_owner
  ; document_owner
  ; canvas_owner
  ; window
  ; closed = false
  ; state =
      { root = None
      ; bindings = Int.Map.empty
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
  builder.bindings <- Map.remove builder.bindings (node_slot mounted.id)
;;

let kind = function
  | View.Expert.Kind.Container -> Wire.Kind.Container
  | Text -> Text
  | Button -> Button
  | Input -> Input
  | Textarea -> Textarea
  | Checkbox -> Checkbox
  | Switch -> Switch
  | Radio_group -> Radio_group
  | Select -> Select
  | Combobox -> Combobox
  | Focus_scope -> Focus_scope
  | Tooltip -> Tooltip
  | Command_scope -> Command_scope
  | Command_button -> Command_button
  | Menu -> Menu
  | Command_palette -> Command_palette
  | Progress -> Progress
  | Toast -> Toast
  | Toast_stack -> Toast_stack
  | Pointer_area -> Pointer_area
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
  | Calendar -> Calendar
  | Virtual_list -> Virtual_list
  | Canvas_view -> Canvas_view
  | Document_view -> Document_view
  | Tab_bar -> Tab_bar
  | Tab_panel -> Tab_panel
  | Split_pane -> Split_pane
  | Extension -> Extension
;;

let compatible mounted view =
  let old = View.Expert.describe mounted.view
  and next = View.Expert.describe view in
  View.Expert.Kind.equal old.kind next.kind
  && Option.equal Key.equal old.key next.key
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

let rec mount builder ~depth previous view =
  if depth > 128 then fail "view exceeds native depth limit";
  match previous with
  | Some mounted when phys_equal mounted.view view && builder.theme_unchanged -> mounted
  | _ ->
    let description = View.Expert.describe view in
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
      Option.value_map previous ~default:(ref (-1L)) ~f:(fun old -> old.number_input_seen)
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
      | None, Some editor, None, None, None, None, None -> Some (Editor editor.on_event)
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
    let callback =
      match description.palette, callback with
      | Some palette, None -> Some (Palette (palette.config, palette.on_dismiss))
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
      match description.canvas, callback with
      | Some item, None ->
        Option.map item.on_event ~f:(fun callback ->
          Canvas (Canvas.Expert.to_wire item.config ~owner:builder.canvas_owner, callback))
      | None, callback -> callback
      | Some _, Some _ -> fail "canvas cannot combine another handler"
    in
    let callback =
      match description.document, callback with
      | Some item, None ->
        Option.map item.on_navigate ~f:(fun callback ->
          Document (Document.Config.source item.config, callback))
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
    let callback =
      match description.virtual_list, list_identity, callback with
      | Some list, Some identity, None ->
        if Option.is_some list.on_viewport || Option.is_some list.on_retain
        then Some (Virtual_list (identity, list))
        else None
      | None, None, callback -> callback
      | _ -> fail "incompatible virtual list callback"
    in
    let rotate_handler =
      (match description.extension, previous with
       | Some item, Some mounted ->
         Option.exists (View.Expert.describe mounted.view).extension ~f:(fun old ->
           not (Wire.Extension.Config.equal old.config item.config))
       | None, _ | Some _, None -> false)
      || (match description.split_pane, previous with
          | Some item, Some mounted ->
            Option.exists (View.Expert.describe mounted.view).split_pane ~f:(fun old ->
              not (Split_pane.Config.equal old.config item.config))
          | None, _ | Some _, None -> false)
      || (match description.canvas, previous with
          | Some item, Some mounted ->
            Option.exists (View.Expert.describe mounted.view).canvas ~f:(fun old ->
              not (Canvas.Config.equal old.config item.config))
          | None, _ | Some _, None -> false)
      || (match description.document, previous with
          | Some document, Some mounted ->
            Option.exists (View.Expert.describe mounted.view).document ~f:(fun old ->
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
    (match previous with
     | None ->
       emit builder (Create (id, kind description.kind, description.text, handler))
     | Some mounted ->
       if
         Option.is_none description.editor
         && Option.is_none description.combobox
         && not (String.equal (View.Expert.describe mounted.view).text description.text)
       then emit builder (Set_text (id, description.text));
       if not (Option.equal Handler_id.equal old_handler handler)
       then emit builder (Bind (id, handler)));
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
             | Platform_bar -> Platform_bar)
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
      if not (Option.equal Toast.Stack.equal old (Some config))
      then emit builder (Set_toast_stack (id, Toast.Expert.stack_to_wire config)));
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
    Option.iter description.document ~f:(fun document ->
      let old =
        Option.bind previous ~f:(fun mounted ->
          Option.map (View.Expert.describe mounted.view).document ~f:(fun old ->
            old.config))
      in
      if not (Option.equal Document.Config.equal old (Some document.config))
      then
        emit
          builder
          (Set_document
             ( id
             , Document.Expert.to_wire
                 document.config
                 ~owner:builder.document_owner
                 ~asset_owner:builder.asset_owner )));
    if Option.is_none description.avatar
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
             Number_input.Config.equal old.config number_input.config
             && Number_input.Value.equal old.initial number_input.initial))
      then
        emit
          builder
          (Set_number_input
             ( id
             , Number_input.Expert.config_to_wire number_input.config
             , Number_input.Expert.value_to_wire number_input.initial )));
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
             Color_input.Config.equal old.config input.config))
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
             , Calendar.Expert.month_to_wire calendar.initial_month )));
    Option.iter description.rating ~f:(fun rating ->
      let old =
        Option.bind previous ~f:(fun mounted ->
          Option.map (View.Expert.describe mounted.view).rating ~f:(fun item ->
            item.config))
      in
      if not (Option.equal Rating.Config.equal old (Some rating.config))
      then emit builder (Set_rating (id, Rating.Expert.to_wire rating.config)));
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
    Option.iter description.loading ~f:(fun config ->
      let old =
        Option.bind previous ~f:(fun mounted ->
          (View.Expert.describe mounted.view).loading)
      in
      if not (Option.equal Loading.Config.equal old (Some config))
      then emit builder (Set_loading (id, Loading.Expert.to_wire config)));
    Option.iter description.progress ~f:(fun progress ->
      let old =
        Option.bind previous ~f:(fun mounted ->
          (View.Expert.describe mounted.view).progress)
      in
      if not (Option.equal Progress.Config.equal old (Some progress))
      then emit builder (Set_progress (id, Progress.Expert.to_wire progress)));
    Option.iter description.palette ~f:(fun palette ->
      let old =
        Option.bind previous ~f:(fun mounted ->
          Option.map (View.Expert.describe mounted.view).palette ~f:(fun palette ->
            palette.config))
      in
      if not (Option.equal Command_palette.Config.equal old (Some palette.config))
      then emit builder (Set_palette (id, Command_palette.Expert.to_wire palette.config)));
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
      if not (Option.equal Text_input.Config.equal old (Some config))
      then emit builder (Set_editor (id, Text_input.Expert.config_to_wire config)));
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
    let placement description =
      match description.View.Expert.overlay, description.tooltip with
      | Some overlay, None ->
        Some (Overlay.Expert.placement overlay.config |> Placement.Expert.to_wire)
      | None, Some tooltip ->
        Some (Tooltip.Expert.placement tooltip.config |> Placement.Expert.to_wire)
      | None, None -> None
      | Some _, Some _ -> fail "incompatible overlay descriptions"
    in
    let next_placement = placement description in
    let old_placement =
      Option.bind previous ~f:(fun mounted ->
        placement (View.Expert.describe mounted.view))
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
    splice builder id old_children children;
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
      let old_identity = Option.bind previous ~f:(fun old -> old.list_identity) in
      if not (Option.exists old_identity ~f:(fun old -> phys_equal old identity))
      then emit builder (Set_list_order (id, List_identity.order identity));
      let rows identity children =
        List.map children ~f:(fun child ->
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
          emit
            builder
            (Scroll_list
               ( id
               , Virtual_list.Expert.scroll_to_wire
                   request
                   ~find_id:(List_identity.id identity)
                 |> value ))));
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
           | Button | Context | Bar -> false)
       then 1
       else 0)
      + List.sum (module Int) children ~f:(fun child -> child.platform_menus)
    in
    if platform_menus > 1 then fail "only one platform menu bar may be mounted per window";
    { view
    ; id
    ; handler
    ; style
    ; animation
    ; animation_seen
    ; animation_program
    ; program_seen
    ; container_query
    ; query_seen
    ; slider_seen
    ; number_input_seen
    ; otp_input_seen
    ; color_input_seen
    ; calendar_seen
    ; list_identity
    ; choice_appearance
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
        { nodes = t.state.nodes
        ; handlers = t.state.handlers
        ; bindings = t.state.bindings
        ; operations = []
        ; operation_count = 0
        ; command_generation = t.state.command_generation
        ; theme
        ; theme_unchanged = Theme.equal theme t.state.theme
        ; asset_owner = t.asset_owner
        ; document_owner = t.document_owner
        ; canvas_owner = t.canvas_owner
        }
      in
      let root =
        match view with
        | None ->
          Option.iter t.state.root ~f:(remove builder);
          None
        | Some view -> Some (mount builder ~depth:0 t.state.root view)
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
      | Some { node; callback = Virtual_list (identity, list); _ }
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
  | Wire.Event.Document_navigation (window, node, handler, revision, source, _, navigation)
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.(revision >= 0L && revision <= t.state.revision) ->
    (match Map.find t.state.bindings (node_slot node) with
     | Some
         { node = expected
         ; handler = expected_handler
         ; callback = Document (expected_source, callback)
         }
       when Node_id.equal node expected
            && Handler_id.equal handler expected_handler
            && Gpuio_protocol.Resource_id.equal
                 source
                 (Text_source.Expert.native_id expected_source) ->
       Document.Expert.navigation navigation |> Result.ok |> Option.map ~f:callback
     | Some _ | None -> None)
  | Wire.Event.List_viewport (window, node, handler, revision, viewport)
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.equal revision t.state.revision ->
    (match Map.find t.state.bindings (node_slot node) with
     | Some
         { node = expected
         ; handler = expected_handler
         ; callback = Virtual_list (identity, list)
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
        | Slider _
        | Number_input _
        | Otp_input _
        | Color_input _
        | Calendar _
        | Choice _
        | Combobox _
        | Dismiss _
        | Tooltip _
        | Commands _
        | Palette _
        | Toast _
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
        | Canvas _
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
       if Int64.(snapshot.revision <= !seen)
       then None
       else (
         match Number_input.Expert.event_of_wire ~window ~node event with
         | Error _ -> None
         | Ok event ->
           seen := snapshot.revision;
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
     | Some { node = expected; handler = expected_handler; callback = Editor callback }
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
  | Palette_dismissed (window, node, handler, revision, reason)
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.(revision >= 0L && revision <= t.state.revision) ->
    (match Map.find t.state.bindings (node_slot node) with
     | Some
         { node = expected
         ; handler = expected_handler
         ; callback = Palette (config, callback)
         }
       when Node_id.equal node expected && Handler_id.equal handler expected_handler ->
       Command_palette.Expert.dismissal config reason |> Option.map ~f:callback
     | Some _ | None -> None)
  | Command_invoked (window, node, handler, revision, id, generation, _)
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
       if
         List.exists commands ~f:(fun command ->
           String.equal command.id id && Int64.equal command.generation generation)
       then
         Result.ok (Ui_command.Id.of_string id)
         |> Option.bind ~f:(Ui_command.Registry.find registry)
         |> Option.bind ~f:Ui_command.Expert.invoke
       else None
     | Some _ | None -> None)
  | Drag_source_event _
  | Drop_target_event _
  | Pointer_event _
  | Toast_dismissed _
  | Palette_dismissed _
  | Command_invoked _
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
  | Editor_event _
  | File_dialog_result _
  | Image_state _
  | Animation_endpoint _
  | Animation_program_event _
  | Rating_requested _
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
  | Window_changed _
  | Window_response _
  | Window_capabilities _
  | Extension_event _
  | Split_resized _
  | Canvas_event _
  | Canvas_response _
  | Document_response _
  | Document_navigation _
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
     ; nodes = Allocator.empty
     ; handlers = Allocator.empty
     }
;;
