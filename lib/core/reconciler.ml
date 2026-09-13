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
  | Click of (unit -> 'a)
  | Editor of (Text_input.Event.t -> 'a)
  | Choice of Choice.Config.t * (Choice.Id.t -> 'a)
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
  ; choice_appearance : Wire.Choice_appearance.t option
  ; children : 'a mounted list
  ; controllers : String.Set.t
  }

type 'a state =
  { root : 'a mounted option
  ; bindings : 'a binding Int.Map.t
  ; nodes : Allocator.t
  ; handlers : Allocator.t
  ; theme : Theme.t
  ; revision : int64
  ; epoch : int
  }

type 'a t =
  { owner : unit ref
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
  ; theme : Theme.t
  ; theme_unchanged : bool
  }

let create window =
  { owner = ref ()
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
;;

let compatible mounted view =
  let old = View.Expert.describe mounted.view
  and next = View.Expert.describe view in
  View.Expert.Kind.equal old.kind next.kind && Option.equal Key.equal old.key next.key
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
    let old_handler = Option.bind previous ~f:(fun mounted -> mounted.handler) in
    let callback =
      match
        description.on_click, description.editor, description.choice, description.combobox
      with
      | Some callback, None, None, None -> Some (Click callback)
      | None, Some editor, None, None -> Some (Editor editor.on_event)
      | None, None, Some choice, None ->
        if Choice.Config.is_disabled choice.config
        then None
        else Some (Choice (choice.config, choice.on_select))
      | None, None, None, Some combo -> Some (Combobox (combo.config, combo.on_event))
      | None, None, None, None -> None
      | _ -> fail "a view cannot combine incompatible handler kinds"
    in
    let rotate_handler =
      match description.combobox, previous with
      | Some combo, Some mounted ->
        Option.exists (View.Expert.describe mounted.view).combobox ~f:(fun old ->
          not
            (Bool.equal
               (Choice.Config.is_disabled (Combobox.Config.choices old.config))
               (Choice.Config.is_disabled (Combobox.Config.choices combo.config))))
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
      (match description.combobox with
       | Some combo -> Some combo.appearance
       | None -> Option.bind description.choice ~f:(fun choice -> choice.appearance))
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
    let controllers =
      let own =
        let controller =
          match description.editor, description.combobox with
          | Some editor, None -> Some editor.controller
          | None, Some combo -> Some combo.controller
          | None, None -> None
          | Some _, Some _ -> fail "incompatible controller descriptions"
        in
        Option.value_map controller ~default:String.Set.empty ~f:(fun key ->
          String.Set.singleton (Key.to_string key))
      in
      List.fold children ~init:own ~f:(fun keys child ->
        if not (Set.is_empty (Set.inter keys child.controllers))
        then fail "text input controller appears more than once in a window";
        Set.union keys child.controllers)
    in
    { view; id; handler; style; choice_appearance; children; controllers }
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
        ; theme
        ; theme_unchanged = Theme.equal theme t.state.theme
        }
      in
      let root =
        match view with
        | None ->
          Option.iter t.state.root ~f:(remove builder);
          None
        | Some view -> Some (mount builder ~depth:0 t.state.root view)
      in
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

let dispatch t = function
  | Wire.Event.Press (window, node, handler, revision)
    when (not t.closed)
         && Window_id.equal window t.window
         && Int64.(revision >= 0L && revision <= t.state.revision) ->
    (match Map.find t.state.bindings (node_slot node) with
     | Some binding
       when Node_id.equal node binding.node && Handler_id.equal handler binding.handler ->
       (match binding.callback with
        | Click callback -> Some (callback ())
        | Editor _ | Choice _ | Combobox _ -> None)
     | Some _ | None -> None)
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
