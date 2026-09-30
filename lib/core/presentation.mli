open Core

(** Stateless compositions. These helpers own no editors, tasks, registrations or
    controllers. All content slots accept ordinary views, including Bonsai effects.
    Application state and asynchronous work remain with the caller. *)
module Size : sig
  type t =
    | Small
    | Medium
    | Large
  [@@deriving equal, sexp_of]
end

module Tone : sig
  type t =
    | Neutral
    | Accent
    | Success
    | Warning
    | Danger
  [@@deriving equal, sexp_of]
end

module Variant : sig
  type t =
    | Soft
    | Outline
    | Solid
  [@@deriving equal, sexp_of]
end

module Axis : sig
  type t =
    | Horizontal
    | Vertical
  [@@deriving equal, sexp_of]
end

module Appearance : sig
  type t

  (** Concrete defaults work with existing custom themes without new tokens.
      Custom colors may use application tokens, resolved by the normal theme. *)
  val light : t

  val dark : t

  (** Default glyph effect for status-bearing compositions. Built-in light/dark
      appearances provide matching explicit shimmer colors/mode; the dark palette
      uses a white highlight so foreground-colored titles still show a sweep. Custom [create]
      defaults to the native palette; supply a resolved application configuration
      here when the app owns its theme. *)
  val with_text_shimmer : t -> Text_shimmer.Config.t -> t

  val create
    :  surface:Color.t
    -> raised:Color.t
    -> foreground:Color.t
    -> muted:Color.t
    -> border:Color.t
    -> on_solid:Color.t
    -> accent:Color.t
    -> success:Color.t
    -> warning:Color.t
    -> danger:Color.t
    -> t
end

(** Styles refine component defaults. Text wraps unless the caller requests
    truncation; empty/localized labels do not allocate hidden placeholder text. *)
val label : ?key:Key.t -> ?style:Style.t -> string -> 'action View.t

(** Inline secondary text, match coloring and display masking from [Label.create].
    Uses one ordinary/selectable text layout. Style refines the inherited primary
    foreground; secondary and matched runs use the appearance's muted/accent colors.
    State remains caller-owned; masked values expose only their replacement text. *)
val styled_label
  :  Appearance.t
  -> ?key:Key.t
  -> ?style:Style.t
  -> Label.t
  -> 'action View.t

val badge
  :  Appearance.t
  -> ?key:Key.t
  -> ?style:Style.t
  -> ?size:Size.t
  -> ?tone:Tone.t
  -> ?variant:Variant.t
  -> ?leading:'action View.t
  -> string
  -> 'action View.t

module Overlay_badge : sig
  type t

  (** Nonnegative [count] and [max] (default 99). Zero hides the overlay; counts
      above [max] display [max+]. [label] is a meaningful, localized description
      of the uncapped count, not just the displayed cap. Labels are nonblank UTF-8
      without NUL, at most 4096 bytes. *)
  val count : ?max:int -> label:string -> int -> t Or_error.t

  (** A colored dot with an explicit meaning, so color is not the only signal. *)
  val dot : label:string -> t Or_error.t

  (** Reuses the icon's explicit meaningful/decorative description and borrowed
      asset handle. Registration and lifetime remain application-owned. *)
  val icon : Icon.Config.t -> t
end

(** Overlay on ordinary content, distinct from the text-chip [badge]. Count/dot
    attach to the top-right corner, icons to the bottom-right. [size] affects the
    badge only; [style] refines the wrapper and [badge_style] refines the overlay.
    Use these styles for alternative offsets/colors. Badges stay pointer-passive
    and nonselectable; they add no action or focus stop. The wrapped content keeps
    its identity when the badge changes or disappears. Ancestor overflow clipping
    still applies. Accessible labels do not automatically announce each update. *)
val overlay_badge
  :  Appearance.t
  -> ?key:Key.t
  -> ?style:Style.t
  -> ?badge_style:Style.t
  -> ?size:Size.t
  -> ?tone:Tone.t
  -> badge:Overlay_badge.t
  -> 'action View.t
  -> 'action View.t

(** The optional trailing slot can hold a separately labelled remove button.
    The tag itself is not an action or an additional focus stop. *)
