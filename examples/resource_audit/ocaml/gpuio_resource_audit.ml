open Core
module X = Gpuio.Extension

let ok = Or_error.ok_exn

module Properties = struct
  type t =
    { warmups : int
    ; measurements : int
    ; cycle : int
    ; metal_memory : bool
    }

  let validate { warmups; measurements; cycle; metal_memory = _ } =
    if
      warmups < 1
      || warmups > 3
      || measurements < 1
      || measurements > 30
      || cycle < 1
      || cycle > warmups + measurements
    then Or_error.error_string "invalid resource audit configuration"
    else Ok ()
  ;;

  let encode ({ warmups; measurements; cycle; metal_memory } as t) =
    let%map.Or_error () = validate t in
    String.of_char_list
      (List.map
         [ warmups; measurements; cycle; Bool.to_int metal_memory ]
         ~f:Char.of_int_exn)
  ;;

  let decode s =
    match List.map (String.to_list s) ~f:Char.to_int with
    | [ warmups; measurements; cycle; ((0 | 1) as metal_memory) ] ->
      let t = { warmups; measurements; cycle; metal_memory = Int.equal metal_memory 1 } in
      let%map.Or_error () = validate t in
      t
    | _ -> Or_error.error_string "invalid resource audit properties"
  ;;
end

let schema =
  X.Schema.create
    ~name:"qualification.resource_audit"
    ~version:2
    ~fingerprint:"f243c615b4edcba209b0cf330328261d2744cfe36fd3a4cc3bca1f4aef5a0433"
  |> ok
;;

let properties =
  X.Codec.create ~max_bytes:4 ~encode:Properties.encode ~decode:Properties.decode |> ok
;;

let empty_codec =
  X.Codec.create
    ~max_bytes:1
    ~encode:(fun () -> Or_error.error_string "resource audit has no commands/events")
    ~decode:(fun _ -> Or_error.error_string "resource audit has no commands/events")
  |> ok
;;

let definition =
  X.Definition.create ~schema ~properties ~commands:empty_codec ~events:empty_codec |> ok
;;

let instance ~warmups ~measurements ~cycle ~metal_memory =
  X.Instance.create
    definition
    ~generation:1L
    ~label:"Native entity qualification"
    { Properties.warmups; measurements; cycle; metal_memory }
;;

let%expect_test "bounded audit configurations" =
  List.iter
    [ ""
    ; "\000\030\001\000"
    ; "\003\031\001\000"
    ; "\003\030\000\000"
    ; "\003\030\034\000"
    ; "\003\030\001"
    ; "\003\030\001\002"
    ; "\003\030\001\000\000"
    ]
    ~f:(fun s -> assert (Result.is_error (Properties.decode s)));
  List.iter [ "\001\003\001\000"; "\003\030\033\001" ] ~f:(fun s ->
    assert (String.equal s (Properties.encode (Properties.decode s |> ok) |> ok)));
  print_endline "invalid and trailing data rejected; valid properties round-trip";
  [%expect {| invalid and trailing data rejected; valid properties round-trip |}]
;;
