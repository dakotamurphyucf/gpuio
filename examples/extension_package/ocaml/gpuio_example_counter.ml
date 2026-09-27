open Core
module X = Gpuio.Extension

module Properties = struct
  type t =
    { value : int
    ; step : int
    }

  let create ~value ?(step = 1) () =
    if value < 0 || value > 100 || step < 1 || step > 10
    then Or_error.error_string "counter value must be 0..100 and step 1..10"
    else Ok { value; step }
  ;;
end

let schema =
  X.Schema.create
    ~name:"example.counter"
    ~version:1
    ~fingerprint:"ce6beefc974d5b1f01875857c46f2ce31cb2fcfc8e8c3ef3a21716b967f96e15"
  |> Or_error.ok_exn
;;

let value_codec =
  X.Codec.create
    ~max_bytes:1
    ~encode:(fun value ->
      if value < 0 || value > 100
      then Or_error.error_string "counter value outside 0..100"
      else Ok (String.of_char (Char.of_int_exn value)))
    ~decode:(fun bytes ->
      if String.length bytes <> 1 || Char.to_int bytes.[0] > 100
      then Or_error.error_string "invalid native counter value"
      else Ok (Char.to_int bytes.[0]))
  |> Or_error.ok_exn
;;

let properties_codec =
  X.Codec.create
    ~max_bytes:2
    ~encode:(fun (t : Properties.t) ->
      Ok (String.of_char_list [ Char.of_int_exn t.value; Char.of_int_exn t.step ]))
    ~decode:(fun bytes ->
      if String.length bytes <> 2
      then Or_error.error_string "invalid counter properties"
      else
        Properties.create ~value:(Char.to_int bytes.[0]) ~step:(Char.to_int bytes.[1]) ())
  |> Or_error.ok_exn
;;

let definition =
  X.Definition.create
    ~schema
    ~properties:properties_codec
    ~commands:value_codec
    ~events:value_codec
  |> Or_error.ok_exn
;;

let instance properties ~generation ?disabled ?set_value () =
  let%bind.Or_error command =
    match set_value with
    | None -> Ok None
    | Some (sequence, value) ->
      X.Command.create ~sequence value |> Or_error.map ~f:Option.some
  in
  X.Instance.create
    definition
    ~generation
    ~label:"Native counter"
    ?disabled
    ?command
    properties
;;
