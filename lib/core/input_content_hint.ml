open Core

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

module Expert = struct
  let is_password = function
    | Password | New_password -> true
    | _ -> false
  ;;

  let to_wire = function
    | Name -> Gpuio_protocol.Input_content_hint_wire.Name
    | Name_prefix -> Gpuio_protocol.Input_content_hint_wire.Name_prefix
    | Given_name -> Gpuio_protocol.Input_content_hint_wire.Given_name
    | Middle_name -> Gpuio_protocol.Input_content_hint_wire.Middle_name
    | Family_name -> Gpuio_protocol.Input_content_hint_wire.Family_name
    | Name_suffix -> Gpuio_protocol.Input_content_hint_wire.Name_suffix
    | Nickname -> Gpuio_protocol.Input_content_hint_wire.Nickname
    | Job_title -> Gpuio_protocol.Input_content_hint_wire.Job_title
    | Organization_name -> Gpuio_protocol.Input_content_hint_wire.Organization_name
    | Location -> Gpuio_protocol.Input_content_hint_wire.Location
    | Full_street_address -> Gpuio_protocol.Input_content_hint_wire.Full_street_address
    | Street_address_line1 -> Gpuio_protocol.Input_content_hint_wire.Street_address_line1
    | Street_address_line2 -> Gpuio_protocol.Input_content_hint_wire.Street_address_line2
    | Address_city -> Gpuio_protocol.Input_content_hint_wire.Address_city
    | Address_state -> Gpuio_protocol.Input_content_hint_wire.Address_state
    | Address_city_and_state ->
      Gpuio_protocol.Input_content_hint_wire.Address_city_and_state
    | Sublocality -> Gpuio_protocol.Input_content_hint_wire.Sublocality
    | Country_name -> Gpuio_protocol.Input_content_hint_wire.Country_name
    | Postal_code -> Gpuio_protocol.Input_content_hint_wire.Postal_code
    | Telephone_number -> Gpuio_protocol.Input_content_hint_wire.Telephone_number
    | Email_address -> Gpuio_protocol.Input_content_hint_wire.Email_address
    | Url -> Gpuio_protocol.Input_content_hint_wire.Url
    | Credit_card_number -> Gpuio_protocol.Input_content_hint_wire.Credit_card_number
    | Credit_card_name -> Gpuio_protocol.Input_content_hint_wire.Credit_card_name
    | Credit_card_given_name ->
      Gpuio_protocol.Input_content_hint_wire.Credit_card_given_name
    | Credit_card_middle_name ->
      Gpuio_protocol.Input_content_hint_wire.Credit_card_middle_name
    | Credit_card_family_name ->
      Gpuio_protocol.Input_content_hint_wire.Credit_card_family_name
    | Credit_card_security_code ->
      Gpuio_protocol.Input_content_hint_wire.Credit_card_security_code
    | Credit_card_expiration ->
      Gpuio_protocol.Input_content_hint_wire.Credit_card_expiration
    | Credit_card_expiration_month ->
      Gpuio_protocol.Input_content_hint_wire.Credit_card_expiration_month
    | Credit_card_expiration_year ->
      Gpuio_protocol.Input_content_hint_wire.Credit_card_expiration_year
    | Credit_card_type -> Gpuio_protocol.Input_content_hint_wire.Credit_card_type
    | Username -> Gpuio_protocol.Input_content_hint_wire.Username
    | Password -> Gpuio_protocol.Input_content_hint_wire.Password
    | New_password -> Gpuio_protocol.Input_content_hint_wire.New_password
    | One_time_code -> Gpuio_protocol.Input_content_hint_wire.One_time_code
    | Shipment_tracking_number ->
      Gpuio_protocol.Input_content_hint_wire.Shipment_tracking_number
    | Flight_number -> Gpuio_protocol.Input_content_hint_wire.Flight_number
    | Date_time -> Gpuio_protocol.Input_content_hint_wire.Date_time
    | Birthdate -> Gpuio_protocol.Input_content_hint_wire.Birthdate
    | Birthdate_day -> Gpuio_protocol.Input_content_hint_wire.Birthdate_day
    | Birthdate_month -> Gpuio_protocol.Input_content_hint_wire.Birthdate_month
    | Birthdate_year -> Gpuio_protocol.Input_content_hint_wire.Birthdate_year
    | Cellular_eid -> Gpuio_protocol.Input_content_hint_wire.Cellular_eid
    | Cellular_imei -> Gpuio_protocol.Input_content_hint_wire.Cellular_imei
  ;;

  let of_wire = function
    | Gpuio_protocol.Input_content_hint_wire.Name -> Name
    | Gpuio_protocol.Input_content_hint_wire.Name_prefix -> Name_prefix
    | Gpuio_protocol.Input_content_hint_wire.Given_name -> Given_name
    | Gpuio_protocol.Input_content_hint_wire.Middle_name -> Middle_name
    | Gpuio_protocol.Input_content_hint_wire.Family_name -> Family_name
    | Gpuio_protocol.Input_content_hint_wire.Name_suffix -> Name_suffix
    | Gpuio_protocol.Input_content_hint_wire.Nickname -> Nickname
    | Gpuio_protocol.Input_content_hint_wire.Job_title -> Job_title
    | Gpuio_protocol.Input_content_hint_wire.Organization_name -> Organization_name
    | Gpuio_protocol.Input_content_hint_wire.Location -> Location
    | Gpuio_protocol.Input_content_hint_wire.Full_street_address -> Full_street_address
    | Gpuio_protocol.Input_content_hint_wire.Street_address_line1 -> Street_address_line1
    | Gpuio_protocol.Input_content_hint_wire.Street_address_line2 -> Street_address_line2
    | Gpuio_protocol.Input_content_hint_wire.Address_city -> Address_city
    | Gpuio_protocol.Input_content_hint_wire.Address_state -> Address_state
    | Gpuio_protocol.Input_content_hint_wire.Address_city_and_state ->
      Address_city_and_state
    | Gpuio_protocol.Input_content_hint_wire.Sublocality -> Sublocality
    | Gpuio_protocol.Input_content_hint_wire.Country_name -> Country_name
    | Gpuio_protocol.Input_content_hint_wire.Postal_code -> Postal_code
    | Gpuio_protocol.Input_content_hint_wire.Telephone_number -> Telephone_number
    | Gpuio_protocol.Input_content_hint_wire.Email_address -> Email_address
    | Gpuio_protocol.Input_content_hint_wire.Url -> Url
    | Gpuio_protocol.Input_content_hint_wire.Credit_card_number -> Credit_card_number
    | Gpuio_protocol.Input_content_hint_wire.Credit_card_name -> Credit_card_name
    | Gpuio_protocol.Input_content_hint_wire.Credit_card_given_name ->
      Credit_card_given_name
    | Gpuio_protocol.Input_content_hint_wire.Credit_card_middle_name ->
      Credit_card_middle_name
    | Gpuio_protocol.Input_content_hint_wire.Credit_card_family_name ->
      Credit_card_family_name
    | Gpuio_protocol.Input_content_hint_wire.Credit_card_security_code ->
      Credit_card_security_code
    | Gpuio_protocol.Input_content_hint_wire.Credit_card_expiration ->
      Credit_card_expiration
    | Gpuio_protocol.Input_content_hint_wire.Credit_card_expiration_month ->
      Credit_card_expiration_month
    | Gpuio_protocol.Input_content_hint_wire.Credit_card_expiration_year ->
      Credit_card_expiration_year
    | Gpuio_protocol.Input_content_hint_wire.Credit_card_type -> Credit_card_type
    | Gpuio_protocol.Input_content_hint_wire.Username -> Username
    | Gpuio_protocol.Input_content_hint_wire.Password -> Password
    | Gpuio_protocol.Input_content_hint_wire.New_password -> New_password
    | Gpuio_protocol.Input_content_hint_wire.One_time_code -> One_time_code
    | Gpuio_protocol.Input_content_hint_wire.Shipment_tracking_number ->
      Shipment_tracking_number
    | Gpuio_protocol.Input_content_hint_wire.Flight_number -> Flight_number
    | Gpuio_protocol.Input_content_hint_wire.Date_time -> Date_time
    | Gpuio_protocol.Input_content_hint_wire.Birthdate -> Birthdate
    | Gpuio_protocol.Input_content_hint_wire.Birthdate_day -> Birthdate_day
    | Gpuio_protocol.Input_content_hint_wire.Birthdate_month -> Birthdate_month
    | Gpuio_protocol.Input_content_hint_wire.Birthdate_year -> Birthdate_year
    | Gpuio_protocol.Input_content_hint_wire.Cellular_eid -> Cellular_eid
    | Gpuio_protocol.Input_content_hint_wire.Cellular_imei -> Cellular_imei
  ;;
end

module Status = struct
  type hint = t [@@deriving equal, sexp_of]

  module Unavailability = struct
    type t =
      | Backend
      | Mapping
      | Native_view
    [@@deriving equal, sexp_of]
  end

  type t =
    | Inactive of hint option
    | Exposed of hint
    | Unavailable of hint * Unavailability.t
  [@@deriving equal, sexp_of]

  let of_wire = function
    | Gpuio_protocol.Input_content_hint_status_wire.Inactive hint ->
      Inactive (Option.map hint ~f:Expert.of_wire)
    | Exposed hint -> Exposed (Expert.of_wire hint)
    | Unavailable (hint, reason) ->
      let reason =
        match reason with
        | Gpuio_protocol.Input_content_hint_status_wire.Unavailability.Backend ->
          Unavailability.Backend
        | Mapping -> Unavailability.Mapping
        | Native_view -> Unavailability.Native_view
      in
      Unavailable (Expert.of_wire hint, reason)
  ;;

  module Expert = struct
    let of_wire = of_wire
  end
end