val tag
  :  Appearance.t
  -> ?key:Key.t
  -> ?style:Style.t
  -> ?size:Size.t
  -> ?tone:Tone.t
  -> ?variant:Variant.t
  -> ?leading:'action View.t
  -> ?trailing:'action View.t
  -> string
  -> 'action View.t

(** A colored dot accompanied by readable text; color is never the only signal. *)
val marker
  :  Appearance.t
  -> ?key:Key.t
  -> ?style:Style.t
  -> ?tone:Tone.t
  -> string
  -> 'action View.t

module Marker : sig
  module Variant : sig
    type t =
      | Plain
      | Separator
      | Border
    [@@deriving equal, sexp_of]
  end

  module Loading_style : sig
    type t =
      | Spinner
      | Shimmer
    [@@deriving equal, sexp_of]
  end

  module Spinner : sig
    type t

    (** Localized native progress-indicator label, default "Loading". Same label,
        animation and period bounds as [Loading.Config.create]; kind is Spinner.
        The indicator adds no focus stop or live announcement. *)
    val create
      :  ?label:string
      -> ?animated:bool
      -> ?period:Time_ns.Span.t
      -> unit
      -> t Or_error.t

    val default : t
  end

  module Icon : sig
    type 'action t

    (** A 16px square, nonshrinking centered slot; style refines those defaults.
        Even an empty typed icon suppresses the automatic spinner. *)
    val create : key:Key.t -> ?style:Style.t -> 'action View.t list -> 'action t
  end

  module Content : sig
    module Item : sig
      type 'action t

      (** Valid UTF-8, at most 16384 bytes even when not loading. Empty typed text
          still counts as text and suppresses the rich-only pulse. *)
      val text : key:Key.t -> ?style:Style.t -> string -> 'action t Or_error.t

      (** Uses this key directly on the supplied view; adds no layout wrapper. *)
      val element : key:Key.t -> 'action View.t -> 'action t
    end

    type 'action t

    (** Rejects duplicate item keys. Content is a stable native animation root,
        including while static, preserving descendant identity across loading and
        variant changes. Styles apply directly to this root. Each mounted content
        consumes one of the application's 1024 advanced animation owner slots;
        use managed lists for large histories. Idle content requests no frames. *)
    val create
      :  key:Key.t
      -> ?style:Style.t
      -> 'action Item.t list
      -> 'action t Or_error.t
  end

  module Item : sig
    type 'action t

    val icon : 'action Icon.t -> 'action t
    val content : 'action Content.t -> 'action t
    val element : key:Key.t -> 'action View.t -> 'action t
  end

  (** Full-width muted row, minimum height 16px, gap 8px. [Separator] centers
      content between decorative lines; [Border] adds a bottom border and padding.
      [style] refines the row; [separator_style] refines each line.

      Loading defaults to false, style to Spinner. A typed Icon suppresses the
      automatic spinner; arbitrary elements do not. Shimmer applies only to typed
      text. Content without any typed text instead pulses its entire styled opacity
      by 0.6..1, using two native ease-in-out stages over the shimmer duration.
      This is a smooth pulse, not exact cosine easing. Mixed rich children and
      root-level arbitrary elements are unchanged. [shimmer] inherits Appearance's
      configuration, including duration/repeat/animated; reduced motion restores
      ordinary paint natively. Stopping restores a factor of one immediately.

      Children own their ordinary interaction/accessibility behavior. The row adds
      no live region; callers can apply [View.with_accessibility] explicitly.
      Item keys must be unique and must not use the reserved prefix
      [gpuio:marker:]. Violations are rejected before reconciliation. Legacy
      [marker]'s dot/string behavior is unchanged. *)
  val create
    :  Appearance.t
    -> ?key:Key.t
    -> ?style:Style.t
    -> ?separator_style:Style.t
    -> ?variant:Variant.t
    -> ?loading:bool
    -> ?loading_style:Loading_style.t
    -> ?spinner:Spinner.t
    -> ?shimmer:Text_shimmer.Config.t
    -> 'action Item.t list
    -> 'action View.t Or_error.t
end

(** Uses native button keyboard/AX activation with Link semantics. Activation
    delivers the supplied action; opening a URL is an explicit application job. *)
val link
  :  Appearance.t
  -> ?key:Key.t
  -> ?style:Style.t
  -> ?disabled:bool
  -> on_click:(unit -> 'action)
  -> string
  -> 'action View.t

(** Styled single-target link with composed passive content. See [View.link] for
    content validation and [Link.Config] for accessibility and Tab policy. *)
val composed_link
  :  Appearance.t
  -> ?key:Key.t
  -> ?style:Style.t
  -> Link.Config.t
  -> on_click:(unit -> 'action)
  -> 'action View.t list
  -> 'action View.t Or_error.t

val separator
  :  Appearance.t
  -> ?key:Key.t
  -> ?style:Style.t
  -> ?axis:Axis.t
  -> unit
  -> 'action View.t

module Separator : sig
  (** Centered decorative line with an optional wrapping text label. Horizontal
      defaults to full width; vertical defaults to full height and needs a bounded
      parent height. Without a label, the cross-axis size is one logical pixel.
      The root has Separator semantics and introduces no focus stop.

      [style] refines the root, [line_style] the absolute line, and [label_style]
      the label. The label defaults to the appearance surface behind muted text;
      refine its background when placing the separator on another surface.
      [color] defaults to the appearance border color. [pattern] defaults to Solid;
      Dashed uses the pinned GPUI border pattern, not a custom dash array.

      The original [separator] helper retains its background-rectangle styling
      contract; this richer composition does not change existing calls. *)
  val create
    :  Appearance.t
    -> ?key:Key.t
    -> ?style:Style.t
    -> ?line_style:Style.t
    -> ?label_style:Style.t
    -> ?axis:Axis.t
    -> ?pattern:Style.Border_style.t
    -> ?color:Color.t
    -> ?label:string
    -> unit
    -> 'action View.t
end

module Group_variant : sig
  type t =
    | Card
    | Plain
    | Filled
    | Outline
  [@@deriving equal, sexp_of]
end

(** Stable header/body/footer wrappers preserve body identity as slots and styles
    change. [Card] (the default) places padding, background and border around the
    whole group, preserving the original appearance. [Plain] adds no panel;
    [Filled] and [Outline] decorate and pad only the body, leaving header/footer
    outside it. Root [style] and each slot style refine their respective defaults.
    The group adds no focus stop; children keep their normal native behavior. *)
val group_box
  :  Appearance.t
  -> ?key:Key.t
  -> ?style:Style.t
  -> ?variant:Group_variant.t
  -> ?header_style:Style.t
  -> ?body_style:Style.t
  -> ?footer_style:Style.t
  -> ?header:'action View.t
  -> ?footer:'action View.t
  -> 'action View.t list
  -> 'action View.t

(** Settings content may contain [Form.field] or other ordinary views. *)
val settings_group
  :  Appearance.t
  -> ?key:Key.t
  -> ?style:Style.t
  -> title:string
  -> ?description:string
  -> 'action View.t list
  -> 'action View.t

module Description : sig
  type 'action t

  val create : key:Key.t -> term:string -> definition:'action View.t -> 'action t
end

(** Entry keys must be unique, as for ordinary keyed siblings. Term/definition
    semantic roles are retained in both horizontal and stacked layouts. *)
val description_list
  :  Appearance.t
  -> ?key:Key.t
  -> ?style:Style.t
  -> ?stacked:bool
  -> 'action Description.t list
  -> 'action View.t

val empty_state
  :  Appearance.t
  -> ?key:Key.t
  -> ?style:Style.t
  -> ?icon:'action View.t
  -> title:string
  -> ?description:string
  -> ?actions:'action View.t
  -> unit
  -> 'action View.t

(** Rich empty-state compositions. Use [empty_state] for the existing string-based
    convenience layout. These helpers accept ordinary views and own no resources,
    effects or focus handles. Text remains accessible; no automatic live region
    or additional focus stop is introduced. *)
module Empty_state : sig
  module Media_variant : sig
    type t =
      | Unframed
      | Icon
    [@@deriving equal, sexp_of]
  end

  (** Intrinsically sized media column, suitable for an image or avatar row.
      [Icon] adds a 32-logical-pixel muted rounded frame. Child styles retain their
      ordinary precedence over inherited defaults. *)
  val media
    :  Appearance.t
    -> ?key:Key.t
    -> ?style:Style.t
    -> ?variant:Media_variant.t
    -> 'action View.t list
    -> 'action View.t

  val title : ?key:Key.t -> ?style:Style.t -> 'action View.t list -> 'action View.t

  (** Muted wrapping text with line height 1.625 times the effective font size.
      Custom font sizes preserve that ratio unless [Line_height] is also refined. *)
  val description
    :  Appearance.t
    -> ?key:Key.t
    -> ?style:Style.t
    -> 'action View.t list
    -> 'action View.t

  (** Optional rich slots appear in media/title/description order. Stable wrappers
      preserve surviving controls when another slot is added or removed. Defaults
      center the slots in a full-width header, capped at 384 logical pixels. *)
  val header
    :  ?key:Key.t
    -> ?style:Style.t
    -> ?media:'action View.t
    -> ?title:'action View.t
    -> ?description:'action View.t
    -> unit
    -> 'action View.t

  (** Centered action/input column, full width capped at 384 logical pixels.
      Styles may change its axis, alignment, gap and width. *)
  val content : ?key:Key.t -> ?style:Style.t -> 'action View.t list -> 'action View.t

  (** Named header/content precede extra children. Extras have a separate keyed
      wrapper, so their keys do not collide with named slots; [children_style]
      refines that wrapper's centered column. Missing slots add no placeholders.
      Root and helper styles refine defaults independently; omit a previous custom
      style to reset it. The root has no visible background or border by default.
      Supplying a border width reveals the default dashed pattern and appearance
      border color; [Border_style Solid] overrides the pattern independently.
      Unsetting [Border_style] removes the helper's declaration, exposing the native
      solid default; omitting that custom unset restores the helper's dashed default.
      Asset registration, input models, async work and visibility remain caller-owned. *)
  val create
    :  Appearance.t
    -> ?key:Key.t
    -> ?style:Style.t
    -> ?children_style:Style.t
    -> ?header:'action View.t
    -> ?content:'action View.t
    -> 'action View.t list
    -> 'action View.t
end

(** [live] defaults to Polite. Use Off for persistent information, Assertive only
    for urgent changes. Reconciliation emits semantics only when they change. *)
val alert
  :  Appearance.t
  -> ?key:Key.t
  -> ?style:Style.t
  -> ?tone:Tone.t
  -> ?live:Accessibility.Live.t
  -> ?icon:'action View.t
  -> title:string
  -> ?actions:'action View.t
  -> 'action View.t list
  -> 'action View.t

val banner
  :  Appearance.t
  -> ?key:Key.t
  -> ?style:Style.t
  -> ?tone:Tone.t
  -> ?live:Accessibility.Live.t
  -> ?icon:'action View.t
  -> title:string
  -> ?actions:'action View.t
  -> 'action View.t list
  -> 'action View.t

module Alert : sig
  module Variant : sig
    type t =
      | Default
      | Info
      | Success
      | Warning
      | Error
    [@@deriving equal, sexp_of]
  end

  module Size : sig
    type t =
      | XSmall
      | Small
      | Medium
      | Large
    [@@deriving equal, sexp_of]
  end

  module Layout : sig
    type t =
      | Card
      | Banner
    [@@deriving equal, sexp_of]
  end

  module Icon : sig
    (** Default uses a semantic text glyph; Custom accepts ordinary views, including
        registered SVG icons. Hidden omits the slot without moving other identities. *)
    type 'action t =
      | Default
      | Hidden
      | Custom of 'action View.t
  end

  module Close : sig
    type 'action t

    (** Localized nonblank UTF-8 label without NUL, at most 1024 bytes. The native
        button displays a cross and exposes this name. Its queued callback does
        not change visibility; application state owns dismissal. *)
    val create
      :  label:string
      -> ?style:Style.t
      -> ?disabled:bool
      -> on_click:(unit -> 'action)
      -> unit
      -> 'action t Or_error.t
  end

  (** Single-line text title with ellipsis; explicit style refines width, weight
      and overflow. Rich titles can instead be supplied directly to [create]. *)
  val title : ?key:Key.t -> ?style:Style.t -> string -> 'action View.t

  (** Full-width alert with independently styled title/body/icon slots. Default
      size Medium and layout Card. Banner omits title and radius but retains the
      border, matching the pinned renderer. Root and slot styles refine defaults.
      Semantic variants use Appearance colors with a 4% background and 30% border
      alpha tint; Default uses surface/border. This is a GPUIO palette mapping.

      Role Alert, default live Off; opt into Polite/Assertive for announcements.
      Cosmetic changes do not change semantic metadata. Visible=false removes
      children and semantics and returns an empty hidden root, regardless of style.
      Surviving body/close controls retain identity across layout/variant changes.
      Removing a slot or hiding retires its ordinary native views; no retained
      hidden tasks, editor state or callbacks are owned by this composition.
      Legacy [alert]/[banner] keep their existing behavior. *)
  val create
    :  Appearance.t
    -> ?key:Key.t
    -> ?style:Style.t
    -> ?title_style:Style.t
    -> ?body_style:Style.t
    -> ?icon_style:Style.t
    -> ?variant:Variant.t
    -> ?size:Size.t
    -> ?layout:Layout.t
    -> ?icon:'action Icon.t
    -> ?title:'action View.t
    -> ?close:'action Close.t
    -> ?live:Accessibility.Live.t
    -> ?visible:bool
    -> 'action View.t list
    -> 'action View.t
end

(** Display only: these labels do not register shortcuts or commands. The caller
    supplies platform-appropriate, already formatted key names. *)
val shortcut_label
  :  Appearance.t
  -> ?key:Key.t
  -> ?style:Style.t
  -> string list
  -> 'action View.t

(** A structural bar, not a live region by default. Individual status content may
    carry explicit live semantics. Slot contents keep their own native actions.
    [center] fills the space between the ends: its content is centered when both
    ends exist, end-aligned with only [leading], and start-aligned otherwise.
    This is centering in the remaining space, not the whole window. Adding or
    removing a slot preserves controls in the other slots. Omitting [center]
    preserves the two-region layout. *)
val status_bar
  :  Appearance.t
  -> ?key:Key.t
  -> ?style:Style.t
  -> ?leading:'action View.t
  -> ?center:'action View.t
  -> ?trailing:'action View.t
  -> unit
  -> 'action View.t

val attachment
  :  Appearance.t
  -> ?key:Key.t
  -> ?style:Style.t
  -> ?preview:'action View.t
  -> name:string
  -> ?detail:string
  -> ?actions:'action View.t
  -> unit
  -> 'action View.t

(** Rich attachments. All uploads, files, task lifetimes, asset registrations and
    lifecycle status remain application-owned. These are ordinary keyed views;
    status/layout changes do not replace surviving slots. The string [attachment]
    helper above keeps its original layout. *)
module Attachment : sig
  module Status : sig
    type t =
      | Pending
      | Uploading
      | Processing
      | Failed
      | Complete
    [@@deriving equal, sexp_of]

    val is_in_progress : t -> bool
  end

  module Size : sig
    type t [@@deriving equal, sexp_of]

    val xsmall : t
    val small : t
    val medium : t
    val large : t

    (** Custom logical-pixel sizing basis, finite and in [1,1000000]. *)
    val pixels : float -> t Or_error.t
  end

  module Title : sig
    type t

    (** Keys are sibling identities within [Content]. Title source must be valid
        UTF-8, at most 16384 bytes, so changing status can always enable shimmer.
        Status/configuration overrides are independent; otherwise both inherit
        from the card. Uploading/Processing enables the native glyph effect. *)
    val create
      :  key:Key.t
      -> ?style:Style.t
      -> ?status:Status.t
      -> ?shimmer:Text_shimmer.Config.t
      -> string
      -> t Or_error.t
  end

  module Description : sig
    type t

    (** Single-line muted text, destructive at 80% alpha when Failed. *)
    val create : key:Key.t -> ?style:Style.t -> ?status:Status.t -> string -> t
  end

  module Content : sig
    module Item : sig
      type 'action t

      val title : Title.t -> 'action t
      val description : Description.t -> 'action t

      (** Arbitrary keyed content keeps its own behavior; it does not implicitly
          inherit title/description status styling. *)
      val element : key:Key.t -> 'action View.t -> 'action t
    end

    type 'action t

    (** Keys follow the ordinary View sibling-uniqueness rule. *)
    val create : ?style:Style.t -> 'action Item.t list -> 'action t
  end

  module Media : sig
    module Image : sig
      type 'action t

      (** Cover by default. Decode/registration ownership and observations follow
          [View.image]. A supplied image, including a failed decode, keeps the
          image status policy; only the image is dimmed during work/failure. *)
      val create
        :  asset:Asset.Handle.t
        -> description:Image.Description.t
        -> ?fit:Image.Fit.t
        -> ?on_change:(Image.State.t -> 'action)
        -> unit
        -> 'action t
    end

    type 'action t

    (** Horizontal size inherits the card unless overridden. Vertical media fills
        the available width with ratio 1. Children and centered [overlay] paint
        above the image without inheriting its 60% opacity. *)
    val create
      :  ?style:Style.t
      -> ?size:Size.t
      -> ?image:'action Image.t
      -> ?overlay:'action View.t
      -> 'action View.t list
      -> 'action t
  end

  module Actions : sig
    type 'action t

    (** Horizontal row; top-right overlay in a vertical card. Pointer shielding
        on the cluster is enforced after custom styling: its gaps and disabled
        controls cannot arm the card trigger. Children keep their own actions;
        wheel events retain native propagation. *)
    val create : ?style:Style.t -> 'action View.t list -> 'action t
  end

  module Trigger : sig
    type 'action t

    (** One native button with a required nonempty accessible name (at most 1024
        UTF-8 bytes, no NUL). Its stable key is scoped separately from other slots.
        The trigger covers media/content and paints below Actions; place independent
        controls in Actions when using whole-card activation. Keyboard/AX activation
        dispatches the same queued action as pointer activation. *)
    val create
      :  key:Key.t
      -> accessible_name:string
      -> ?style:Style.t
      -> ?disabled:bool
      -> on_click:(unit -> 'action)
      -> unit
      -> 'action t Or_error.t
  end

  (** Defaults: Complete, Medium, Horizontal. Pending uses a dashed border;
      Failed uses a translucent destructive border. Vertical cards are 120px wide
      with content and 96px without; root style refines all visual defaults.
      [shimmer] overrides the appearance default for this card, while an individual
      Title may override it again. Reduced motion remains native-owned. *)
  val create
    :  Appearance.t
    -> ?key:Key.t
    -> ?style:Style.t
    -> ?status:Status.t
    -> ?size:Size.t
    -> ?axis:Axis.t
    -> ?shimmer:Text_shimmer.Config.t
    -> ?media:'action Media.t
    -> ?content:'action Content.t
    -> ?actions:'action Actions.t
    -> ?trigger:'action Trigger.t
    -> unit
    -> 'action View.t

  (** Ordinary horizontal scroll container with stable identity, 12px gaps and
      4px vertical padding. Scroll ownership follows ordinary [Overflow_x Scroll];
      this is not a managed/virtualized list. Child attachment keys remain caller-owned. *)
  val group : key:Key.t -> ?style:Style.t -> 'action View.t list -> 'action View.t
end

(** Document/asset registration and streaming state remain application-owned.
    These slots also accept native Markdown/code/diff views. *)
val message
  :  Appearance.t
  -> ?key:Key.t
  -> ?style:Style.t
  -> ?avatar:'action View.t
  -> author:string
  -> ?detail:string
  -> ?actions:'action View.t
  -> ?footer:'action View.t
  -> 'action View.t
  -> 'action View.t

val bubble
  :  Appearance.t
  -> ?key:Key.t
  -> ?style:Style.t
  -> ?tone:Tone.t
  -> 'action View.t
  -> 'action View.t

val tool_result
  :  Appearance.t
  -> ?key:Key.t
  -> ?style:Style.t
  -> title:string
  -> ?status:'action View.t
  -> ?actions:'action View.t
  -> ?footer:'action View.t
  -> 'action View.t
  -> 'action View.t
