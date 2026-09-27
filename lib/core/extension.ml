open Core

module Schema = struct
  type t =
    { name : string
    ; version : int
    ; fingerprint : string
    }
  [@@deriving equal, sexp_of]

  let lower c = Char.(c >= 'a' && c <= 'z')
  let digit c = Char.(c >= '0' && c <= '9')

  let create ~name ~version ~fingerprint =
    let segments = String.split name ~on:'.' in
    let valid_segment segment =
      (not (String.is_empty segment))
      && lower segment.[0]
      && String.for_all segment ~f:(fun c -> lower c || digit c || Char.equal c '_')
    in
    if
      String.length name > 128
      || List.length segments < 2
      || not (List.for_all segments ~f:valid_segment)
    then Or_error.error_string "invalid qualified component name"
    else if version < 1 || version > 65535
    then Or_error.error_string "component version must be in 1..65535"
    else if
      String.length fingerprint <> 64
      || not
           (String.for_all fingerprint ~f:(fun c ->
              digit c || Char.(c >= 'a' && c <= 'f')))
    then Or_error.error_string "schema fingerprint must be 64 lowercase hex characters"
    else Ok { name; version; fingerprint }
  ;;

  let name t = t.name
  let version t = t.version
  let fingerprint t = t.fingerprint

  module Expert = struct
    let to_wire t : Gpuio_protocol.Extension_wire.Schema.t =
      { name = t.name; version = Int64.of_int t.version; fingerprint = t.fingerprint }
    ;;

    let of_wire (t : Gpuio_protocol.Extension_wire.Schema.t) =
      if Gpuio_protocol.Extension_wire.Schema.valid t
      then
        create
          ~name:t.name
          ~version:(Int64.to_int_exn t.version)
          ~fingerprint:t.fingerprint
      else Or_error.error_string "invalid native extension schema"
    ;;
  end
end

module Codec = struct
  type 'a t =
    { max_bytes : int
    ; encode : 'a -> string Or_error.t
    ; decode : string -> 'a Or_error.t
    }

  let create ~max_bytes ~encode ~decode =
    if max_bytes < 1 || max_bytes > 65536
    then Or_error.error_string "extension codec bound must be in 1..65536"
    else Ok { max_bytes; encode; decode }
  ;;

  let max_bytes t = t.max_bytes
  let contain f = Or_error.try_with f |> Or_error.join

  let encode t value =
    let%bind.Or_error bytes = contain (fun () -> t.encode value) in
    if String.length bytes > t.max_bytes
    then Or_error.error_string "extension encoded payload exceeds bound"
    else Ok bytes
  ;;

  let decode t bytes =
    if String.length bytes > t.max_bytes
    then Or_error.error_string "extension payload exceeds bound"
    else contain (fun () -> t.decode bytes)
  ;;

  let bin_prot ~max_bytes (bin : 'a Bin_prot.Type_class.t) ~validate =
    let encode value =
      let%bind.Or_error () = validate value in
      let length = bin.writer.size value in
      if length < 0 || length > max_bytes
      then Or_error.error_string "extension encoded payload exceeds bound"
      else (
        let buffer = Bigstring.create length in
        let written = bin.writer.write buffer ~pos:0 value in
        if written <> length
        then Or_error.error_string "extension writer size mismatch"
        else Ok (Bigstring.to_string buffer))
    in
    let decode bytes =
      let buffer = Bigstring.of_string bytes in
      let pos_ref = ref 0 in
      let value = bin.reader.read buffer ~pos_ref in
      if !pos_ref <> String.length bytes
      then Or_error.error_string "trailing extension payload bytes"
      else (
        let%map.Or_error () = validate value in
        value)
    in
    create ~max_bytes ~encode ~decode
  ;;
end

module Error = Gpuio_protocol.Extension_wire.Error

module Event = struct
  type 'a t =
    | Data of 'a
    | Mounted
    | Command_completed of int64
    | Failed of Error.t
  [@@deriving sexp_of]
end

module Definition = struct
  type ('properties, 'command, 'event) t =
    { schema : Schema.t
    ; properties : 'properties Codec.t
    ; commands : 'command Codec.t
    ; events : 'event Codec.t
    }

  let create ~schema ~properties ~commands ~events =
    if
      Codec.max_bytes commands > Gpuio_protocol.Extension_wire.max_message
      || Codec.max_bytes events > Gpuio_protocol.Extension_wire.max_message
    then Or_error.error_string "extension command/event codec exceeds 16KiB"
    else Ok { schema; properties; commands; events }
  ;;

  let schema t = t.schema
end

module Command = struct
  type 'a t =
    { sequence : int64
    ; value : 'a
    }

  let create ~sequence value =
    if Int64.(sequence <= 0L)
    then Or_error.error_string "extension command sequence must be positive"
    else Ok { sequence; value }
  ;;
end

module Instance = struct
  module Wire = Gpuio_protocol.Extension_wire

  type 'event t =
    { config : Wire.Config.t
    ; events : 'event Codec.t
    }

  let create
        (definition : (_, _, _) Definition.t)
        ~generation
        ~label
        ?(disabled = false)
        ?command
        properties
    =
    let%bind.Or_error properties = Codec.encode definition.properties properties in
    let%bind.Or_error command =
      match command with
      | None -> Ok None
      | Some (command : _ Command.t) ->
        let%map.Or_error payload = Codec.encode definition.commands command.value in
        Some ({ sequence = command.sequence; payload } : Wire.Command.t)
    in
    let config : Wire.Config.t =
      { schema = Schema.Expert.to_wire definition.schema
      ; generation
      ; label
      ; disabled
      ; properties
      ; command
      }
    in
    if Wire.Config.valid config
    then Ok { config; events = definition.events }
    else Or_error.error_string "invalid extension instance"
  ;;

  module Expert = struct
    let to_wire t = t.config

    let event t = function
      | Wire.Signal.Data bytes ->
        (match Codec.decode t.events bytes with
         | Ok value -> Event.Data value
         | Error _ -> Event.Failed Invalid_event)
      | Mounted -> Event.Mounted
      | Command_completed sequence -> Event.Command_completed sequence
      | Failed error -> Event.Failed error
    ;;
  end
end
