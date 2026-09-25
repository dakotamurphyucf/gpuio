open Core
module Editor = Gpuio_eio.Text_input
module List_view = Gpuio_bonsai.Virtual_list

(** A bounded demo workspace retains three seeded conversation panels per
    window, including tabs that have been closed. Native drafts, selections and
    scroll state survive reopening; closing the window releases every panel. *)
type t

module Panel : sig
  type t =
    { editor : Editor.t
    ; list : int List_view.Output.t
    }
end

val create : icons:Icons.t -> Conversation.t list -> selected:int -> t

val component
  :  t
  -> open_window:(int -> unit)
  -> read_file:(Gpuio.File_path.t -> string)
  -> attachment_directory:Gpuio.File_path.t option
  -> Gpuio_eio.App.Window.t
  -> Bonsai.Cont.graph
  -> Gpuio_bonsai.View.t Bonsai.Cont.t

val install_close_handler : t -> Gpuio_eio.App.Window.t -> unit
val select : t -> int -> unit
val close_tab : t -> int -> unit
val panel : t -> int -> Panel.t option
val set_backend : t -> Conversation.Backend.Config.t -> unit
val dark : t -> bool
val toggle_theme : t -> unit
val notice : t -> string
