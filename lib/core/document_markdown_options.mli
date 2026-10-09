open Core

module Frontmatter : sig
  type t =
    | Disabled
    | Code_block
    | Description_list
  [@@deriving equal, sexp_of]
end

(** Bounded Markdown parser settings; not JavaScript evaluation or a YAML editor.
    Changing options reparses the same source asynchronously, supersedes old work
    and clears selection/link targets when the new interpretation installs. *)
type t [@@deriving equal, sexp_of]

(** Defaults: frontmatter Disabled, mdx false. Code_block displays leading YAML
    frontmatter as code. Description_list renders up to 128 restricted top-level
    plain mappings, including |- and >- values; unsupported forms remain code. MDX enables JSX/expression syntax: unclaimed JSX
    displays
    its children, inline expressions their text and flow expressions code. No
    code executes or imports load. Nondefault options require Markdown mode. *)
val create : ?frontmatter:Frontmatter.t -> ?mdx:bool -> unit -> t

val default : t
val frontmatter : t -> Frontmatter.t
val mdx : t -> bool

module Expert : sig
  val to_wire : t -> Gpuio_protocol.Document_wire.Markdown_options.t
end
