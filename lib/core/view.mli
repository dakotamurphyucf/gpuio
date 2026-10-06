(** Pure, immutable UI descriptions. Actions need not be Bonsai effects: tests and
    other runtimes can use ordinary variants. Callbacks run only on the OCaml UI
    domain, after generation validation, using the latest accepted closure. *)
type 'action t

(** Observe native hover on a direct Button, Command_button or Link root.
    Subscription identity is independent of click handling and native focus.
    Initial delivery is the last observed native state (false before first hit
    testing), followed by ordered changes. Native hover follows GPUI input
    modality and pointer/drag behavior; unavailable or culled controls report false.
    Observations are asynchronous history, not authorization for an action.
    Removing/readding the observer retires queued callbacks from its old generation.
    Compose this inside a Tooltip anchor. Other root kinds return an error. *)
val with_hover : 'action t -> on_change:(bool -> 'action) -> 'action t Core.Or_error.t

(** Replace the description's sibling key without adding a layout wrapper.
    Descendants, styles and callbacks are unchanged. A changed key replaces native
    identity on the next reconciliation, just like a constructor's [~key]. *)
val with_key : 'action t -> Key.t -> 'action t

(** Attach scrollbar presentation to an ordinary container or a managed
    list/tree/table root. It uses that viewport's existing native scroll handle;
    it does not enable overflow or change the scroll axis. [None] restores the
    owner's default bar presentation without resetting its offset or children.
    Managed list/table scrollbar visibility flags still suppress their bars.
    Rejects roots without a supported scroll owner, including native editors. *)
val with_scrollbar : 'action t -> Scrollbar.t option -> 'action t Core.Or_error.t

(** Attach native title-bar/resize/exclusion policy to an ordinary container.
    None clears it. Rust owns gestures; this does not create a focusable control
    or supply native chrome by itself. Use Window.Chrome.Custom for a custom bar.
    Interactive descendants keep their own input; Exclude marks custom content. *)
val with_window_region : 'action t -> Window_region.t option -> 'action t Core.Or_error.t

(** Preserve keyed identity while applying validated native semantics. General
    presentation roles apply to containers/text; Navigation requires a container.
    Link applies to buttons. Current-item metadata supports text/buttons only. Field
    metadata applies to native input/textarea/combobox, checkbox/switch and
    radio/select roots. Ambiguous or unsupported placements return an error. *)
val with_accessibility : 'action t -> Accessibility.t -> 'action t Core.Or_error.t

val text : ?key:Key.t -> ?style:Style.t -> string -> 'action t

(** One logical text flow with foreground runs. Uncovered ranges inherit [style].
    Colors resolve against the current theme during reconciliation; missing tokens
    return a preparation error. Color-only updates retain native selection; source
    changes follow ordinary text selection behavior. Copy and default accessibility
    expose the complete source string. This is ordinary text, not a native editor. *)
val styled_text : ?key:Key.t -> ?style:Style.t -> Text_content.t -> 'action t

(** Set or clear a text-glyph shimmer on ordinary [text] or [styled_text]. This
    preserves the key, source, style and accessibility metadata. Other view kinds
    are rejected. [Some config] requires valid UTF-8 of at most 16,384 bytes;
    [None] clears the effect without applying its source-size limit. The native
    painter falls back to static text above 256 lines or 4,096 shaped glyphs.
    Each window frame additionally admits at most 64 visible animation candidates
    and 16,384 shaped glyphs across admitted effects, in paint order. Rejected
    effects keep ordinary text and pause; a later frame can resume them when
    capacity becomes available. This bounds overlay work, not ordinary layout.

    Experimental: mounted native rendering is implemented; public component
    acceptance and host capability advertisement remain in progress. *)
val with_text_shimmer
  :  'action t
  -> Text_shimmer.Config.t option
  -> 'action t Core.Or_error.t

(** Native chart backed by a scoped data registration. Style determines its size. *)
val chart
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?on_event:(Chart.Event.t -> 'action)
  -> Chart.Config.t
  -> 'action t

(** Retained native canvas backed by a scoped scene registration. Style determines
    its size. Native interaction observations enqueue application actions. *)

val canvas
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?on_event:(Canvas.Event.t -> 'action)
  -> Canvas.Config.t
  -> 'action t

(** Native Markdown/HTML/code/unified-diff display, backed by a scoped document
    resource. Parsing, selection and copy are native; navigation is asynchronous.

    [on_preview] requires Markdown/HTML Flow and reports the painted presentation
    asynchronously, including expansion with no line limit.
    [on_diff] requires an explicit [Document.Config.diff] value. Navigation and
    diff observations share the node's asynchronous handler.
    [on_action] requires Markdown/HTML and is mandatory for custom code/table
    actions. It receives an immutable snapshot with installed source provenance. *)
val document
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?on_navigate:(Document.Navigation.t -> 'action)
  -> ?on_diff:(Document.Diff.Event.t -> 'action)
  -> ?on_preview:(Document.Preview.Event.t -> 'action)
  -> ?on_action:(Document.Actions.Event.t -> 'action)
  -> Document.Config.t
  -> 'action t

(** Clear a profile and suppress inheritance from application defaults. *)
val without_document_profile : 'action t -> 'action t Core.Or_error.t

(** Attach a typed static profile to a rich Markdown/HTML document. Explicit
    attachment overrides application defaults; omission inherits them. *)
val with_document_profile
  :  'action t
  -> 'event Document.Profile.Instance.t
  -> on_event:('event Document.Profile.Event.t -> 'action)
  -> 'action t Core.Or_error.t

(** An explicit accessible name must be nonempty and at most 1024 bytes;
    invalid names raise, as with literal styles built with [Style.create_exn]. *)
val button
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?config:Button.Config.t
  -> ?accessible_name:string
  -> ?disabled:bool
  -> ?leading_icon:Icon.Decoration.t
  -> ?trailing_icon:Icon.Decoration.t
  -> on_click:(unit -> 'action)
  -> string
  -> 'action t

(** A single native link target around composed passive content. The accessible
    name and Tab policy come from [config]; navigation is the supplied asynchronous
    action. Allowed descendants are containers, text/styled text, images/icons,
    avatars, loading indicators and animations without callbacks. Nested controls,
    selectable text, scrolling and pointer shields are rejected. At most 4096
    descendants and 128 content levels. Resource registrations stay caller-owned. *)
val link
  :  ?key:Key.t
  -> ?style:Style.t
  -> Link.Config.t
  -> on_click:(unit -> 'action)
  -> 'action t list
  -> 'action t Core.Or_error.t

(** Icon-only button with a required accessible label. The icon has no separate
    focus/action target. [style] customizes the button; the decoration styles its icon. *)
val icon_button
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?config:Button.Config.t
  -> ?disabled:bool
  -> label:string
  -> on_click:(unit -> 'action)
  -> Icon.Decoration.t
  -> 'action t

(** One native action around checked passive content, including progress/loading.
    Loading blocks activation but retains focus; content owns no callbacks. *)
val button_with_content
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?config:Button.Config.t
  -> ?disabled:bool
  -> accessible_name:string
  -> on_click:(unit -> 'action)
  -> 'action t
  -> 'action t Core.Or_error.t

(** Uses the registry command label as the accessible name. Loading is per owner. *)
val command_button_with_content
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?config:Button.Config.t
  -> command:Command.Id.t
  -> 'action t
  -> 'action t Core.Or_error.t

(** Controlled application values. Activation emits an intent, never a Boolean
    computed from the last render. Apply it to the current model (for example
    [Bonsai.Cont.toggle] or a state machine using [Check_state.activate]). Disabled
    controls are inert, excluded from Tab traversal and invalidate their handler.
    [appearance] styles the indicator and mark independently of the root/label.
    Omitting it restores defaults while preserving the keyed native control. *)
val checkbox
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?appearance:Control_appearance.t
  -> ?tab_order:Tab_order.t
  -> ?accessible_name:string
  -> ?disabled:bool
  -> state:Check_state.t
  -> on_toggle:(unit -> 'action)
  -> string
  -> 'action t

val switch
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?appearance:Control_appearance.t
  -> ?tab_order:Tab_order.t
  -> ?accessible_name:string
  -> ?disabled:bool
  -> checked:bool
  -> on_toggle:(unit -> 'action)
  -> string
  -> 'action t

(** One native radio. Unchecked activation requests selection; checked activation
    is a no-op. [on_select] should select an application value, never toggle it.
    No implicit group or Arrow navigation is created. [position] is semantic
    metadata; [tab_order] preserves pointer/AX focus when excluded from Tab. *)
val radio
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?appearance:Control_appearance.t
  -> ?accessible_name:string
  -> ?disabled:bool
  -> ?tab_order:Tab_order.t
  -> ?position:Radio.Position.t
  -> checked:bool
  -> on_select:(unit -> 'action)
  -> string
  -> 'action t

(** Passive rich label with the same ownership/name validation as checkbox labels. *)
val radio_with_label
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?appearance:Control_appearance.t
  -> ?disabled:bool
  -> ?tab_order:Tab_order.t
  -> ?position:Radio.Position.t
  -> accessible_name:string
  -> checked:bool
  -> on_select:(unit -> 'action)
  -> 'action t
  -> 'action t Core.Or_error.t

(** Registry definitions are inherited by descendants; the nearest definition of
    an ID wins. A command button uses the registry's label/enabled state. Missing
    command references are rejected before a view update is submitted. *)
val command_scope
  :  ?key:Key.t
  -> ?style:Style.t
  -> commands:'action Command.Registry.t
  -> 'action t list
  -> 'action t

val command_button
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?config:Button.Config.t
  -> ?leading_icon:Icon.Decoration.t
  -> ?trailing_icon:Icon.Decoration.t
  -> command:Command.Id.t
  -> unit
  -> 'action t

(** By default mounting opens a modal, native-owned search session. Search/navigation do not
    roundtrip through OCaml. Escape, permitted outside clicks, or selecting a
    command close it natively and restore the prior eligible focus. [on_dismiss]
    should remove the view. A closed session stays closed until unmounted and
    mounted again, or replaced with a new key; metadata updates do not reopen it.
    [Config.presentation=Embedded] instead stays in normal layout, without
    autofocus/trapping. Selecting commands keeps it mounted and does not emit
    [Selected]. Escape requests cancellation through [on_dismiss] without hiding
    the embedded session; outside clicks do not dismiss it.
    The query is independent of document editors and never becomes the target of
    registry native-edit commands. Commands resolve at the palette's tree location.
    [on_change] receives initial and changed native query/highlight snapshots
    asynchronously. Omission creates no subscription. Changing its presence
    retires queued callbacks; replacing the callback preserves the subscription. *)
val command_palette
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?appearance:Command_palette.Appearance.t
  -> config:Command_palette.Config.t
  -> ?on_change:(Command_palette.Snapshot.t -> 'action)
  -> on_dismiss:(Command_palette.Dismissal.t -> 'action)
  -> unit
  -> 'action t

(** Add ordinary interactive header/footer/empty views and passive command-row
    content to a direct palette. Row content is keyed by known unique command
    IDs; the native row retains its registry name, action and keyboard ownership.
    Rows may have different measured heights. Filtered rows and an inactive empty
    view cannot receive input. Header/footer controls retain ordinary focus/key
    behavior; an unhandled Escape dismisses the palette after child handling.
    Query editing and the captured document target remain native-owned.
    All slots together are limited to 4096 nodes and 128 levels. Calling this
    again replaces all content; absent slots use the built-in presentation. *)
val with_palette_content
  :  'action t
  -> ?header:'action t
  -> ?footer:'action t
  -> ?empty:'action t
  -> items:(Command.Id.t * 'action t) list
  -> unit
  -> 'action t Core.Or_error.t

(** Native-managed menu navigation resolves the same command registry as buttons
    and shortcuts. The context menu wraps one arbitrary child and opens on right
    click or Shift-F10. Escape restores prior focus. [menu_bar] defaults to the
    native menu bar on macOS and an in-window bar on Linux; [platform=false]
    renders an in-window bar on either platform. At most one platform bar may be
    mounted per window. Menus are replaced with the active window's definitions.
    [Menu.Item.Label] is supported in drawn menus; [menu_bar ~platform:true]
    rejects section labels, including in nested submenus.

    [on_open_change] observes native visibility, with an initial snapshot when
    attached and subsequent root open/close transitions. Submenu movement and
    style-only changes do not emit repeated snapshots. Menu-definition changes
    retire queued observations from the old definition. Native culling closes a
    retained menu; recreating its native owner emits a fresh closed snapshot.
    Removing the OCaml node retires the callback without a final call.
    [placement] positions only the root popup;
    omission preserves Bottom/Start with a two-pixel gap. Native navigation and
    command activation never wait for this callback. *)
val menu_button
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?appearance:Menu.Appearance.t
  -> ?placement:Placement.t
  -> ?on_open_change:(bool -> 'action)
  -> menu:Menu.t
  -> unit
  -> 'action t

(** [platform=true] uses an AppKit popup on macOS and the drawn menu on Linux.
    The default is [false]. The OS popup can extend outside the window and owns
    its appearance; [appearance] configures only the drawn fallback. Rich View
    row content is rejected for this presentation on both platforms. *)
val context_menu
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?appearance:Menu.Appearance.t
  -> ?platform:bool
  -> menu:Menu.t
  -> 'action t
  -> 'action t

(** Native Cut/Copy/Paste/Select-all menu bound to this exact editor placement.
    [child] must be a direct [text_input] view, single-line or multiline. Other
    kinds return an error. The menu opens on right-click or Shift-F10 and returns
    focus on Escape/activation. Pointer placement follows native selection rules;
    opening with the keyboard preserves selection. Disabled,
    hidden, modal-blocked and composing editors cannot open it. Password and
    read-only policies are checked natively again when an action is invoked.

    Keep this wrapper mounted around the editor; adding/removing a wrapper
    changes the tree placement and may remount the editing session. Menu enabled
    state, labels and appearance may change without replacing the editor. The
    wrapper defaults to the child's key, preserving keyed sibling reorders. It
    declares scoped native commands and does not install keyboard shortcuts.
    [item_content] uses the same bounded passive labels as [with_menu_item_content]. *)
val editor_menu
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?appearance:Menu.Appearance.t
  -> ?config:Editor_menu.t
  -> ?item_content:(Menu.Item_path.t * 'action t) list
  -> 'action t
  -> 'action t Core.Or_error.t

(** Decorate a direct [text_input] view without adding an ancestor or replacing
    its native editor. Leading/trailing slots have stable structural identities;
    changing them or the frame config preserves editing state. [on_reveal] adds
    a focus-preserving button on password inputs and emits an intent to the
    application; apply it to current application state. It does not mutate the
    native privacy policy by itself. Other inputs with [on_reveal], and multiline
    inputs with a clear control, return an error. Apply [editor_menu] after this
    helper when both are wanted. Ordinary styles on the input style the frame. *)
val input_frame
  :  ?config:Input_frame.t
  -> ?leading:'action t
  -> ?trailing:'action t
  -> ?on_reveal:(unit -> 'action)
  -> 'action t
  -> 'action t Core.Or_error.t

(** Decorate a direct [number_input], retaining its native editor and step
    actions. Leading/trailing slots accept ordinary interactive views; disabling
    the numeric control disables those descendants. Read-only only restricts
    numeric editing and stepping. Decrement/increment content must be passive
    and retains the native buttons' labels, action and repeat behavior. Stable
    role slots preserve surviving content when other slots change. Applying the
    helper again replaces its slots; omitting it restores the plain numeric view. *)
val number_frame
  :  ?appearance:Number_input.Appearance.t
  -> ?leading:'action t
  -> ?trailing:'action t
  -> ?decrement:'action t
  -> ?increment:'action t
  -> 'action t
  -> 'action t Core.Or_error.t

(** A native-coordinated horizontal split action. At least one part is required.
    [primary] must be a button/command button; [menu] must be a menu button.
    Either may be wrapped in at most eight Tooltip anchors. Other compositions
    return an error. Internal keyed part slots preserve each surviving owner's
    identity when the other part is added or removed, including full caller keys.
    Each part retains independent activation, focus and disabled/loading policy.
    The pair aligns to the start of its parent's cross axis by default, keeping
    shared hover within its content width in a column. [style] may override this.
    Whole-pair disabled policy uses [style]. Shared paint stays native; no
    [on_open_change] subscription is required. In split mode inner corners and
    the menu's left border are removed; single-part mode keeps the supplied
    corners/borders. Size, variant and selected appearance use ordinary styles. *)
val split_button
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?appearance:Split_button.Appearance.t
  -> ?primary:'action t
  -> ?menu:'action t
  -> unit
  -> 'action t Core.Or_error.t

val menu_bar
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?appearance:Menu.Appearance.t
  -> ?platform:bool
  -> Menu.t list
  -> 'action t Core.Or_error.t

(** Replace the visual label of drawn menu items with passive view content.
    The receiver must be a menu button, context menu or drawn menu bar. Paths
    address positions in its definitions; unknown/duplicate paths, separators
    and platform bars are rejected. Omitted paths retain their string labels.
    Passing an empty list removes all custom content. The command/submenu/label
    still supplies the accessible name; descendants add no independent actions.
    Content can compose icons, text and other passive rich-label elements. The
    complete slot/content forest is bounded to 4096 nodes and 128 levels and
    uses the rich control-label restrictions. Rows retain the configured uniform
    menu row height; content must fit it. Resource registrations remain caller-
    owned. Content is keyed by position: moving it to another path remounts it. *)
val with_menu_item_content
  :  'action t
  -> items:(Menu.Item_path.t * 'action t) list
  -> 'action t Core.Or_error.t

(** Rich labels are passive, with one native activation/focus target. The required
    name is nonblank UTF-8 without NUL, at most 1024 bytes. Decorative label
    descendants are not separately announced. Same-key plain/rich changes retain
    the native control. See [Control_appearance] for indicator/label order. *)
val checkbox_with_label
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?appearance:Control_appearance.t
  -> ?tab_order:Tab_order.t
  -> ?disabled:bool
  -> accessible_name:string
  -> state:Check_state.t
  -> on_toggle:(unit -> 'action)
  -> 'action t
  -> 'action t Core.Or_error.t

val switch_with_label
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?appearance:Control_appearance.t
  -> ?tab_order:Tab_order.t
  -> ?disabled:bool
  -> accessible_name:string
  -> checked:bool
  -> on_toggle:(unit -> 'action)
  -> 'action t
  -> 'action t Core.Or_error.t

(** Native focus policy for the supplied subtree. Scope lifetime follows keyed
    mount/unmount; changing descendants preserves the scope's restoration target. *)
val focus_scope
  :  ?key:Key.t
  -> ?style:Style.t
  -> config:Focus_scope.t
  -> 'action t list
  -> 'action t

(** [None] closes and unmounts modal content. A dialog traps focus until closed.
    [backdrop] overrides the default half-opacity black using a theme-aware color.
    A transparent backdrop still blocks background input and traps focus.
    [motion] defaults to [Overlay.Motion.Immediate]; [Enter] animates the native
    panel/backdrop on opening, respects reduced motion and never delays removal.
    Presentation updates do not replay entry or replace child owners.
    [style] applies to the panel, and content can contain any ordinary views. *)
val dialog
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?backdrop:Color.t
  -> ?motion:Overlay.Motion.t
  -> config:Overlay.Config.t
  -> on_dismiss:(Overlay.Dismissal.t -> 'action)
  -> 'action t option
  -> 'action t

(** A modal edge-attached drawer, sharing dialog focus/restoration and dismissal
    ordering. [None] unmounts native content immediately; application models and
    Eio task lifetimes remain the caller's responsibility. No close animation
    retains removed resources. [backdrop] follows [dialog] paint-only semantics. *)
val sheet
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?backdrop:Color.t
  -> ?motion:Overlay.Motion.t
  -> config:Sheet.Config.t
  -> on_dismiss:(Overlay.Dismissal.t -> 'action)
  -> 'action t option
  -> 'action t

(** An alert dialog enters its first eligible control. Put the safe/cancel action
    first in content order, especially for destructive confirmation. Confirmation
    uses ordinary buttons; Enter never implicitly confirms on the panel itself.
    [None] closes and unmounts. Backdrop clicks are ignored, including with a
    transparent [backdrop]. *)
val alert_dialog
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?backdrop:Color.t
  -> ?motion:Overlay.Motion.t
  -> config:Alert_dialog.Config.t
  -> on_dismiss:(Overlay.Dismissal.t -> 'action)
  -> 'action t option
  -> 'action t

(** The anchor remains mounted when closed. Content is positioned against its
    current frame's bounds, enters focus without trapping, and restores on close.
    A direct button or command-button anchor exposes dialog-popup and expanded
    accessibility state from the accepted native tree and is the preferred focus
    return target when closing with focus inside the popup. Its role and activation
    callback are unchanged. Other anchors retain previous-focus restoration and
    their own semantics; no nested control is inferred to be the popup trigger. *)
val popover
  :  ?key:Key.t
  -> ?style:Style.t
  -> config:Overlay.Config.t
  -> on_dismiss:(Overlay.Dismissal.t -> 'action)
  -> anchor:'action t
  -> 'action t option
  -> 'action t

(** Content remains retained while native open state hides it. This preserves
    Bonsai models and editor buffers; unmount the tooltip to dispose its content.
    [style] customizes the tooltip panel. The anchor retains its own styles. *)
val tooltip
  :  ?key:Key.t
  -> ?style:Style.t
  -> config:Tooltip.Config.t
  -> ?on_open_change:(bool -> 'action)
  -> anchor:'action t
  -> content:'action t
  -> unit
  -> 'action t

(** An interactive preview with nonmodal dialog semantics, rather than tooltip
    help text. Hover never moves focus; use a focusable anchor (normally a button
    or link) for keyboard access. Tab can enter the content and leave normally.
    Escape and trigger pointer-down request closure until a fresh hover/focus
    entry. Outside pointer-down also requests closure. Closing focused content
    restores the first eligible painted anchor control when available, otherwise
    using the normal enclosing/window fallback. It never steals outside focus.

    Native content is retained while closed, including editor buffers. Hidden
    content cannot receive input or appear in accessibility. Unmount disposes
    native resources and timers; Bonsai models and Eio tasks keep their explicit
    lifetimes. [style] applies to the card panel. Managed changes are observations;
    Controlled changes are requests, with visibility following the accepted value. *)
val hover_card
  :  ?key:Key.t
  -> ?style:Style.t
  -> config:Hover_card.Config.t
  -> ?on_open_change:(bool -> 'action)
  -> anchor:'action t
  -> content:'action t
  -> unit
  -> 'action t

val row : ?key:Key.t -> ?style:Style.t -> 'action t list -> 'action t

(** A composed custom title bar with native drag/double-click behavior. Use with
    [Window.Chrome.Custom]. It reserves 80 logical pixels on macOS while windowed,
    12 otherwise; its minimum height is 34. Feed the current backend and fullscreen
    observation (for example from [App.Window.on_change]). User style overrides
    these layout defaults. Native traffic lights remain OS-owned. Compose ordinary
    buttons for commands and the application's close-decision request; this helper
    never force-closes a window. Use [Window_region.Exclude] for custom interactive
    descendants that are not ordinary native controls. *)
val title_bar
  :  ?key:Key.t
  -> ?style:Style.t
  -> backend:Window.Backend.t
  -> fullscreen:bool
  -> 'action t list
  -> 'action t

(** Ordinary accessible buttons for a custom Linux title bar. No buttons are
    drawn on macOS (native traffic lights) or with server decorations. Client
    decorations show only supported minimize/maximize actions, plus Close.
    Feed the current window snapshot. Callback ownership and keyboard behavior
    are the same as [button]; [on_close] must request the application's close
    decision rather than force-closing. Stable part keys preserve other controls
    when capabilities change. Styles apply to the row and each button. *)
val window_controls
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?button_style:Style.t
  -> backend:Window.Backend.t
  -> snapshot:Window.Snapshot.t
  -> on_minimize:(unit -> 'action)
  -> on_zoom:(unit -> 'action)
  -> on_close:(unit -> 'action)
  -> unit
  -> 'action t

val column : ?key:Key.t -> ?style:Style.t -> 'action t list -> 'action t

(** Retains all supplied descriptions, building native elements only for the
    viewport/overscan. Use the managed Bonsai component for bounded row graphs.
    Keys must be unique. A fixed config height applies to the row wrapper. *)
val virtual_list
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?on_viewport:(Virtual_list.Viewport.t -> 'action)
  -> ?scroll:Virtual_list.Scroll_request.t
  -> config:Virtual_list.Config.t
  -> (Key.t * 'action t) list
  -> 'action t Core.Or_error.t

(** Attach ordered native input to a virtual list with List_box accessibility.
    Rows expose Option_item metadata; non-option rows may be section headings.
    This does not own or mutate selection. Reduce intents against current data
    and use incarnation-safe keys when membership can be removed/reinserted.
    The cursor must belong to the logical order and be enabled if mounted.
    Query references resolve during reconciliation to a unique direct sibling
    Input; invalid relationships reject preparation without advancing epochs.
    Tree/table input cannot share this owner. Rebuilding without this attachment
    clears input, preserving its generation watermark for later reinstallation. *)
val with_list_input
  :  'action t
  -> config:List_input.Config.t
  -> on_input:(Key.t List_input.t -> 'action)
  -> 'action t Core.Or_error.t

val grid
  :  ?key:Key.t
  -> ?style:Style.t
  -> columns:int
  -> 'action t list
  -> 'action t Core.Or_error.t

(** One controller identifies one placement across a window tree. Initial text is
    read only on native creation; use explicit editor commands for later edits. *)
val text_input
  :  ?style:Style.t
  -> ?initial_text:string
  -> controller:Key.t
  -> config:Text_input.Config.t
  -> on_event:(Text_input.Event.t -> 'action)
  -> unit
  -> 'action t Core.Or_error.t

(** One native Tab stop. Arrow/Home/End keys navigate enabled options; native
    activation requests a stable option ID. OCaml owns the selected value.
    Requests can match the currently rendered value: rapid navigation must keep
    the final intent even while earlier requests await an OCaml commit. Apply
    each request to the current model rather than treating it as a value-change
    notification. Native radio metadata exposes both selected and toggled state,
    with one-based position and total collection size (including disabled items).
    [appearance] applies to each option's indicator, label gap and label order;
    root [style] controls arrangement of the options. *)
val radio_group
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?appearance:Control_appearance.t
  -> ?tab_order:Tab_order.t
  -> config:Choice.Config.t
  -> on_select:(Choice.Id.t -> 'action)
  -> unit
  -> 'action t

(** Passive rich labels keyed by option ID. Reject unknown/duplicate IDs; omitted
    IDs use their string labels. Configured labels still supply accessible names.
    Reorders retain each option's keyed label slot. The total slot/label content
    is bounded to 4096 nodes and 128 levels. *)
val radio_group_with_labels
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?appearance:Control_appearance.t
  -> ?tab_order:Tab_order.t
  -> config:Choice.Config.t
  -> labels:(Choice.Id.t * 'action t) list
  -> on_select:(Choice.Id.t -> 'action)
  -> unit
  -> 'action t Core.Or_error.t

(** Decorative tab labels keyed by [Choice.Id]. Accessible names and selection
    remain in [config]. Omitted IDs use their string labels; duplicate/unknown
    IDs and interactive or unbounded label trees are rejected as for
    [radio_group_with_labels]. Reordering keeps keyed label owners. This does
    not supply interactive close buttons or tab overflow management. *)
val tab_bar_with_labels
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?appearance:Tab_bar.Appearance.t
  -> ?viewport:Tab_bar.Viewport.t
  -> ?motion:Tab_bar.Motion.t
  -> config:Choice.Config.t
  -> labels:(Choice.Id.t * 'action t) list
  -> on_select:(Choice.Id.t -> 'action)
  -> unit
  -> 'action t Core.Or_error.t

module Tab_content : sig
  type 'action view := 'action t

  module Label : sig
    type 'action t =
      | Default
      | Custom of 'action view
      | Hidden
  end

  type 'action t

  (** The label is decorative; prefix and suffix keep their ordinary independent
      actions, focus and native owners. Part hit areas do not select the tab. [Hidden] preserves the configured
      accessible name and exempts icon-only content from [max_width]. *)
  val create
    :  ?prefix:'action view
    -> ?label:'action Label.t
    -> ?suffix:'action view
    -> unit
    -> 'action t Core.Or_error.t
end

(** Structured tabs with optional independent controls. Unknown/duplicate IDs,
    blank configured names and more than 4096 nodes or 128 levels (including
    structural slots) are rejected. Omitted IDs use [Label.Default].
    [max_width] is finite, 1..1e6 logical pixels, and caps the whole tab;
    default labels ellipsize and custom labels clip. Prefix/suffix do not shrink.
    An impossibly small cap can clip controls; fully clipped controls lose Tab
    eligibility. Padding can impose a larger minimum native box.
    Explicit target sizing styles still follow native layout constraints.
    Choice disabled flags suppress selection; independent part controls remain
    usable. Ancestor [Disabled]/[Inert] suppress the entire subtree. *)
val tab_bar_with_content
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?appearance:Tab_bar.Appearance.t
  -> ?max_width:float
  -> ?viewport:Tab_bar.Viewport.t
  -> ?motion:Tab_bar.Motion.t
  -> config:Choice.Config.t
  -> content:(Choice.Id.t * 'action Tab_content.t) list
  -> on_select:(Choice.Id.t -> 'action)
  -> unit
  -> 'action t Core.Or_error.t

(** A stable frame for a direct tab bar. Prefix/suffix stay outside its native
    horizontal viewport; [trailing] is an ordinary view inside the scroller,
    after the logical tabs. The default trailing space is 12 pixels when a
    suffix or menu exists, empty otherwise. Explicit trailing content is always kept.
    The optional all-tabs menu sits before the suffix and uses current tab choices.
    Existing viewport/reveal settings are preserved; absent settings opt in to
    [Tab_bar.Viewport.default]. Outer [style] sizes the frame; tab styles still
    size its viewport. Keep the frame mounted to retain descendant owners.
    Rejects non-tab/already-framed views and content over 4096 nodes/128 levels.
    Omitted [key] inherits the input tab bar's key. *)
val tab_bar_frame
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?menu:Tab_bar.Menu.t
  -> ?prefix:'action t
  -> ?suffix:'action t
  -> ?trailing:'action t
  -> 'action t
  -> 'action t Core.Or_error.t

(** A statically registered native component. Changing its schema replaces the
    node; increasing its generation resets native state. Events are delivered
    asynchronously and obsolete property callbacks are rejected. *)
val extension
  :  ?key:Key.t
  -> ?style:Style.t
  -> on_event:('event Extension.Event.t -> 'action)
  -> 'event Extension.Instance.t
  -> 'action t

(** Flat native panels with stable ID ownership. Supply exactly one content for
    every configured panel; input order is irrelevant. Hidden panels remain
    mounted. Optional grips are passive views keyed by the preceding panel ID;
    the last visible panel has no divider. Paint styles never change hit geometry.
    Rust owns measured sizes and gestures; [on_resize] observes completed resizes
    asynchronously. Configuration changes fence queued observations; presentation
    changes preserve the handler, focus and live sizes. Bound both container axes. *)
val split_group
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?appearance:Split_group.Appearance.t
  -> ?handles:(Split_group.Id.t * 'action t) list
  -> ?on_resize:(Split_group.Snapshot.t -> 'action)
  -> config:Split_group.Config.t
  -> panels:(Split_group.Id.t * 'action t) list
  -> unit
  -> 'action t Core.Or_error.t

(** A native-owned divider between two retained children. Give the parent a
    bounded size. Pointer resizing stays in Rust; the optional callback reports
    completed resizes. Keyboard and accessibility actions share native limits.
    An immediate child with base [Display Hidden] (including an inactive [panel]
    or [tab_panel]) takes no space: the other child fills the split and the
    divider is absent. Hiding either child cancels an active resize without a
    completion event. Reopening preserves the previous native sizes, adjusted
    for the current container. Both hidden children produce an empty split.
    Child identities remain retained according to their own lifetime policy;
    [Visibility Hidden] and [Inert true] still occupy space. *)
val split_pane
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?on_resize:(Split_pane.Snapshot.t -> 'action)
  -> config:Split_pane.Config.t
  -> first:'action t
  -> second:'action t
  -> unit
  -> 'action t

(** Native tab-list roles, one keyboard focus stop, automatic selection with
    arrows/Home/End and pointer/accessibility activation. The caller owns the
    selected ID and panel lifetimes. Disabled tabs are skipped. *)
val tab_bar
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?appearance:Tab_bar.Appearance.t
  -> ?viewport:Tab_bar.Viewport.t
  -> ?motion:Tab_bar.Motion.t
  -> config:Choice.Config.t
  -> on_select:(Choice.Id.t -> 'action)
  -> unit
  -> 'action t

(** Retained panel with native tab-panel semantics. Inactive panels are hidden,
    preserving native editor/list state while remaining mounted. Bound the number
    of retained panels in the application; unmounting is explicit. *)
val tab_panel
  :  key:Key.t
  -> label:string
  -> active:bool
  -> ?style:Style.t
  -> 'action t list
  -> 'action t

(** A generic labelled region. [hidden] is an explicit native lifetime policy.
    Inactive regions remain absent from native input, regardless of supplied
    style. Opt-in [motion] may paint inert retained content while closing;
    otherwise inactive regions are absent from layout. See [Disclosure.Motion]. This does not deactivate a Bonsai computation that the caller
    continues evaluating. A label is nonempty UTF-8 without NUL, at most 4096
    bytes; invalid literal labels raise as with [Style.create_exn]. *)
val panel
  :  key:Key.t
  -> label:string
  -> active:bool
  -> hidden:Content_policy.t
  -> ?motion:Disclosure.Motion.t
  -> ?style:Style.t
  -> 'action t list
  -> 'action t

(** Present application-owned history in an assigned-size native viewport.
    Retain builds every keyed page; Unmount calls [content] only for the current
    entry and removes inactive descendants immediately. Hiding/removing a page
    does not cancel application Eio tasks or deactivate separately built Bonsai
    computations. At most one retained outgoing page paints, already inert.
    Removed pages never delay resource disposal for an exit animation.

    The 128-entry history bound does not bypass native resource quotas. Retained
    hidden editors still consume the ordinary editor admission budget. Use
    Unmount when inactive native resources should be released while application
    data remains available.

    Native timing and focus restoration follow [Navigation_stack.Motion]. The
    container clips pages to its assigned size; give it explicit dimensions or
    flex allocation. Label must be nonempty UTF-8 without NUL, at most 4096 bytes. *)
val navigation_stack
  :  'data Navigation_stack.t
  -> ?key:Key.t
  -> ?style:Style.t
  -> ?page_style:Style.t
  -> ?motion:Navigation_stack.Motion.t
  -> hidden:Content_policy.t
  -> label:string
  -> content:('data Navigation_stack.Entry.t -> 'action t list)
  -> unit
  -> 'action t

(** Measured track adapter under construction: full focus/accessibility
    qualification remains unfinished. All bounded items stay
    mounted and may have unequal extents. Reduce requests with
    [Carousel_track.apply_request], including Layout events; layout updates
    do not change the serialized model revision.

    The viewport and default controls handle Home/End and axis arrows; descendant
    card controls keep their keys. Pointer activation of a default control focuses
    the viewport; keyboard and assistive activation retain control focus.
    Automatic advancement waits for settled visible paint and pauses while the
    group is hovered or focused, inactive or under reduced motion.
    One proposal remains pending until model or measured geometry changes.
    Background dragging and precise trackpad input preview measured pixels; release
    proposes the nearest item. Line-wheel bursts propose one measured step. Nested
    native scrollers receive input first; no per-frame offsets enter OCaml.

    Fully clipped cards stay mounted but are excluded from focus, input and
    accessibility. Navigation reveals a card before its children can receive
    focus. A focused child moving fully out of view returns focus to the eligible
    viewport; external/modal focus is preserved. Anchored popovers suspend with
    their card and resume without repeating autofocus. Independent application
    modals remain open; structural hiding/removal keeps its existing semantics. *)
val carousel_track
  :  'data Carousel_track.t
  -> ?key:Key.t
  -> ?motion:Carousel_track.Motion.t
  -> ?style:Style.t
  -> ?viewport_style:Style.t
  -> ?track_style:Style.t
  -> ?item_style:('data Carousel_track.Item.t -> Style.t)
  -> ?controls_style:Style.t
  -> ?control_style:Style.t
  -> ?show_controls:bool
  -> label:string
  -> on_request:(Carousel_track.Request.t -> 'action)
  -> content:('data Carousel_track.Item.t -> 'action t list)
  -> unit
  -> 'action t

(** Application-owned carousel selection. The viewport reuses retained navigation
    pages; [hidden] controls native resources independently of Bonsai/Eio lifetimes.
    Give the owner or [viewport_style] an assigned height. Stable item IDs key pages.
    Default controls include first/previous/numbered/next/last navigation; disable
    them to supply application controls outside the component. Requests must be
    reduced against the latest model using [Carousel.apply_request].

    The focusable carousel surface and its ordinary controls handle Home/End and
    arrows on the selected axis; child editors and other native widgets keep their
    own keys. Optional auto-advance runs on native deadlines after settled visible
    paint, pausing for interaction, hidden/inactive windows and reduced motion.
    Pointer dragging previews the accepted and adjacent pages natively, emitting
    one request on release. Ignored requests snap back; accepted updates retarget
    from painted geometry. Child native controls keep input precedence. Wheel
    input respects the configured axis and groups momentum into bounded bursts.
    Full component accessibility acceptance remains under implementation. *)
val carousel
  :  'data Carousel.t
  -> ?key:Key.t
  -> ?style:Style.t
  -> ?viewport_style:Style.t
  -> ?page_style:Style.t
  -> ?controls_style:Style.t
  -> ?control_style:Style.t
  -> ?show_controls:bool
  -> ?axis:Carousel.Axis.t
  -> ?motion:Carousel.Motion.t
  -> hidden:Content_policy.t
  -> label:string
  -> on_request:(Carousel.Request.t -> 'action)
  -> content:('data Carousel.Item.t -> 'action t list)
  -> unit
  -> 'action t

(** Controlled collapsible content with a native button trigger and labelled
    region. Enter/Space, pointer and accessibility activation emit one intent;
    reduce it against current application state. Collapsing focused content
    restores its eligible trigger, respecting enclosing modal focus policy.
    Styles refine the outer, trigger and panel boxes independently. *)
val disclosure
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?trigger_style:Style.t
  -> ?panel_style:Style.t
  -> label:string
  -> expanded:bool
  -> ?disabled:bool
  -> hidden:Content_policy.t
  -> ?motion:Disclosure.Motion.t
  -> on_toggle:(unit -> 'action)
  -> 'action t list
  -> 'action t

(** Disclosure with independent header content and a dedicated toggle button.
    [trigger] must be a plain [button]; other kinds return an error. The helper
    assigns stable internal keys and places arbitrary header views beside it.
    Only the toggle receives expanded semantics and collapse-focus restoration;
    other header actions (for example a navigation link) remain independent. *)
val disclosure_with_header
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?header_style:Style.t
  -> ?panel_style:Style.t
  -> label:string
  -> expanded:bool
  -> hidden:Content_policy.t
  -> ?motion:Disclosure.Motion.t
  -> header:'action t list
  -> trigger:'action t
  -> 'action t list
  -> 'action t Core.Or_error.t

(** Ordered disclosures with stable item IDs. Up/Down/Home/End move among eligible
    headers; Enter/Space request a toggle. Content runs on the OCaml domain when
    building this description, never in native layout/paint. With [Unmount], it
    is not called for collapsed items; this alone does not deactivate a Bonsai
    computation evaluated outside that callback. Nested accordions have separate
    header navigation groups. *)
val accordion
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?trigger_style:Style.t
  -> ?panel_style:Style.t
  -> model:Disclosure.t
  -> hidden:Content_policy.t
  -> ?motion:Disclosure.Motion.t
  -> on_request:(Disclosure.Request.t -> 'action)
  -> content:(Choice.Id.t -> 'action t list)
  -> unit
  -> 'action t

(** Accordion with passive rich trigger labels keyed by item ID. Unknown and
    duplicate labels are rejected; omitted labels use the Choice text. Native
    triggers keep Choice labels as accessible names (rich names must satisfy the
    button's nonblank 1024-byte limit). Label content uses the rich-button rules,
    with an aggregate limit of 4096 nodes and depth 128. Interactive descendants
    are rejected. Content/requests and Retain/Unmount follow [accordion].

    [heading_level] defaults to 3 and must be 1..6. Each header is an accessible
    heading containing one full-width toggle, independent of panel content.
    [item_style] is a pure per-item outer style callback. Trigger hover/focus and
    panel styles use ordinary style states; title/icon styling belongs in labels. *)
val accordion_with_labels
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?item_style:(Choice.t -> Style.t)
  -> ?trigger_style:Style.t
  -> ?panel_style:Style.t
  -> ?heading_level:int
  -> model:Disclosure.t
  -> labels:(Choice.Id.t * 'action t) list
  -> hidden:Content_policy.t
  -> ?motion:Disclosure.Motion.t
  -> on_request:(Disclosure.Request.t -> 'action)
  -> content:(Choice.Id.t -> 'action t list)
  -> unit
  -> 'action t Core.Or_error.t

(** A native select with a bounded, scrollable option popup. Focus remains on
    the trigger. Arrows move the open popup highlight without changing the
    application value; Enter requests the highlighted ID, Escape cancels, and
    Tab closes before normal traversal. The label is shown when no value is
    selected. Printable text searches enabled labels without committing a value;
    repeated characters cycle, and the bounded prefix expires between inputs.
    Disabling or removing the control disposes its open popup. *)
val select
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?appearance:Choice.Appearance.t
  -> config:Choice.Config.t
  -> on_select:(Choice.Id.t -> 'action)
  -> unit
  -> 'action t

(** A grouped single/multiple picker with a native popup and retained query.
    The description owns committed selection; gestures emit ordered intents.
    Query changes use the native editor snapshot and never implicitly replace its
    draft. Passive slots reject callbacks, input and scrolling; only the footer
    is interactive. The complete wrapped slot forest is limited to 4096 nodes
    and 128 levels. Query controller keys are unique in their window.
    A clearable picker provides a separate keyboard-focusable Clear action.
    Enter/Space emits a clear intent without changing the query draft or popup.
    Open-popup Tab order is trigger, Clear, query, then footer controls. *)
val choice_picker
  :  ?key:Key.t
  -> ?style:Style.t
  -> on_event:(Choice_picker.Event.t -> 'action)
  -> 'action t Choice_picker.Description.t
  -> 'action t Core.Or_error.t

(** One native editor placement with choice selection intents. Initial text is
    used only on mount; choosing never implicitly replaces the native query. *)
val combobox
  :  ?style:Style.t
  -> ?appearance:Choice.Appearance.t
  -> ?initial_text:string
  -> controller:Key.t
  -> config:Combobox.Config.t
  -> on_event:(Combobox.Event.t -> 'action)
  -> unit
  -> 'action t Core.Or_error.t

(** A retained animation wrapper. Animation targets own the corresponding numeric
    style properties. Native frames do not recompute Bonsai. Endpoint callbacks use
    the latest closure; [run_id] identifies the completed/cancelled run. Replacing
    a running config can report its cancellation while the new run is current.
    Callbacks are discarded on unmount and never run after window disposal. *)
val animate
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?on_event:(Animation.Event.t -> 'action)
  -> Animation.Config.t
  -> 'action t list
  -> 'action t

(** Native assigned-size selection among retained, stable named presentations.
    All branches remain mounted in Bonsai. Hidden branches retain editing state,
    but do not paint or receive native input. The selected child's intrinsic size
    cannot size the outer query. Supply a meaningful parent/explicit size.
    [on_select] observes painted selection asynchronously; it never drives layout.
    Rejects duplicate, missing or extra presentation IDs before reconciliation. *)
val container_query
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?on_select:(Container_query.Selection.t -> 'action)
  -> Container_query.Config.t
  -> (Container_query.Branch_id.t * 'action t) list
  -> 'action t Core.Or_error.t

(** Retained springs/sequences/shared repeats. Animated targets own matching
    numeric style fields, except [Opacity_factor] which multiplies their resolved
    style opacity. Playback-only changes preserve run identity; new bodies
    retarget from painted values and a higher restart token resets initial values.
    Events arrive in ordered batches using the latest accepted closure. Hidden
    content pauses independent timing; shared members rejoin the group phase.
    Reduced motion and widget/window disposal are handled natively. *)
val animate_program
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?on_event:(Animation.Program.Event.t -> 'action)
  -> Animation.Program.t
  -> 'action t list
  -> 'action t

(** Encoded assets registered with [Gpuio_eio.Asset]. Layout comes from [style];
    [config] declares fit and accessibility. State changes are asynchronous and
    refer to the currently mounted source. Retired registrations keep existing
    mounts usable but cannot create new bindings. *)
val image
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?on_change:(Image.State.t -> 'action)
  -> Image.Config.t
  -> 'action t

(** SVG alpha mask tinted by the native foreground, with ordinary logical
    layout styles. Source bytes are registered once; size/tint updates stay native. *)
val icon
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?on_change:(Image.State.t -> 'action)
  -> Icon.Config.t
  -> 'action t

(** Native-owned single/range slider. [controller] identifies one mounted owner;
    [initial] seeds it only on mount. Use a new controller key to change between
    single and range mode. Observations do not reset the native drag or value.
    Disabled/read-only owners still report programmatic observations.
    [Style.Foreground] controls the selected rail, thumbs and native focus ring;
    the unselected rail uses the same color at 25% opacity. Focus state styles
    apply while either thumb is focused. Width/height set the available travel;
    background and border styles decorate the outer control. *)
val slider
  :  ?style:Style.t
  -> ?appearance:Slider.Appearance.t
  -> controller:Key.t
  -> config:Slider.Config.t
  -> initial:Slider.Value.t
  -> on_event:(Slider.Event.t -> 'action)
  -> unit
  -> 'action t

(** Native numeric editor placement. The stable controller identifies one native
    owner. [initial] seeds its committed value once. [initial_draft] optionally
    seeds independent text, including unfinished/invalid numeric expressions;
    omitted means formatted normalized value. Both are read only on creation.
    Selection/undo/IME start fresh. Observations never reset draft/selection.
    Explicit commands update live state. Configuration changes preserve the
    draft and normalize the committed value in the new domain. *)
val number_input
  :  ?style:Style.t
  -> controller:Key.t
  -> config:Number_input.Config.t
  -> initial:Number_input.Value.t
  -> ?initial_draft:Number_input.Draft.t
  -> on_event:(Number_input.Event.t -> 'action)
  -> unit
  -> 'action t

(** One native segmented OTP editor. [initial] seeds a mount once and must fit
    [config] policy. Changing a retained length/alphabet rejects; use a new
    controller identity to remount deliberately. Observations never replace text.
    Use [Gpuio_eio.Otp_input] for the mounted Bonsai command controller. The
    numeric-input capability covers this native editor and its command/event
    contract. *)
val otp_input
  :  ?style:Style.t
  -> ?appearance:Otp_input.Appearance.t
  -> controller:Key.t
  -> config:Otp_input.Config.t
  -> initial:Otp_input.Value.t
  -> on_event:(Otp_input.Event.t -> 'action)
  -> unit
  -> 'action t

(** Retained color-control description. Seeds once per controller identity;
    later configuration changes preserve the native value. Rust owns native
    channels, palette selection and editable drafts. Use [Gpuio_eio.Color_input]
    for correlated commands, or [Gpuio_eio.Color_picker] for a controlled popup. *)
val color_input
  :  ?style:Style.t
  -> ?appearance:Color_input.Appearance.t
  -> controller:Key.t
  -> config:Color_input.Config.t
  -> initial:Color_value.Value.t
  -> on_event:(Color_input.Event.t -> 'action)
  -> unit
  -> 'action t

module Calendar_content : sig
  type 'action view := 'action t

  module Item : sig
    type 'action t

    (** Replace a slot's visible content with checked passive views. Native
        name, selection, focus and activation remain owned by the calendar.
        [description] augments its accessible name; nonblank UTF-8 without
        ASCII controls, at most 1024 bytes. Interactive descendants and callbacks
        are rejected; the same style restrictions as rich button labels apply. *)
    val create
      :  ?description:string
      -> slot:Calendar.Slot.t
      -> 'action view
      -> 'action t Core.Or_error.t
  end

  type 'action t

  (** At most 1024 unique slots and 65536 description bytes; at most 4096 nodes
      and 128 levels across all content, including structural slot wrappers.
      Entries are sorted with their content to preserve stable slot identity.
      Unspecified slots retain their native labels. Empty collections are valid. *)
  val create : 'action Item.t list -> 'action t Core.Or_error.t
end

(** Retained native calendar. Seeds
    selection and displayed month once per controller identity. Mode is immutable;
    configuration changes may invalidate a historical selection without clearing
    it. [on_viewport_change] opts into asynchronous logical pane observations:
    one on subscription, then changes, with adjacent queued changes coalesced.
    Ordinary callback updates retain the subscription; removing/readding it
    retires queued events. Selection revisions remain independent. See
    [Calendar.Viewport] for bounds, hidden-owner and async-result semantics. *)
val calendar
  :  ?style:Style.t
  -> ?appearance:Calendar.Appearance.t
  -> ?content:'action Calendar_content.t
  -> ?on_viewport_change:(Calendar.Viewport.t -> 'action)
  -> controller:Key.t
  -> config:Calendar.Config.t
  -> initial:Calendar.Selection.t
  -> initial_month:Calendar.Month.t
  -> on_event:(Calendar.Event.t -> 'action)
  -> unit
  -> 'action t

(** Controlled integer rating. Apply each request against the latest application
    state with [Rating.Config.apply_request]. Hover stays native; keyboard and
    accessibility report ordered requests without a second committed model. *)
val rating
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?appearance:Rating.Appearance.t
  -> config:Rating.Config.t
  -> on_request:(Rating.Request.t -> 'action)
  -> unit
  -> 'action t

(** Stable native avatar with image/fallback selection. Image observations use the
    existing source-generation fences; no asset means no observation. *)
val avatar
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?on_change:(Image.State.t -> 'action)
  -> Avatar.Config.t
  -> 'action t

(** Native fallback slot for passive rich content. The bounded subtree stays
    mounted while the primary image paints; its native motion is suspended while
    hidden. Removing this slot or its avatar retires the subtree normally.

    The fallback is centered in the assigned avatar rectangle and cannot size
    the avatar itself. Overflow clipping is rectangular, as in GPUI containers;
    root corner styles round the background and primary image, not arbitrary
    descendants. Style full-bleed child images/backgrounds with their own corners.
    Visible content is decorative: the avatar config owns its sole semantic name.

    At most 4096 nodes and 128 levels are admitted. Layout, text, images/icons,
    avatars, loading and animation are allowed without callbacks, selectable
    text, scrolling, controls or pointer shields. Invalid content returns Error.
    Source failures and loading select the slot natively, without an OCaml render
    callback. GPU upload failure can select it on a subsequent notified frame. *)
val avatar_with_fallback
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?on_change:(Image.State.t -> 'action)
  -> Avatar.Config.t
  -> fallback:'action t
  -> 'action t Core.Or_error.t

(** Native indeterminate spinner with optional decorative SVG and queued icon
    state observations. There are no per-frame OCaml callbacks. Reusing a key
    across [spinner] and [loading] retains the node while replacing its state. *)
val spinner
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?on_icon_change:(Image.State.t -> 'action)
  -> config:Spinner.Config.t
  -> unit
  -> 'action t

(** Noninteractive skeleton/shimmer/spinner. Styles set size, color and placeholder
    corners; motion and hidden/reduced/static behavior remain native. No events or
    progress value are produced. See [Loading.Config]. *)
val loading : ?key:Key.t -> ?style:Style.t -> config:Loading.Config.t -> unit -> 'action t

(** A noninteractive native progress bar. Root Background styles the track and
    Foreground styles the indicator. Indeterminate motion stays on the native side;
    updates retain node identity. Value changes are immediate unless [transition]
    is supplied. The accessible value is a percentage or absent
    when indeterminate. Width/height use ordinary logical-pixel styles. *)
val progress
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?transition:Progress.Transition.t
  -> config:Progress.Config.t
  -> unit
  -> 'action t

(** Circular progress with keyed center content. Defaults to 32px and a 200ms
    ease-out transition. The semantic value reports the target immediately;
    inert/reduced motion snaps artwork. Center controls retain their own state. *)
val progress_circle
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?transition:Progress.Transition.t
  -> config:Progress.Config.t
  -> 'action t list
  -> 'action t

(** A keyed notification session, usable only as a toast-stack item. *)
type 'action toast

(** Native drag source/drop target regions with shared style and children.
    Callbacks observe native decisions; provide keyboard command alternatives. *)
val drag_source
  :  ?key:Key.t
  -> ?style:Style.t
  -> config:Drag_and_drop.Source.t
  -> on_event:(Drag_and_drop.Source_event.t -> 'action)
  -> 'action t list
  -> 'action t

val drop_target
  :  ?key:Key.t
  -> ?style:Style.t
  -> config:Drag_and_drop.Target.t
  -> on_event:(Drag_and_drop.Target_event.t -> 'action)
  -> 'action t list
  -> 'action t

(** Observe native bindings asynchronously. This ordinary container introduces
    no focus stop. Replacing config retires the old handler/epoch; changing only
    the callback preserves the subscription and uses the current callback.
    The supplied editor context must belong to the enclosing window. *)
val command_binding_scope
  :  ?key:Key.t
  -> ?style:Style.t
  -> config:Command_binding.Config.t
  -> on_update:(Command_binding.Observation.t -> 'action)
  -> 'action t list
  -> 'action t

(** Retained highlight declaration. Empty config overrides an ancestor. Children
    retain their identities; updates are asynchronous. Native painting integration
    is in development and no highlight capability is advertised yet. *)
val highlight_scope
  :  ?key:Key.t
  -> ?style:Style.t
  -> config:Highlight.Config.t
  -> ?on_update:(Highlight.Observation.t -> 'action)
  -> 'action t list
  -> 'action t

(** General opt-in input observations. The region owns its focus and observation
    binding; child native widgets retain their state. Policies execute in Rust and
    callbacks run asynchronously. Changing config retires queued old observations;
    changing only the callback uses the latest accepted closure. *)
val input_region
  :  ?key:Key.t
  -> ?style:Style.t
  -> config:Input_region.Config.t
  -> on_event:(Input_region.Event.t -> 'action)
  -> 'action t list
  -> 'action t

(** Native captured mouse gestures with ordinary content. Root styles do not
    replace the captured node; hiding/unmounting releases native capture. *)
val pointer_area
  :  ?key:Key.t
  -> ?style:Style.t
  -> config:Pointer.Config.t
  -> on_event:(Pointer.Event.t -> 'action)
  -> 'action t list
  -> 'action t

(** Native dismissal closes the session once; the callback should remove it.
    Content changes preserve elapsed time; changing timeout resets its interval.
    Hidden items pause. A closed item only reopens with a new key or remount. *)
val toast
  :  key:Key.t
  -> ?style:Style.t
  -> config:Toast.Config.t
  -> on_dismiss:(Toast.Dismissal.t -> 'action)
  -> 'action t list
  -> 'action toast

(** Newest items occupy the visible stack; older excess receives Overflow.
    Rejects duplicate keys or more than 32 submitted items. *)
val toast_stack
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?config:Toast.Stack.t
  -> 'action toast list
  -> 'action t Core.Or_error.t

module Expert : sig
  type 'action table =
    { source_key : Key.t option
    ; config : Table.Config.t
    ; query_generation : int64
    ; commands : Key.t Table.Command.t list
    ; on_input : Key.t Table.Request.t -> 'action
    ; on_column_viewport : (Table.Column_viewport.t -> 'action) option
    }

  type 'action virtual_list =
    { config : Virtual_list.Config.t
    ; order : Virtual_list.Order.t
    ; managed : bool
    ; invalidated : Key.t list
    ; invalidation_revision : int64
    ; scroll : Virtual_list.Scroll_request.t option
    ; on_viewport : (Virtual_list.Viewport.t -> 'action) option
    ; on_retain : (Key.t list -> 'action) option
    ; on_tree_input : (Key.t Tree_input.t -> 'action) option
    ; list_input : (List_input.Config.t * (Key.t List_input.t -> 'action)) option
    ; tree_moves : bool
    ; table : 'action table option
    }

  (** Native list adapters supply the desired row set, including pinned rows.
      [on_retain] handles a native veto of stale viewport-driven eviction.
      [on_tree_input] opts into tree keyboard/pointer/accessibility requests and
      requires Tree semantics on the final list root. Reconciliation rotates the
      handler when input or [tree_moves] changes, retiring already queued requests.
      [tree_moves] defaults to false and requires [on_tree_input]; Move delivers
      two current keys as a proposal, never an implicit hierarchy mutation. *)
  val managed_virtual_list
    :  ?key:Key.t
    -> ?style:Style.t
    -> ?scroll:Virtual_list.Scroll_request.t
    -> ?invalidated:Key.t list
    -> ?invalidation_revision:int64
    -> config:Virtual_list.Config.t
    -> order:Virtual_list.Order.t
    -> on_viewport:(Virtual_list.Viewport.t -> 'action)
    -> on_retain:(Key.t list -> 'action)
    -> ?on_tree_input:(Key.t Tree_input.t -> 'action)
    -> ?tree_moves:bool
    -> (Key.t * 'action t) list
    -> 'action t Core.Or_error.t

  (** Native retained table adapter. Rows contain exactly one cell per schema
      column in schema order; wrappers are generated with stable column keys.
      Only [config]'s bounded active rows are accepted. No arbitrary row wrapper,
      native handle or synchronous renderer crosses this interface.

      Query generations must not decrease during a mount. A query reset retires
      callbacks/viewport observations while preserving surviving row IDs. The
      reconciler generates schema revisions from accepted column/sort changes.
      Commands are ordered batches of at most 64 items; a retained equal batch
      executes once, new serials strictly increase across the mount (also across
      omitted batches and query resets). Delayed obsolete targets are errors;
      higher-level controllers should filter them before constructing a View.
      Changing [source_key] replaces the entire native mount independently of
      the sibling [key]; keep it stable for point updates and query changes.

      [on_column_viewport] observes horizontal column bands after native layout
      and paint, including pins and empty-data headers. Delivery requires the
      current revision/schema/query. It is a latest-layout snapshot, not an OS
      occlusion query; hidden/unmounted views need not emit a final empty value.
      Horizontal observation does not change cell allocation limits.

      [headers] supplies up to 320 keyed leaf/group header Views, separate from
      managed body rows and their active-cell budget. Keys retain content owners
      independently of column position or target; changing target membership or
      removing a slot advances the schema revision. Native geometry, resizing,
      sorting and selection remain owned by the table. Ordinary View/node/editor
      quotas apply to header content. Invalid or duplicate targets fail before
      reconciliation.

      [header_presentation] and [row_presentations] style native containers within
      the checked [Table_presentation] scope. Row keys must be unique and belong
      to submitted active rows. Absent/empty values restore defaults. Theme/paint
      updates preserve schema revisions, native owners and scroll geometry;
      typography inherits into content unless overridden by child Views. *)
  val managed_table
    :  ?key:Key.t
    -> ?source_key:Key.t
    -> ?style:Style.t
    -> ?headers:'action t Table_header.t list
    -> ?header_presentation:Table_presentation.Header.t
    -> ?row_presentations:(Key.t * Table_presentation.Row.t) list
    -> ?commands:Key.t Table.Command.t list
    -> ?on_column_viewport:(Table.Column_viewport.t -> 'action)
    -> config:Table.Config.t
    -> query_generation:int64
    -> order:Virtual_list.Order.t
    -> on_viewport:(Virtual_list.Viewport.t -> 'action)
    -> on_retain:(Key.t list -> 'action)
    -> on_input:(Key.t Table.Request.t -> 'action)
    -> (Key.t * (Table.Cell.t * 'action t) list) list
    -> 'action t Core.Or_error.t

  (** Compact plain-text cell for [managed_table]. Display and copy text are
      identical. This leaf is only valid as a cell in that table's row/schema;
      ordinary rich cell content uses the existing wrapper representation. *)
  val table_text : Table.Cell.t -> 'action t

  type 'action container_query =
    { config : Container_query.Config.t
    ; on_select : (Container_query.Selection.t -> 'action) option
    }

  type 'action animation_program =
    { config : Animation.Program.t
    ; on_event : (Animation.Program.Event.t -> 'action) option
    }

  type 'action animation =
    { config : Animation.Config.t
    ; on_event : (Animation.Event.t -> 'action) option
    }

  type 'action extension =
    { config : Gpuio_protocol.Extension_wire.Config.t
    ; on_event : Gpuio_protocol.Extension_wire.Signal.t -> 'action
    }

  type 'action split_group =
    { config : Split_group.Config.t
    ; appearance : Split_group.Appearance.t
    ; on_resize : (Split_group.Snapshot.t -> 'action) option
    }

  type 'action split_pane =
    { config : Split_pane.Config.t
    ; on_resize : (Split_pane.Snapshot.t -> 'action) option
    }

  type 'action canvas =
    { config : Canvas.Config.t
    ; on_event : (Canvas.Event.t -> 'action) option
    }

  type 'action chart =
    { config : Chart.Config.t
    ; on_event : (Chart.Event.t -> 'action) option
    }

  type 'action document =
    { config : Document.Config.t
    ; on_navigate : (Document.Navigation.t -> 'action) option
    ; on_diff : (Document.Diff.Event.t -> 'action) option
    ; on_preview : (Document.Preview.Event.t -> 'action) option
    ; on_action : (Document.Actions.Event.t -> 'action) option
    ; inherit_profile : bool
    ; profile :
        (Gpuio_protocol.Document_profile_wire.Instance.t
        * (Gpuio_protocol.Document_profile_wire.Event.t -> 'action option))
          option
    }

  type 'action slider =
    { controller : Key.t
    ; config : Slider.Config.t
    ; appearance : Slider.Appearance.t option
    ; initial : Slider.Value.t
    ; on_event : Slider.Event.t -> 'action
    }

  type 'action number_input =
    { controller : Key.t
    ; config : Number_input.Config.t
    ; appearance : Number_input.Appearance.t option
    ; initial : Number_input.Value.t
    ; initial_draft : Number_input.Draft.t option
    ; on_event : Number_input.Event.t -> 'action
    }

  type 'action otp_input =
    { controller : Key.t
    ; appearance : Otp_input.Appearance.t option
    ; config : Otp_input.Config.t
    ; initial : Otp_input.Value.t
    ; on_event : Otp_input.Event.t -> 'action
    }

  type 'action color_input =
    { controller : Key.t
    ; config : Color_input.Config.t
    ; appearance : Color_input.Appearance.t option
    ; initial : Color_value.Value.t
    ; on_event : Color_input.Event.t -> 'action
    }

  type 'action calendar =
    { controller : Key.t
    ; config : Calendar.Config.t
    ; initial : Calendar.Selection.t
    ; initial_month : Calendar.Month.t
    ; appearance : Calendar.Appearance.t option
    ; content : Gpuio_protocol.Calendar_content_wire.t option
    ; on_event : Calendar.Event.t -> 'action
    ; on_viewport_change : (Calendar.Viewport.t -> 'action) option
    }

  type 'action rating =
    { config : Rating.Config.t
    ; appearance : Rating.Appearance.t option
    ; on_request : Rating.Request.t -> 'action
    }

  type 'action image =
    { config : Image.Config.t
    ; on_change : (Image.State.t -> 'action) option
    }

  type 'action drag_source =
    { config : Drag_and_drop.Source.t
    ; on_event : Drag_and_drop.Source_event.t -> 'action
    }

  type 'action drop_target =
    { config : Drag_and_drop.Target.t
    ; on_event : Drag_and_drop.Target_event.t -> 'action
    }

  type 'action command_binding_scope =
    { config : Command_binding.Config.t
    ; on_update : Command_binding.Observation.t -> 'action
    }

  type 'action highlight_scope =
    { config : Highlight.Config.t
    ; on_update : (Highlight.Observation.t -> 'action) option
    }

  type 'action input_region =
    { config : Input_region.Config.t
    ; on_event : Input_region.Event.t -> 'action
    }

  type 'action pointer =
    { config : Pointer.Config.t
    ; on_event : Pointer.Event.t -> 'action
    }

  type 'action notification =
    { config : Toast.Config.t
    ; on_dismiss : Toast.Dismissal.t -> 'action
    }

  module Kind : sig
    type t =
      | Container
      | Text
      | Button
      | Input
      | Textarea
      | Checkbox
      | Switch
      | Radio_group
      | Select
      | Combobox
      | Choice_picker
      | Focus_scope
      | Tooltip
      | Command_scope
      | Command_button
      | Menu
      | Command_palette
      | Progress
      | Toast
      | Toast_stack
      | Pointer_area
      | Drag_source
      | Drop_target
      | Image
      | Icon
      | Animated
      | Virtual_list
      | Document_view
      | Tab_bar
      | Tab_panel
      | Split_pane
      | Split_group
      | Extension
      | Canvas_view
      | Animation_program
      | Container_query
      | Loading
      | Avatar
      | Rating
      | Slider
      | Number_input
      | Otp_input
      | Calendar
      | Color_input
      | Panel
      | Disclosure
      | Accordion
      | Navigation_stack
      | Hover_card
      | Carousel
      | Carousel_track
      | Carousel_track_group
      | Chart_view
      | Input_region
      | Highlight_scope
      | Link
      | Radio
    [@@deriving equal, sexp_of]
  end

  module Control : sig
    type t =
      | Button of { disabled : bool }
      | Checkbox of
          { state : Check_state.t
          ; disabled : bool
          }
      | Switch of
          { checked : bool
          ; disabled : bool
          }
      | Radio of
          { checked : bool
          ; disabled : bool
          ; position : Radio.Position.t option
          }
    [@@deriving equal, sexp_of]

    val to_wire : t -> Gpuio_protocol.Wire.Control.t
  end

  type 'action overlay =
    { kind : Gpuio_protocol.Wire.Overlay_kind.t
    ; config : Overlay.Config.t
    ; backdrop : Color.t option
    ; motion : Overlay.Motion.t
    ; sheet_insets : Sheet.Insets.t option
    ; on_dismiss : Overlay.Dismissal.t -> 'action
    }

  type 'action tooltip =
    { config : Tooltip.Config.t
    ; on_open_change : (bool -> 'action) option
    }

  type 'action combobox =
    { controller : Key.t
    ; config : Combobox.Config.t
    ; appearance : Choice.Appearance.t
    ; on_event : Combobox.Event.t -> 'action
    }

  type 'action choice =
    { config : Choice.Config.t
    ; appearance : Choice.Appearance.t option
    ; tab_appearance : Tab_bar.Appearance.t option
    ; tab_content : Gpuio_protocol.Wire.Tab_content.t option
    ; tab_viewport : Tab_bar.Viewport.t option
    ; tab_motion : Tab_bar.Motion.t option
    ; tab_trailing : bool
    ; choice_menu : bool
    ; on_select : Choice.Id.t -> 'action
    }

  type 'action editor_callback =
    | Editor_events of (Text_input.Event.t -> 'action)
    | Picker_query

  type 'action editor =
    { controller : Key.t
    ; config : Text_input.Config.t
    ; frame : Input_frame.t option
    ; on_event : 'action editor_callback
    }

  type 'action palette =
    { config : Command_palette.Config.t
    ; appearance : Command_palette.Appearance.t
    ; on_change : (Command_palette.Snapshot.t -> 'action) option
    ; on_dismiss : Command_palette.Dismissal.t -> 'action
    }

  type 'action menu =
    { presentation : Menu.Expert.presentation
    ; menus : Menu.t list
    ; appearance : Menu.Appearance.t
    ; placement : Placement.t option
    ; on_open_change : (bool -> 'action) option
    }

  type 'action description =
    { key : Key.t option
    ; structural_key : (string * string) option
    ; kind : Kind.t
    ; text : string
    ; text_content : Text_content.t option
    ; text_shimmer : Text_shimmer.Config.t option
    ; scrollbar : Scrollbar.t option
    ; window_region : Window_region.t option
    ; link : Link.Config.t option
    ; style : Style.t
    ; on_click : (unit -> 'action) option
    ; on_hover : (bool -> 'action) option
    ; editor : 'action editor option
    ; control : Control.t option
    ; split_button :
        (Split_button.Appearance.t * Gpuio_protocol.Wire.Split_button.Parts.t) option
    ; button_presentation : Button.Expert.Presentation.t option
    ; tab_order : Tab_order.t option
    ; control_appearance : Control_appearance.t option
    ; choice : 'action choice option
    ; combobox : 'action combobox option
    ; choice_picker :
        ('action t Choice_picker.Description.t * (Choice_picker.Event.t -> 'action))
          option
    ; popover : bool
    ; overlay : 'action overlay option
    ; tooltip : 'action tooltip option
    ; commands : 'action Command.Registry.t option
    ; command_ref : Command.Id.t option
    ; drag_source : 'action drag_source option
    ; drop_target : 'action drop_target option
    ; pointer : 'action pointer option
    ; input_region : 'action input_region option
    ; highlight_scope : 'action highlight_scope option
    ; command_binding_scope : 'action command_binding_scope option
    ; notification : 'action notification option
    ; toast_stack : Toast.Stack.t option
    ; progress : Progress.Config.t option
    ; progress_presentation : Gpuio_protocol.Progress_wire.Presentation.t option
    ; loading : Loading.Config.t option
    ; spinner : Spinner.Config.t option
    ; avatar : Avatar.Config.t option
    ; rating : 'action rating option
    ; slider : 'action slider option
    ; number_input : 'action number_input option
    ; otp_input : 'action otp_input option
    ; color_input : 'action color_input option
    ; calendar : 'action calendar option
    ; animation : 'action animation option
    ; animation_program : 'action animation_program option
    ; reveal : Gpuio_protocol.Reveal_wire.t option
    ; navigation_stack : Gpuio_protocol.Navigation_stack_wire.Config.t option
    ; carousel :
        (Gpuio_protocol.Carousel_wire.Config.t * (Carousel.Request.t -> 'action)) option
    ; carousel_track_motion : Gpuio_protocol.Carousel_track_wire.Motion.t option
    ; carousel_track :
        (Gpuio_protocol.Carousel_track_wire.Config.t
        * (Carousel_track.Request.t -> 'action))
          option
    ; container_query : 'action container_query option
    ; accessibility : Accessibility.t option
    ; image : 'action image option
    ; extension : 'action extension option
    ; split_pane : 'action split_pane option
    ; split_group : 'action split_group option
    ; document : 'action document option
    ; canvas : 'action canvas option
    ; chart : 'action chart option
    ; palette : 'action palette option
    ; menu : 'action menu option
    ; focus_scope : Focus_scope.t option
    ; virtual_list : 'action virtual_list option
    ; table_header : Table_header.Target.t option
    ; table_header_style : Table_presentation.Header.t option
    ; table_row_style : Table_presentation.Row.t option
    ; table_cell : Table.Cell.t option
    ; children : 'action t list
    }

  val describe : 'action t -> 'action description
end
