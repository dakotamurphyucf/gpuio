open Core

(** Semantic metadata, independent of password display and edit validation.
    Native exposure and autofill availability depend on backend and OS; a hint
    never promises autofill. Password/New_password require explicit password
    privacy in Text_input.Config. *)
type t =
  | Name
  | Name_prefix
  | Given_name
  | Middle_name
  | Family_name
  | Name_suffix
  | Nickname
  | Job_title
  | Organization_name
  | Location
  | Full_street_address
  | Street_address_line1
  | Street_address_line2
  | Address_city
  | Address_state
  | Address_city_and_state
  | Sublocality
  | Country_name
  | Postal_code
  | Telephone_number
  | Email_address
  | Url
  | Credit_card_number
  | Credit_card_name
  | Credit_card_given_name
  | Credit_card_middle_name
  | Credit_card_family_name
  | Credit_card_security_code
  | Credit_card_expiration
  | Credit_card_expiration_month
  | Credit_card_expiration_year
  | Credit_card_type
  | Username
  | Password
  | New_password
  | One_time_code
  | Shipment_tracking_number
  | Flight_number
  | Date_time
  | Birthdate
  | Birthdate_day
  | Birthdate_month
  | Birthdate_year
  | Cellular_eid
  | Cellular_imei
[@@deriving equal, sexp_of]

module Expert : sig
  val to_wire : t -> Gpuio_protocol.Input_content_hint_wire.t
  val is_password : t -> bool
end

(** A reply describes metadata at native query execution, not guaranteed autofill.
    The hint is included so a delayed reply cannot be mistaken for a newer config. *)
module Status : sig
  type hint = t [@@deriving equal, sexp_of]

  module Unavailability : sig
    type t =
      | Backend
      | Mapping
      | Native_view
    [@@deriving equal, sexp_of]
  end

  (** [Inactive] means no configured hint or no eligible editable focus.
      [Exposed] means this editor owns the current native property.
      [Unavailable] distinguishes a missing backend, unmapped hint and
      incompatible native view. Every payload describes query execution. *)
  type t =
    | Inactive of hint option
    | Exposed of hint
    | Unavailable of hint * Unavailability.t
  [@@deriving equal, sexp_of]

  module Expert : sig
    val of_wire : Gpuio_protocol.Input_content_hint_status_wire.t -> t
  end
end
