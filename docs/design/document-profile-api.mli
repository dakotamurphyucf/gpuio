(** Historical design entry point. The checked interface now lives in
    [lib/core/document_profile.mli]; do not maintain a second divergent API here.
    The module is exported, but native runtime/gallery attachment remains WIP. *)
include module type of Gpuio.Document.Profile

(** Only Markdown/HTML DocumentViews accept a profile. *)
val with_document_profile
  :  'message Gpuio.View.t
  -> 'event Instance.t
  -> on_event:('event Event.t -> 'message)
  -> 'message Gpuio.View.t Core.Or_error.t
