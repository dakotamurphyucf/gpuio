open Core

(** Logical avatar size. Named values match the pinned component's
    16/24/48/80px presets; custom values are finite, positive and at most 1,000,000px.
    Default text size is 30% of size, clamped to 1..4096px and overridable by item
    styles. Native image/resource budgets apply independently of logical size. *)
module Size : sig
  type t [@@deriving equal, sexp_of]

  val xsmall : t
  val small : t
  val medium : t
  val large : t
  val of_pixels : float -> t Or_error.t
  val pixels : t -> float
end

module Item : sig
  type 'action t

  (** The group borrows the supplied asset configuration; it does not register
      images or own application tasks. [key] is stable and unique across all
      items, including currently omitted members. *)
  val create
    :  key:Key.t
    -> ?style:Style.t
    -> ?on_change:(Image.State.t -> 'action)
    -> Avatar.Config.t
    -> 'action t

  (** Checked passive rich slot with [View.avatar_with_fallback]'s ownership,
      layout, clipping and semantic contract. Group sizing styles the avatar root. *)
  val create_with_fallback
    :  key:Key.t
    -> ?style:Style.t
    -> ?on_change:(Image.State.t -> 'action)
    -> Avatar.Config.t
    -> fallback:'action View.t
    -> 'action t Or_error.t

  val key : _ t -> Key.t
end

(** A passive ellipsis avatar, useful in [create]'s overflow renderer. The caller
    chooses a meaningful description (including the omitted count) or decorative
    semantics. [size] defaults to Medium. No click action is implied. *)
val ellipsis
  :  ?size:Size.t
  -> ?style:Style.t
  -> description:Image.Description.t
  -> unit
  -> 'action View.t

(** Compact avatars in logical item order. Defaults: Medium size, limit three,
    30% overlap and no overflow content. [limit] is nonnegative; zero mounts no
    members. [overlap] is finite and in [0,1), measured as a fraction of size.
    Duplicate keys are rejected before applying the limit.

    Only the visible prefix is mounted. Lowering the limit unmounts omitted
    members and retires native image leases/observations; increasing it remounts
    them. Reordering surviving keys or changing size retains native identities.
    Actual registrations remain caller-owned. Hidden members have no callbacks.

    [overflow] is called during ordinary OCaml view construction only when members
    are omitted, with their exact count and shared size. Its returned view occupies
    a separate stable trailing slot, separated by 4px. It may be a normal native
    control with caller-owned actions. It is never invoked from Rust rendering.

    Logical/AX order is input order followed by overflow. Later avatars paint over
    earlier avatars; this differs from the pinned styled layer's reverse paint
    order while preserving a predictable semantic order. Interaction belongs to
    explicit native controls around/after the group, not to a passive avatar leaf.

    Styles refine appearance. Shared avatar dimensions, nonshrinking sizing,
    horizontal layout and overlap margins are applied after custom base styles;
    do not override those structural fields with state styles. Native clipping,
    source-generation checks and leaf accessibility semantics remain unchanged. *)
val create
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?size:Size.t
  -> ?limit:int
  -> ?overlap:float
  -> ?overflow:(omitted:int -> size:Size.t -> 'action View.t)
  -> 'action Item.t list
  -> 'action View.t Or_error.t
