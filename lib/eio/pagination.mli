open Core

(** Controlled pagination with an optional native page-entry popover. The
    application owns the model and applies requests to its latest value. The
    chooser never enumerates a gap: it renders at most seven shortcuts and one
    Rust-owned numeric field, even at [Gpuio.Pagination.max_pages]. *)
type t

val create
  :  App.Window.t
  -> model:Gpuio.Pagination.t Bonsai.Cont.t
  -> ?layout:Gpuio.Navigation.Pagination_layout.t Bonsai.Cont.t
  -> on_request:(Gpuio.Pagination.Request.t -> unit Bonsai.Effect.t) Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> t Bonsai.Cont.t

val is_open : t -> bool
val is_confirming : t -> bool
val error : t -> Gpuio.Number_input.Command_error.t option

(** Captured dismissal cannot close a newer opening. *)
val cancel : t -> unit Bonsai.Effect.t

(** Read and commit the actual native draft, then revalidate the opening and
    latest application model before requesting a page. Numeric normalization
    clamps to the chosen gap and rounds to an integer. Rejection/composition
    preserves the popup and exposes [error]. Concurrent confirmations coalesce.
    Enter in the field normalizes its draft; it does not navigate. Use the Go
    button or this effect to confirm. *)
val confirm : t -> unit Bonsai.Effect.t

(** Use once per controller. Full layout has interactive ellipses; compact has
    only previous/next controls and closes the chooser. Every opening gives the
    numeric field a fresh identity. Model changes, disabling and deactivation
    discard the opening; delayed commands cannot navigate or dismiss a new one.
    Requests use the latest callback. Shortcut buttons select immediately.

    Each gap button owns its popup: placement uses that button's bounds, expanded
    state identifies only the open gap and closing restores that button's focus.
    Closed gap anchors remain mounted. Ordinary native dismissal rules apply.
    [labels] localizes pagination; chooser labels and range text are English.
    Style overrides affect the navigation and panel respectively. Chooser
    buttons use the theme's [background]/[foreground] pair with accent borders,
    so a light accent does not become a surface beneath light text. *)
val view
  :  ?style:Gpuio.Style.t
  -> ?panel_style:Gpuio.Style.t
  -> ?appearance:Gpuio.Navigation.Appearance.t
  -> ?labels:Gpuio.Navigation.Pagination_labels.t
  -> overlay:Gpuio.Overlay.Config.t
  -> t
  -> Gpuio_bonsai.View.t Or_error.t
