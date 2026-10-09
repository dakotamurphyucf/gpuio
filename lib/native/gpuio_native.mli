type file_descr = Unix.file_descr

open Core

(** Low-level bridge handle. Create on the OS main thread with a dedicated
    nonblocking wake pipe. The native side duplicates its write descriptor.
    Keep the read end alive until [run] returns and all worker activity ends.
    [submit]/[drain] may run on the single OCaml UI domain while [run] owns the
    OS main thread. [dispose] is idempotent and requires [run] to have returned.
    Rust panics become OCaml exceptions at the export boundary. *)
type t

val create : file_descr -> t

(** Creation policy; false keeps the host alive without windows until shutdown. *)
val create_with_options : exit_on_last_window:bool -> file_descr -> t

val run : t -> unit

(** [Error Closed] means the native mailbox has closed, potentially after the
    caller's most recent [drain]. A final [Stopped] follows earlier queued output.
    Stop submitting and keep draining for ordered completion; do not treat this as
    an acknowledgement for the rejected message or dispose before [run] returns. *)
val submit
  :  t
  -> Gpuio_protocol.Wire.Message.t
  -> (unit, Gpuio_protocol.Wire.Error_code.t) Result.t

val drain : t -> Gpuio_protocol.Wire.Event.t list Or_error.t

module Traffic : sig
  (** Cumulative counters for one handle, sampled on the same UI domain as
      [submit]/[drain]. Counts serialized buffers, not allocator or GPU memory.
      Attempts and attempted bytes include native rejections/retries; accepted
      bytes count successful submissions only. Drains include empty event batches
      (for example clock polling); [received_events] counts decoded events, not polling calls.
      Catalog initialization and emergency abort/dispose calls are excluded. *)
  type t =
    { submission_attempts : int
    ; attempted_bytes : int
    ; submitted_messages : int
    ; submitted_bytes : int
    ; drain_calls : int
    ; drained_bytes : int
    ; received_events : int
    }
  [@@deriving sexp_of]
end

val traffic : t -> Traffic.t

module Command_queue : sig
  (** Accepted native commands awaiting dispatch. Bytes count their serialized
      buffer sizes, not decoded heap memory. Peak bytes is the lifetime high-water
      mark; rejected submissions do not contribute. Pop/close release current
      bytes. This excludes currently executing work and output events. *)
  type t =
    { commands : int
    ; bytes : int
    ; peak_bytes : int
    }
  [@@deriving sexp_of]
end

(** Read-only snapshot under the native mailbox mutex, without a bridge command
    or wake. Like [traffic], call on the UI domain before native disposal. *)
val command_queue : t -> Command_queue.t

(** Emergency wake-and-stop, independent of command queue capacity. Cancels all
    outstanding requests. Used when the OCaml worker fails. *)
val abort : t -> unit

val dispose : t -> unit

(** The immutable, statically linked component schemas. Initializes the selected
    backend and freezes registration. Call on the OS main thread before [run]. *)
val extension_catalog : unit -> Gpuio_protocol.Extension_wire.Schema.t list Or_error.t

(** Immutable static document profile schemas; same initialization/main-thread
    rules as [extension_catalog]. *)
val document_profile_catalog
  :  unit
  -> Gpuio_protocol.Extension_wire.Schema.t list Or_error.t

(** Preflight before [run] or command submission; owns native startup input and
    Linux session-bus lease. Blocking OS work releases the OCaml runtime. *)
val prepare_desktop
  :  t
  -> Gpuio_protocol.Desktop_wire.Launch_request.t
  -> Gpuio_protocol.Desktop_wire.Launch_response.t

(** Pure bounded regex syntax/resource preflight. Copies the bounded request and
    releases the OCaml runtime while compiling. No transport/backend initialization
    or GPUI window is required. Prefer the Eio wrapper to avoid blocking its domain. *)
val prepare_input_regex
  :  Gpuio_protocol.Input_validation_wire.Source.t
  -> Gpuio_protocol.Input_validation_wire.Preparation.t Or_error.t
