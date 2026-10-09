open Core
module Diff = Document_diff
module Preview = Document_preview
module Style = Document_style
module Activation = Document_activation
module Markdown_options = Document_markdown_options
module Actions = Document_actions
module Profile = Document_profile

module Language : sig
  (** A syntax token/extension, not an executable grammar or file path.
      Unknown languages render as plain text. *)
  type t [@@deriving equal, sexp_of]

  val plain_text : t
  val ocaml : t
  val rust : t
  val shell : t
  val json : t
  val python : t
  val javascript : t
  val typescript : t
  val yaml : t
  val toml : t
  val ini : t
  val of_string : string -> t Or_error.t
  val to_string : t -> string
end

module Mode : sig
  type t =
    | Markdown
    | Code of Language.t
    | Diff
    | Html
    (** Basic reader markup, without browser scripting/stylesheets/resource loading. *)
  [@@deriving equal, sexp_of]
end

module Appearance : sig
  type t =
    | Light
    | Dark
  [@@deriving equal, sexp_of]
end

module Layout : sig
  type t =
    | Flow
    | Viewport of float
  [@@deriving equal, sexp_of]
end

module Navigation : sig
  module Side : sig
    type t =
      | Before
      | After
    [@@deriving equal, sexp_of]
  end

  (** Current readers supply input metadata for links. [activation=None] is
      reserved for the original URL-only bridge event, with unknown input source.
      No URL opens automatically. Keyboard metadata also covers native synthetic
      accessibility activation; it does not establish a physical input source. *)
  type t =
    | Link of
        { url : string
        ; activation : Activation.t option
        }
    | Line of
        { path : string option
        ; side : Side.t
        ; line : int
        }
  [@@deriving equal, sexp_of]
end

module Selection_format : sig
  type t =
    | Plain_text
    | Markdown
  [@@deriving equal, sexp_of]
end

module Setting : sig
  type 'a t =
    | Inherit
    | Builtin
    | Value of 'a
  [@@deriving equal, sexp_of]
end

