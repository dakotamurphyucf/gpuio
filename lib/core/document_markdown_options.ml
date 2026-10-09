open Core
module Wire = Gpuio_protocol.Document_wire.Markdown_options
module Frontmatter = Wire.Frontmatter

type t = Wire.t [@@deriving equal, sexp_of]

let create ?(frontmatter = Frontmatter.Disabled) ?(mdx = false) () =
  { Wire.frontmatter; mdx }
;;

let default = create ()
let frontmatter (t : t) = t.frontmatter
let mdx (t : t) = t.mdx

module Expert = struct
  let to_wire (t : t) = t
end
