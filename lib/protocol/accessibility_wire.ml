open Core

module Role = struct
  type t =
    | Group
    | Label
    | Link
    | Separator
    | Description_list
    | Term
    | Definition
    | Status
    | Alert
    | Image
    | Heading of int
    | Navigation
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Heading level -> level >= 1 && level <= 6
    | Group
    | Label
    | Link
    | Separator
    | Description_list
    | Term
    | Definition
    | Status
    | Alert
    | Image
    | Navigation -> true
  ;;
end

module Current = struct
  type t =
    | True
    | Page
    | Step
    | Location
    | Date
    | Time
  [@@deriving bin_io, equal, sexp_of]
end

module Live = struct
  type t =
    | Off
    | Polite
    | Assertive
  [@@deriving bin_io, equal, sexp_of]
end

let valid_text text =
  (not (String.is_empty text))
  && String.length text <= 4096
  && Stdlib.String.is_valid_utf_8 text
  && not (String.contains text '\000')
;;

module Field = struct
  type t =
    { label : string
    ; help : string option
    ; error : string option
    ; required : bool
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    valid_text t.label
    && Option.for_all t.help ~f:valid_text
    && Option.for_all t.error ~f:valid_text
  ;;
end

module Config = struct
  type t =
    { role : Role.t option
    ; label : string option
    ; description : string option
    ; live : Live.t
    ; field : Field.t option
    ; current : Current.t option
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Option.for_all t.role ~f:Role.valid
    && Option.for_all t.label ~f:valid_text
    && Option.for_all t.description ~f:valid_text
    && (Option.is_none t.current || Option.is_some t.description)
    && Option.for_all t.field ~f:(fun field ->
      Field.valid field
      && Option.is_none t.current
      && Option.is_none t.role
      && Option.is_none t.label
      && Option.is_none t.description
      && Live.equal t.live Off)
  ;;
end
