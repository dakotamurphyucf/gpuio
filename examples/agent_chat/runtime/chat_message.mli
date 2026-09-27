module Kind : sig
  type t =
    | User
    | Assistant
    | Artifact
end

(** Stateless transcript composition using public presentation helpers. The caller
    owns the document, its revision stream, and all actions inside [content]. *)
val view
  :  kind:Kind.t
  -> dark:bool
  -> icons:(Icons.Name.t * Gpuio.Asset.Handle.t) list
  -> content:Gpuio_bonsai.View.t
  -> Gpuio_bonsai.View.t
