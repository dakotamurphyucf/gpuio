open Core
module X = Gpuio.Extension

let ok = Or_error.ok_exn

module Properties = struct
  type t =
    { warmups : int
    ; measurements : int
    ; cycle : int
    ; metal_memory : bool
    ; presentation : bool
    }

  let validate { warmups; measurements; cycle; metal_memory = _; presentation = _ } =
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

  let encode ({ warmups; measurements; cycle; metal_memory; presentation } as t) =
    let%map.Or_error () = validate t in
    String.of_char_list
      (List.map
         [ warmups
         ; measurements
         ; cycle
         ; Bool.to_int metal_memory
         ; Bool.to_int presentation
         ]
         ~f:Char.of_int_exn)
  ;;

  let decode s =
    match List.map (String.to_list s) ~f:Char.to_int with
    | [ warmups
      ; measurements
      ; cycle
      ; ((0 | 1) as metal_memory)
      ; ((0 | 1) as presentation)
      ] ->
      let t =
        { warmups
        ; measurements
        ; cycle
        ; metal_memory = Int.equal metal_memory 1
        ; presentation = Int.equal presentation 1
        }
      in
      let%map.Or_error () = validate t in
      t
    | _ -> Or_error.error_string "invalid resource audit properties"
  ;;
end

let schema =
  X.Schema.create
    ~name:"qualification.resource_audit"
    ~version:3
    ~fingerprint:"245fc08e5aa950f642710e391572b120743a54eece59e0c09345cfeacda6cc83"
  |> ok
;;

let properties =
  X.Codec.create ~max_bytes:5 ~encode:Properties.encode ~decode:Properties.decode |> ok
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

let instance ~warmups ~measurements ~cycle ~metal_memory ~presentation =
  X.Instance.create
    definition
    ~generation:1L
    ~label:"Native entity qualification"
    { Properties.warmups; measurements; cycle; metal_memory; presentation }
;;

let%expect_test "bounded audit configurations" =
  List.iter
    [ []
    ; [ 0; 30; 1; 0; 0 ]
    ; [ 3; 31; 1; 0; 0 ]
    ; [ 3; 30; 0; 0; 0 ]
    ; [ 3; 30; 34; 0; 0 ]
    ; [ 3; 30; 1; 0 ]
    ; [ 3; 30; 1; 2; 0 ]
    ; [ 3; 30; 1; 0; 2 ]
    ; [ 3; 30; 1; 0; 0; 0 ]
    ]
    ~f:(fun bytes ->
      let s = String.of_char_list (List.map bytes ~f:Char.of_int_exn) in
      assert (Result.is_error (Properties.decode s)));
  List.iter
    [ [ 1; 3; 1; 0; 0 ]; [ 3; 30; 33; 1; 1 ] ]
    ~f:(fun bytes ->
      let s = String.of_char_list (List.map bytes ~f:Char.of_int_exn) in
      assert (String.equal s (Properties.encode (Properties.decode s |> ok) |> ok)));
  print_endline "invalid and trailing data rejected; valid properties round-trip";
  [%expect {| invalid and trailing data rejected; valid properties round-trip |}]
;;
