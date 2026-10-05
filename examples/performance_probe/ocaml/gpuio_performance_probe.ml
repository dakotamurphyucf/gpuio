open Core
module X = Gpuio.Extension

module Command = struct
  type t =
    | Begin
    | Finish
    | Document_preparation
    | Buckets of
        { metric : int
        ; offset : int
        }
end

module Event = struct
  type t =
    | Begun of int64
    | Document_preparation of
        { queue_us : int64
        ; configure_us : int64
        ; parse_us : int64
        ; highlight_us : int64
        ; search_us : int64
        ; source_bytes : int64
        ; completed : int64
        ; discarded : int64
        ; peak_workers : int64
        ; peak_reserved_bytes : int64
        }
    | Finished of
        { elapsed_ns : int64
        ; capture_ns : int64
        ; dropped_inputs : int64
        ; counts : int64 list
        }
    | Buckets of
        { metric : int
        ; offset : int
        ; total : int
        ; values : (int64 * int64) list
        }
  [@@deriving sexp]
end

let ok = Or_error.ok_exn

let schema =
  X.Schema.create
    ~name:"qualification.performance"
    ~version:2
    ~fingerprint:"4af1fe7a3cc0e8d5ad9f52555fd37b5634e93a9a2b0050f739cdcd6626156889"
  |> ok
;;

let properties =
  X.Codec.create
    ~max_bytes:1
    ~encode:(fun () -> Ok "\000")
    ~decode:(fun s ->
      if String.equal s "\000"
      then Ok ()
      else Or_error.error_string "invalid probe properties")
  |> ok
;;

let encode_command = function
  | Command.Begin -> Ok "\000"
  | Finish -> Ok "\001"
  | Document_preparation -> Ok "\003"
  | Buckets { metric; offset } ->
    if metric < 0 || metric > 4 || offset < 0 || offset > 4_294_967_295
    then Or_error.error_string "probe bucket request out of range"
    else Ok ("\002" ^ String.of_char (Char.of_int_exn metric) ^ Int.to_string offset)
;;

let decode_command s =
  match String.to_list s with
  | [ '\000' ] -> Ok Command.Begin
  | [ '\001' ] -> Ok Command.Finish
  | [ '\003' ] -> Ok Command.Document_preparation
  | '\002' :: metric :: (_ :: _ as digits) ->
    Or_error.try_with (fun () ->
      let metric = Char.to_int metric in
      let offset = Int.of_string (String.of_char_list digits) in
      let command = Command.Buckets { metric; offset } in
      let encoded = encode_command command |> ok in
      if not (String.equal encoded s) then failwith "noncanonical probe command";
      command)
  | _ -> Or_error.error_string "invalid probe command"
;;

let commands =
  X.Codec.create ~max_bytes:12 ~encode:encode_command ~decode:decode_command |> ok
;;

let events =
  X.Codec.create
    ~max_bytes:16384
    ~encode:(fun e -> Ok (Sexp.to_string (Event.sexp_of_t e)))
    ~decode:(fun s -> Or_error.try_with (fun () -> Event.t_of_sexp (Sexp.of_string s)))
  |> ok
;;

let definition = X.Definition.create ~schema ~properties ~commands ~events |> ok

let instance ~sequence value =
  let%bind.Or_error command = X.Command.create ~sequence value in
  X.Instance.create
    definition
    ~generation:1L
    ~label:"Qualification measurements"
    ~command
    ()
;;

let%expect_test "native event shapes and bounded commands" =
  List.iter
    [ "(Begun 17)"
    ; "(Finished (elapsed_ns 1000) (capture_ns 20) (dropped_inputs 0) (counts (2 2 0 0 \
       0)))"
    ; "(Buckets (metric 0) (offset 0) (total 2) (values ((100 1) (200 1))))"
    ]
    ~f:(fun bytes ->
      let event = X.Codec.decode events bytes |> ok in
      print_s (Event.sexp_of_t event));
  List.iter [ "\000"; "\001"; "\003"; "\002\0044294967295" ] ~f:(fun bytes ->
    let command = X.Codec.decode commands bytes |> ok in
    assert (String.equal bytes (X.Codec.encode commands command |> ok)));
  List.iter
    [ ""
    ; "\000\000"
    ; "\001\000"
    ; "\003\000"
    ; "\002\0050"
    ; "\002\000"
    ; "\002\000-1"
    ; "\002\00000"
    ; "\002\0004294967296"
    ; "\002\0001x"
    ]
    ~f:(fun bytes -> assert (Result.is_error (X.Codec.decode commands bytes)));
  [%expect
    {|
    (Begun 17)
    (Finished (elapsed_ns 1000) (capture_ns 20) (dropped_inputs 0)
     (counts (2 2 0 0 0)))
    (Buckets (metric 0) (offset 0) (total 2) (values ((100 1) (200 1))))
    |}]
;;

let%expect_test "document preparation retains cumulative worker units and peaks" =
  let bytes =
    "(Document_preparation (queue_us 12) (configure_us 23) (parse_us 34) (highlight_us \
     45) (search_us 56) (source_bytes 8388608) (completed 7) (discarded 2) (peak_workers \
     4) (peak_reserved_bytes 67108864))"
  in
  let event = X.Codec.decode events bytes |> ok in
  (match event with
   | Event.Document_preparation
       { queue_us
       ; configure_us
       ; parse_us
       ; highlight_us
       ; search_us
       ; source_bytes
       ; completed
       ; discarded
       ; peak_workers
       ; peak_reserved_bytes
       } ->
     print_s
       [%sexp
         ([ queue_us; configure_us; parse_us; highlight_us; search_us ] : int64 list)
       , ([ source_bytes; completed; discarded; peak_workers; peak_reserved_bytes ]
          : int64 list)]
   | event -> raise_s [%sexp (event : Event.t)]);
  [%expect {| ((12 23 34 45 56) (8388608 7 2 4 67108864)) |}]
;;
