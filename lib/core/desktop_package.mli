open Core

(** Generated application metadata. This module performs no I/O, installation or
    default-handler changes. Use Eio or packaging tools to write the artifact. *)
type t

val file_name : t -> string
val contents : t -> string

(** An application entry named [Identity.identifier ^ ".desktop"]. Executes the
    absolute [executable] with [arguments], then [--open-uris] and a separate [%U]
    field. Applications must explicitly parse those arguments and pass the links
    to [App.run_desktop]. Declares the identity's schemes as MIME handlers.

    The Desktop Entry Exec string requires printable ASCII. The executable may
    not contain [=]; use an ASCII installation path or launcher when necessary.
    At most 32 arguments of 1024 bytes each. Literal percent signs and reserved
    characters are escaped, without a shell. Display names remain UTF-8.
    Executable paths containing [%] use [/usr/bin/env --] as a launcher: GIO
    checks the first executable before expanding percent escapes. The launcher
    replaces itself with the application and preserves its arguments/environment.
    Does not declare standard D-Bus activation: GPUIO's forwarding protocol is
    independent of org.freedesktop.Application. *)
val linux_entry
  :  Desktop.Identity.t
  -> executable:File_path.t
  -> ?arguments:string list
  -> unit
  -> t Or_error.t

(** Info.plist for a macOS application bundle using the identity and declared
    schemes. [executable] is a portable basename (letters/digits/dot/hyphen/
    underscore, 1..128 bytes, excluding dot and dot-dot). [version] has three
    numeric components; [build] has one to three, each at most four digits.
    The current pinned backend targets macOS 14.4 or later. Code signing,
    entitlements, icons, distribution and notarization remain packaging concerns. *)
val macos_info_plist
  :  Desktop.Identity.t
  -> executable:string
  -> version:string
  -> build:string
  -> t Or_error.t
