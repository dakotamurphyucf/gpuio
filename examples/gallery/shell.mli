(** Stateless GPUIO layout. Receives current values and effects; owns no Bonsai state or I/O. *)
module Snapshot : sig
  type t =
    { page : Gpuio_gallery_model.Page.t
    ; appearance : Gpuio_gallery_model.Appearance.t
    ; preference : Gpuio_gallery_model.Theme_selection.t
    ; scale : Gpuio_gallery_model.Appearance.Scale.t
    ; window_snapshot : Gpuio.Window.Snapshot.t option
    ; capabilities : Gpuio.Window.Capabilities.t option
    ; custom_chrome : bool
    }
end

module Actions : sig
  type t =
    { select_page : Gpuio_gallery_model.Page.t -> unit Bonsai.Effect.t
    ; toggle_appearance : unit Bonsai.Effect.t
    ; follow_system : unit Bonsai.Effect.t
    ; next_scale : unit Bonsai.Effect.t
    ; open_window : unit Bonsai.Effect.t
    ; toggle_fullscreen : unit Bonsai.Effect.t
    ; minimize : unit Bonsai.Effect.t
    ; zoom : unit Bonsai.Effect.t
    ; close : unit Bonsai.Effect.t
    }
end

val view
  :  palette:Palette.t
  -> snapshot:Snapshot.t
  -> actions:Actions.t
  -> content:Gpuio_bonsai.View.t
  -> Gpuio_bonsai.View.t
