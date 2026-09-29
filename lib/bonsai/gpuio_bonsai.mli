open Core
module Managed_rows = Managed_rows
module Virtual_list = Virtual_list
module Tree_rows = Tree_rows
module Tree = Tree
module Table = Table

(** The pure view API specialized to Bonsai effects. No driver, I/O runtime or
    scheduling policy is introduced here; window lifecycle scheduling is OCH-9. *)
module View : sig
  type t = unit Bonsai.Effect.t Gpuio.View.t
  type toast = unit Bonsai.Effect.t Gpuio.View.toast

  val with_accessibility : t -> Gpuio.Accessibility.t -> t Or_error.t

  val container_query
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> ?on_select:(Gpuio.Container_query.Selection.t -> unit Bonsai.Effect.t)
    -> Gpuio.Container_query.Config.t
    -> (Gpuio.Container_query.Branch_id.t * t) list
    -> t Or_error.t

  (** Native springs, ordered sequences and synchronized repeating programs. *)
  val animate_program
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> ?on_event:(Gpuio.Animation.Program.Event.t -> unit Bonsai.Effect.t)
    -> Gpuio.Animation.Program.t
    -> t list
    -> t

  (** Retained native motion; endpoint callbacks execute as Bonsai effects. *)
  val animate
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> ?on_event:(Gpuio.Animation.Event.t -> unit Bonsai.Effect.t)
    -> Gpuio.Animation.Config.t
    -> t list
    -> t

  val chart
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> ?on_event:(Gpuio.Chart.Event.t -> unit Bonsai.Effect.t)
    -> Gpuio.Chart.Config.t
    -> t

  val canvas
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> ?on_event:(Gpuio.Canvas.Event.t -> unit Bonsai.Effect.t)
    -> Gpuio.Canvas.Config.t
    -> t

  (** Revisioned native documents; register source with [Gpuio_eio.Document]. *)
  val document
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> ?on_navigate:(Gpuio.Document.Navigation.t -> unit Bonsai.Effect.t)
    -> ?on_diff:(Gpuio.Document.Diff.Event.t -> unit Bonsai.Effect.t)
    -> Gpuio.Document.Config.t
    -> t

  (** Pure image placement; register encoded bytes with [Gpuio_eio.Asset]. *)
  val image
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> ?on_change:(Gpuio.Image.State.t -> unit Bonsai.Effect.t)
    -> Gpuio.Image.Config.t
    -> t

  val icon
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> ?on_change:(Gpuio.Image.State.t -> unit Bonsai.Effect.t)
    -> Gpuio.Icon.Config.t
    -> t

  val drag_source
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> config:Gpuio.Drag_and_drop.Source.t
    -> on_event:(Gpuio.Drag_and_drop.Source_event.t -> unit Bonsai.Effect.t)
    -> t list
    -> t

  val drop_target
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> config:Gpuio.Drag_and_drop.Target.t
    -> on_event:(Gpuio.Drag_and_drop.Target_event.t -> unit Bonsai.Effect.t)
    -> t list
    -> t

  val highlight_scope
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> config:Gpuio.Highlight.Config.t
    -> ?on_update:(Gpuio.Highlight.Observation.t -> unit Bonsai.Effect.t)
    -> t list
    -> t

  val input_region
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> config:Gpuio.Input_region.Config.t
    -> on_event:(Gpuio.Input_region.Event.t -> unit Bonsai.Effect.t)
    -> t list
    -> t

  val pointer_area
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> config:Gpuio.Pointer.Config.t
    -> on_event:(Gpuio.Pointer.Event.t -> unit Bonsai.Effect.t)
    -> t list
    -> t

  val toast
    :  key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> config:Gpuio.Toast.Config.t
    -> on_dismiss:(Gpuio.Toast.Dismissal.t -> unit Bonsai.Effect.t)
    -> t list
    -> toast

  val toast_stack
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> ?config:Gpuio.Toast.Stack.t
    -> toast list
    -> t Or_error.t

  val text : ?key:Gpuio.Key.t -> ?style:Gpuio.Style.t -> string -> t

  val button
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> ?accessible_name:string
    -> ?disabled:bool
    -> ?leading_icon:Gpuio.Icon.Decoration.t
    -> ?trailing_icon:Gpuio.Icon.Decoration.t
    -> on_click:unit Bonsai.Effect.t
    -> string
    -> t

  val icon_button
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> ?disabled:bool
    -> label:string
    -> on_click:unit Bonsai.Effect.t
    -> Gpuio.Icon.Decoration.t
    -> t

  val checkbox
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> ?accessible_name:string
    -> ?disabled:bool
    -> state:Gpuio.Check_state.t
    -> on_toggle:unit Bonsai.Effect.t
    -> string
    -> t

  val switch
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> ?accessible_name:string
    -> ?disabled:bool
    -> checked:bool
    -> on_toggle:unit Bonsai.Effect.t
    -> string
    -> t

  val slider
    :  ?style:Gpuio.Style.t
    -> controller:Gpuio.Key.t
    -> config:Gpuio.Slider.Config.t
    -> initial:Gpuio.Slider.Value.t
    -> on_event:(Gpuio.Slider.Event.t -> unit Bonsai.Effect.t)
    -> unit
    -> t

  val rating
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> config:Gpuio.Rating.Config.t
    -> on_request:(Gpuio.Rating.Request.t -> unit Bonsai.Effect.t)
    -> unit
    -> t

  val avatar
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> ?on_change:(Gpuio.Image.State.t -> unit Bonsai.Effect.t)
    -> Gpuio.Avatar.Config.t
    -> t

  val loading
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> config:Gpuio.Loading.Config.t
    -> unit
    -> t

  val progress
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> config:Gpuio.Progress.Config.t
    -> unit
    -> t

  val command_palette
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> ?appearance:Gpuio.Command_palette.Appearance.t
    -> config:Gpuio.Command_palette.Config.t
    -> on_dismiss:(Gpuio.Command_palette.Dismissal.t -> unit Bonsai.Effect.t)
    -> unit
    -> t

  val menu_button
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> ?appearance:Gpuio.Menu.Appearance.t
    -> menu:Gpuio.Menu.t
    -> unit
    -> t

  val context_menu
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> ?appearance:Gpuio.Menu.Appearance.t
    -> menu:Gpuio.Menu.t
    -> t
    -> t

  val menu_bar
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> ?appearance:Gpuio.Menu.Appearance.t
    -> ?platform:bool
    -> Gpuio.Menu.t list
    -> t Core.Or_error.t

  val command_scope
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> commands:unit Bonsai.Effect.t Gpuio.Command.Registry.t
    -> t list
    -> t

  val command_button
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> ?leading_icon:Gpuio.Icon.Decoration.t
    -> ?trailing_icon:Gpuio.Icon.Decoration.t
    -> command:Gpuio.Command.Id.t
    -> unit
    -> t

  val focus_scope
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> config:Gpuio.Focus_scope.t
    -> t list
    -> t

  val dialog
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> config:Gpuio.Overlay.Config.t
    -> on_dismiss:(Gpuio.Overlay.Dismissal.t -> unit Bonsai.Effect.t)
    -> t option
    -> t

  val sheet
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> config:Gpuio.Sheet.Config.t
    -> on_dismiss:(Gpuio.Overlay.Dismissal.t -> unit Bonsai.Effect.t)
    -> t option
    -> t

  val alert_dialog
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> config:Gpuio.Alert_dialog.Config.t
    -> on_dismiss:(Gpuio.Overlay.Dismissal.t -> unit Bonsai.Effect.t)
    -> t option
    -> t

  val popover
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> config:Gpuio.Overlay.Config.t
    -> on_dismiss:(Gpuio.Overlay.Dismissal.t -> unit Bonsai.Effect.t)
    -> anchor:t
    -> t option
    -> t

  val tooltip
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> config:Gpuio.Tooltip.Config.t
    -> ?on_open_change:(bool -> unit Bonsai.Effect.t)
    -> anchor:t
    -> content:t
    -> unit
    -> t

  val hover_card
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> config:Gpuio.Hover_card.Config.t
    -> ?on_open_change:(bool -> unit Bonsai.Effect.t)
    -> anchor:t
    -> content:t
    -> unit
    -> t

  val row : ?key:Gpuio.Key.t -> ?style:Gpuio.Style.t -> t list -> t

  val extension
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> on_event:('event Gpuio.Extension.Event.t -> unit Bonsai.Effect.t)
    -> 'event Gpuio.Extension.Instance.t
    -> t

  val split_pane
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> ?on_resize:(Gpuio.Split_pane.Snapshot.t -> unit Bonsai.Effect.t)
    -> config:Gpuio.Split_pane.Config.t
    -> first:t
    -> second:t
    -> unit
    -> t

  val tab_bar
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> config:Gpuio.Choice.Config.t
    -> on_select:(Gpuio.Choice.Id.t -> unit Bonsai.Effect.t)
    -> unit
    -> t

  val tab_panel
    :  key:Gpuio.Key.t
    -> label:string
    -> active:bool
    -> ?style:Gpuio.Style.t
    -> t list
    -> t

  (** Generic region and disclosure lifetimes follow [Gpuio.View.panel] and
      [Gpuio.View.disclosure]; hiding native content does not itself deactivate
      its Bonsai computation or cancel application tasks. *)
  val panel
    :  key:Gpuio.Key.t
    -> label:string
    -> active:bool
    -> hidden:Gpuio.Content_policy.t
    -> ?style:Gpuio.Style.t
    -> t list
    -> t

  (** See [Gpuio.View.carousel] for selection and native lifetime contracts. *)
  val carousel
    :  'data Gpuio.Carousel.t
    -> ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> ?viewport_style:Gpuio.Style.t
    -> ?page_style:Gpuio.Style.t
    -> ?controls_style:Gpuio.Style.t
    -> ?control_style:Gpuio.Style.t
    -> ?show_controls:bool
    -> ?axis:Gpuio.Carousel.Axis.t
    -> ?motion:Gpuio.Carousel.Motion.t
    -> hidden:Gpuio.Content_policy.t
    -> label:string
    -> on_request:(Gpuio.Carousel.Request.t -> unit Bonsai.Effect.t)
    -> content:('data Gpuio.Carousel.Item.t -> t list)
    -> unit
    -> t

  (** Native route presentation; see [Gpuio.View.navigation_stack] for lifetime,
      geometry and focus rules. Content may contain ordinary Bonsai effects. *)
  val navigation_stack
    :  'data Gpuio.Navigation_stack.t
    -> ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> ?page_style:Gpuio.Style.t
    -> ?motion:Gpuio.Navigation_stack.Motion.t
    -> hidden:Gpuio.Content_policy.t
    -> label:string
    -> content:('data Gpuio.Navigation_stack.Entry.t -> t list)
    -> unit
    -> t

  val disclosure
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> ?trigger_style:Gpuio.Style.t
    -> ?panel_style:Gpuio.Style.t
    -> label:string
    -> expanded:bool
    -> ?disabled:bool
    -> hidden:Gpuio.Content_policy.t
    -> on_toggle:unit Bonsai.Effect.t
    -> t list
    -> t

  val disclosure_with_header
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> ?header_style:Gpuio.Style.t
    -> ?panel_style:Gpuio.Style.t
    -> label:string
    -> expanded:bool
    -> hidden:Gpuio.Content_policy.t
    -> header:t list
    -> trigger:t
    -> t list
    -> t Or_error.t

  val accordion
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> ?trigger_style:Gpuio.Style.t
    -> ?panel_style:Gpuio.Style.t
    -> model:Gpuio.Disclosure.t
    -> hidden:Gpuio.Content_policy.t
    -> on_request:(Gpuio.Disclosure.Request.t -> unit Bonsai.Effect.t)
    -> content:(Gpuio.Choice.Id.t -> t list)
    -> unit
    -> t

  val radio_group
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> config:Gpuio.Choice.Config.t
    -> on_select:(Gpuio.Choice.Id.t -> unit Bonsai.Effect.t)
    -> unit
    -> t

  val select
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> ?appearance:Gpuio.Choice.Appearance.t
    -> config:Gpuio.Choice.Config.t
    -> on_select:(Gpuio.Choice.Id.t -> unit Bonsai.Effect.t)
    -> unit
    -> t

  val combobox
    :  ?style:Gpuio.Style.t
    -> ?appearance:Gpuio.Choice.Appearance.t
    -> ?initial_text:string
    -> controller:Gpuio.Key.t
    -> config:Gpuio.Combobox.Config.t
    -> on_event:(Gpuio.Combobox.Event.t -> unit Bonsai.Effect.t)
    -> unit
    -> t Or_error.t

  val virtual_list
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> ?on_viewport:(Gpuio.Virtual_list.Viewport.t -> unit Bonsai.Effect.t)
    -> ?scroll:Gpuio.Virtual_list.Scroll_request.t
    -> config:Gpuio.Virtual_list.Config.t
    -> (Gpuio.Key.t * t) list
    -> t Or_error.t

  val column : ?key:Gpuio.Key.t -> ?style:Gpuio.Style.t -> t list -> t

  val grid
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> columns:int
    -> t list
    -> t Or_error.t
end
