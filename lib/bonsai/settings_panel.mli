open Core
module B = Bonsai.Cont
module S = Gpuio.Settings

type view = unit Bonsai.Effect.t Gpuio.View.t

module Labels : sig
  type t

  val english : t

  val create
    :  navigation:string
    -> empty:string
    -> resize:string
    -> reset_matches:string
    -> reset_page:string
    -> reset_group:string
    -> current_page:string
    -> current_group:string
    -> expand:(string -> string)
    -> collapse:(string -> string)
    -> t Or_error.t
end

module Output : sig
  type t

  val view : t -> view
  val active_groups : t -> int
  val budget_exhausted : t -> bool
end

(** Controlled settings presentation. Give the split a bounded width/height.
    The caller owns [model], reduces requests against its latest metadata, and
    supplies one search editor placement whose edits update Settings.Query.
    Reset callbacks receive scopes, never precomputed setters; resolve them with
    Settings.reset_targets against current state, including after confirmation.

    Groups use the existing native managed list; item computations are transient.
    Keep persistent field values/controllers/tasks outside [render_item]. Guard
    delayed row work with its lifetime. Custom rows own their semantic content;
    labelled fields receive a label/help layout, and the supplied control must
    carry its own native accessible name. Disabled items wrap all contents in
    native Inert, including custom controls.

    Native breakpoint observations select vertical layout below 480 logical
    content pixels, horizontal otherwise. They update styles on a stable control
    placement; they never duplicate editor controllers across query branches.
    Item Vertical overrides a horizontal container. Page suffixes are ordinary
    views, and must not duplicate a controller used elsewhere. Page icons must
    satisfy composed-link passive-content rules; invalid contents return errors.
    Page changes retire the transient viewport; application scroll restoration
    is not implicit. [size] sets group/item spacing, not supplied control sizes.

    The sidebar uses ordinary keyed disclosure/link controls with separate page
    and group namespaces, preserving full-length typed IDs without hashing.
    Split defaults are 250px, 160..360px. Resizing remains native. [on_resize]
    reports completed native changes. Event callbacks run asynchronously in OCaml.
    Renderers, slot suppliers and localizers are pure OCaml view computations.
    Invalid dynamic localized labels are returned as errors. *)
val component
  :  ?key:Gpuio.Key.t
  -> ?style:Gpuio.Style.t B.t
  -> ?labels:Labels.t
  -> ?split:Gpuio.Split_pane.Config.t
  -> ?groups:Gpuio.Virtual_list.Config.t
  -> ?group_variant:Gpuio.Presentation.Group_variant.t B.t
  -> ?size:Gpuio.Presentation.Size.t B.t
  -> ?on_resize:(Gpuio.Split_pane.Snapshot.t -> unit Bonsai.Effect.t) B.t
  -> ?page_suffix:(S.Page.t -> view option) B.t
  -> ?page_icon:(S.Page.t -> view option) B.t
  -> appearance:Gpuio.Presentation.Appearance.t B.t
  -> model:S.t B.t
  -> search:view B.t
  -> on_request:(S.Request.t -> unit Bonsai.Effect.t) B.t
  -> on_reset:(S.Reset_scope.t -> unit Bonsai.Effect.t) B.t
  -> render_item:
       (item:S.Item.t B.t
        -> layout:S.Layout.t B.t
        -> lifetime:Managed_rows.Lifetime.t B.t
        -> B.graph
        -> view B.t)
  -> B.graph
  -> Output.t Or_error.t B.t