module Config : sig
  type t [@@deriving equal, sexp_of]

  (** Omitted reader settings inherit application defaults when reconciled.
      Explicit arguments override them, even when equal to built-in values.
      Before runtime resolution, getters show the built-in fallback for omitted
      settings. [with_overrides] expresses per-field resets and inheritance.

      [source] is a borrowed registered document. [Flow] fits bounded content
      into its parent; [Viewport height] owns a native vertical scroll viewport
      of 1..16384 logical pixels. Code/diff scroll horizontally without wrapping.
      Huge Markdown/HTML uses the bounded source presentation described in the
      document contract. [initially_collapsed] applies on mount/generation reset.
      [path] labels navigation; it never reads a file. [search] is literal text.

      [diff] is valid only for [Mode.Diff] and configures the extended diff
      controls: per-file collapse, body-row previews, Show more and word emphasis.
      [View.document ~on_diff] observes typed actions asynchronously. Omission
      preserves the raw unified-diff presentation. Diff syntax uses each side's
      filename label, without file I/O; unknown languages or syntax work limits
      retain complete diff colors. Grammar state resets at hunk gaps because
      omitted source cannot be recovered from the patch.

      Markdown/HTML image URLs resolve only through [images], an explicit mapping to
      application asset handles; no implicit network or filesystem acquisition.
      At most128 unique URLs, 4096 UTF-8 bytes each. *)
  val create
    :  source:Text_source.Handle.t
    -> mode:Mode.t
    -> ?appearance:Appearance.t
    -> ?layout:Layout.t
    -> ?label:string
    -> ?path:string
    -> ?line_numbers:bool
    -> ?initially_collapsed:bool
    -> ?search:string
    -> ?images:(string * Asset.Handle.t) list
    -> ?diff:Diff.Config.t
    -> ?selection_format:Selection_format.t
    -> ?max_lines:int
    -> ?text_style:Style.t
    -> ?actions:Actions.Config.t
    -> ?markdown_options:Markdown_options.t
    -> unit
    -> t Or_error.t

  (** Omitted modifier fields preserve their prior intent. Inherit restores
      application inheritance; Builtin ignores the application value. *)
  val with_overrides
    :  t
    -> ?appearance:Appearance.t Setting.t
    -> ?layout:Layout.t Setting.t
    -> ?line_numbers:bool Setting.t
    -> ?initially_collapsed:bool Setting.t
    -> ?selection_format:Selection_format.t Setting.t
    -> ?max_lines:int Setting.t
    -> ?text_style:Style.t Setting.t
    -> ?actions:Actions.Config.t Setting.t
    -> ?markdown_options:Markdown_options.t Setting.t
    -> unit
    -> t Or_error.t

  val source : t -> Text_source.Handle.t
  val mode : t -> Mode.t
  val appearance : t -> Appearance.t
  val layout : t -> Layout.t
  val label : t -> string
  val path : t -> string option
  val line_numbers : t -> bool
  val initially_collapsed : t -> bool
  val search : t -> string
  val images : t -> (string * Asset.Handle.t) list
  val diff : t -> Diff.Config.t option

  (** Native selected-content copy format for rich Markdown. Default [Plain_text].
      [Markdown] copies original source for select-all and reconstructed Markdown
      for partial selection. Format changes preserve selection and source identity.
      HTML always copies selected plain text.
      Code/diff, source fallback and explicit Copy source/code/table are unchanged. *)
  val selection_format : t -> Selection_format.t

  (** Optional 1..4096 body-line-height budget for rich Markdown/HTML in [Flow].
      Paragraph spacing, headings and embedded content consume that budget.
      Omission removes the limit. Other modes/layouts reject a supplied limit.
      This limits presentation, not source parsing or source-copy behavior.
      Changing it preserves the source, native entity and parser work.
      [View.document ~on_preview] observes the actual native presentation. *)
  val max_lines : t -> int option

  (** Optional internal Markdown/HTML styling; other modes reject a supplied value. *)
  val text_style : t -> Style.t option

  (** Parser settings; nondefault values require Markdown mode. *)
  val markdown_options : t -> Markdown_options.t

  (** Rich code/table action buttons. Nondefault values require Markdown/HTML;
      custom actions also require [View.document ~on_action] or an application
      default action handler. *)
  val actions : t -> Actions.Config.t
end

module Defaults : sig
  (** Immutable application-owned reader defaults. Pass to [Gpuio_eio.App.run]
      or [run_desktop]; all their windows inherit these settings. *)
  type 'action t

  val empty : 'action t

  val create
    :  ?appearance:Appearance.t
    -> ?layout:Layout.t
    -> ?line_numbers:bool
    -> ?initially_collapsed:bool
    -> ?selection_format:Selection_format.t
    -> ?max_lines:int
    -> ?text_style:Style.t
    -> ?actions:Actions.Config.t
    -> ?markdown_options:Markdown_options.t
    -> ?on_action:(Config.t -> Actions.Event.t -> 'action)
    -> unit
    -> 'action t Or_error.t

  val with_profile
    :  'action t
    -> 'event Profile.Instance.t
    -> on_event:(Config.t -> 'event Profile.Event.t -> 'action)
    -> 'action t

  module Expert : sig
    val resolve : _ t -> Config.t -> Config.t Or_error.t
    val action_handler : 'action t -> Config.t -> (Actions.Event.t -> 'action) option

    val profile
      :  'action t
      -> Config.t
      -> (Gpuio_protocol.Document_profile_wire.Instance.t
         * (Gpuio_protocol.Document_profile_wire.Event.t -> 'action option))
           option
  end
end

module Expert : sig
  val to_wire
    :  Config.t
    -> owner:Text_source.Expert.Owner.t option
    -> asset_owner:Asset.Expert.Owner.t option
    -> Gpuio_protocol.Document_wire.Config.t

  val navigation : Gpuio_protocol.Document_wire.Navigation.t -> Navigation.t Or_error.t
end
