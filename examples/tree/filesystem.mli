open Core

module Entry : sig
  type t =
    { relative : string
    ; kind : Eio.File.Stat.kind
    }
end

val root_id : Gpuio.Tree.Id.t
val id : string -> Gpuio.Tree.Id.t Or_error.t
val initial : unit -> Entry.t Gpuio.Tree.t

(** Read one sorted directory page through the supplied Eio capability. Symlinks
    are leaves. Directory changes between pages may invalidate a page; Reload
    resets the example's source. This is a paging example, not a file watcher. *)
val load
  :  _ Eio.Path.t
  -> Gpuio.Tree_loading.Request.t
  -> Entry.t Gpuio.Tree_loading.Page.t Or_error.t
