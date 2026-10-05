open Core
module X = Gpuio.Extension

let ok = Or_error.ok_exn

module Properties = struct
  type t =
    { warmups : int
    ; measurements : int
    ; cycle : int
    }

  let validate { warmups; measurements; cycle } =
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

  let encode ({ warmups; measurements; cycle } as t) =
    let%map.Or_error () = validate t in
    String.of_char_list (List.map [ warmups; measurements; cycle ] ~f:Char.of_int_exn)
  ;;

  let decode s =
    match List.map (String.to_list s) ~f:Char.to_int with
    | [ warmups; measurements; cycle ] ->
      let t = { warmups; measurements; cycle } in
      let%map.Or_error () = validate t in
      t
    | _ -> Or_error.error_string "invalid resource audit properties"
  ;;
end

let schema =
  X.Schema.create
    ~name:"qualification.resource_audit"
    ~version:1
    ~fingerprint:"b3406256732852686166dd465bfbfa1ed3db612a4f05f0820919fcab4c6679e4"
  |> ok
;;

let properties =
  X.Codec.create ~max_bytes:3 ~encode:Properties.encode ~decode:Properties.decode |> ok
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

let instance ~warmups ~measurements ~cycle =
  X.Instance.create
    definition
    ~generation:1L
    ~label:"Native entity qualification"
    { Properties.warmups; measurements; cycle }
;;

let%expect_test "bounded audit configurations" =
  List.iter
    [ ""
    ; "\000\030\001"
    ; "\003\031\001"
    ; "\003\030\000"
    ; "\003\030\034"
    ; "\003\030\001\000"
    ]
    ~f:(fun s -> assert (Result.is_error (Properties.decode s)));
  List.iter [ "\001\003\001"; "\003\030\033" ] ~f:(fun s ->
    assert (String.equal s (Properties.encode (Properties.decode s |> ok) |> ok)));
  print_endline "invalid and trailing data rejected; valid properties round-trip";
  [%expect {| invalid and trailing data rejected; valid properties round-trip |}]
;;
