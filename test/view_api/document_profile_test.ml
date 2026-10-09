open Core
open Gpuio
module P = Document.Profile
module W = Gpuio_protocol.Wire
module Wire = Gpuio_protocol.Document_profile_wire

let ok = Or_error.ok_exn

let definition () =
  let schema =
    P.Schema.create ~name:"test.reader" ~version:1 ~fingerprint:(String.make 64 'a') |> ok
  in
  let codec =
    P.Codec.bin_prot ~max_bytes:16 Int.bin_t ~validate:(fun n ->
      if n >= 0 then Ok () else Or_error.error_string "negative")
    |> ok
  in
  P.Definition.create ~schema ~properties:codec ~events:codec |> ok
;;

let instance ?(generation = 2L) value =
  P.Instance.create (definition ()) ~generation value |> ok
;;

let event : Wire.Event.t =
  { config_epoch = 1L
  ; instance_generation = 2L
  ; source_revision = 7L
  ; source_generation = 3L
  ; signal = Data "\007"
  }
;;

let hex writer value =
  Bin_prot.Utils.bin_dump writer value
  |> Bigstring.to_string
  |> String.to_list
  |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
  |> String.concat
;;

let%expect_test "paired document profile bytes and bounded typed event decoding" =
  let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok in
  let wire = { (P.Expert.to_wire (instance 0)) with properties = "\000\255\128" } in
  assert (
    String.equal
      (hex
         W.Op.bin_writer_t
         (Set_document_profile (node, { epoch = 1L; instance = Some wire })))
      ("79000101010b746573742e7265616465720140"
       ^ String.concat (List.init 64 ~f:(fun _ -> "61"))
       ^ "020300ff80"));
  assert (
    String.equal
      (hex
         W.Op.bin_writer_t
         (Set_document_profile (node, { epoch = 2L; instance = None })))
      "7900010200");
  let raw =
    "\001\078\000\001\000\001\000\001\004\000\001\001\002\007\003\000\003\000\255\128"
  in
  (match W.Event.decode raw |> ok with
   | [ Document_profile_event (_, _, _, _, _, actual) ] ->
     assert (Wire.Event.equal actual { event with signal = Data "\000\255\128" })
   | _ -> assert false);
  let failed =
    "\001\078\000\001\000\001\000\001\004\000\001\001\002\007\003\001\004\017"
  in
  (match W.Event.decode failed |> ok with
   | [ Document_profile_event (_, _, _, _, _, actual) ] ->
     assert (Wire.Event.equal actual { event with signal = Failed (Input, Invalid_event) })
   | _ -> assert false);
  let profile = instance 0 in
  print_s [%sexp (P.Expert.event profile event |> ok : int P.Event.t)];
  List.iter [ ""; "\007\000"; "\255\255" ] ~f:(fun bytes ->
    match (P.Expert.event profile { event with signal = Data bytes } |> ok).signal with
    | Failed { stage = Input; error = Invalid_event } -> ()
    | _ -> assert false);
  List.iter
    [ { event with config_epoch = 0L }
    ; { event with instance_generation = 1L }
    ; { event with source_revision = 0L }
    ; { event with source_generation = 0L }
    ; { event with signal = Data (String.make 16385 'x') }
    ]
    ~f:(fun event -> assert (Result.is_error (P.Expert.event profile event)));
  assert (Result.is_error (P.Instance.create (definition ()) ~generation:0L 0));
  assert (Result.is_error (P.Instance.create (definition ()) ~generation:1L (-1)));
  [%expect {| ((source_revision 7) (source_generation 3) (signal (Data 7))) |}]
;;

let%expect_test "profile epochs survive clear and fence queued observations" =
  let owner = Text_source.Expert.Owner.create () in
  let source_id = Gpuio_protocol.Resource_id.create ~slot:0L ~generation:1L |> ok in
  let source = Text_source.Expert.handle ~owner source_id in
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let reconciler = Reconciler.create ~document_owner:owner window in
  let view ?(mode = Document.Mode.Markdown) profile tag =
    let view = View.document (Document.Config.create ~source ~mode () |> ok) in
    match profile with
    | None -> view
    | Some profile ->
      View.with_document_profile view profile ~on_event:(fun _ -> tag) |> ok
  in
  assert (
    Result.is_error
      (View.with_document_profile (View.text "bad") (instance 0) ~on_event:Fn.id));
  let code = view ~mode:(Code Document.Language.ocaml) None "code" in
  assert (
    Result.is_error
      (View.with_document_profile code (instance 0) ~on_event:(fun _ -> "bad")));
  let prepare profile tag =
    Reconciler.prepare reconciler ~theme:Theme.default (Some (view profile tag))
  in
  let publish profile tag =
    let update = prepare profile tag |> ok in
    let ops =
      match Reconciler.message update with
      | Some (Apply tx) -> tx.operations
      | None -> []
      | Some _ -> assert false
    in
    Reconciler.accept reconciler update |> ok;
    ops
  in
  let first = publish (Some (instance 1)) "first" in
  let node, handler =
    List.find_map_exn first ~f:(function
      | W.Op.Create (n, _, _, Some h) -> Some (n, h)
      | _ -> None)
  in
  let dispatch ?(source = source_id) ?(revision = 1L) event =
    Reconciler.dispatch
      reconciler
      (W.Event.Document_profile_event (window, node, handler, revision, source, event))
  in
  assert (Option.equal String.equal (dispatch event) (Some "first"));
  assert (List.is_empty (publish (Some (instance 1)) "latest"));
  assert (Option.equal String.equal (dispatch event) (Some "latest"));
  (match publish (Some (instance 2)) "changed" with
   | [ Set_document_profile (same, { epoch = 2L; instance = Some _ }) ] ->
     assert (Gpuio_protocol.Node_id.equal node same)
   | _ -> assert false);
  assert (Option.is_none (dispatch event));
  assert (
    Option.equal String.equal (dispatch { event with config_epoch = 2L }) (Some "changed"));
  assert (Result.is_error (prepare (Some (instance ~generation:1L 1)) "regressed"));
  assert (Option.is_none (dispatch ~revision:99L { event with config_epoch = 2L }));
  let other_source = Gpuio_protocol.Resource_id.create ~slot:1L ~generation:1L |> ok in
  assert (Option.is_none (dispatch ~source:other_source { event with config_epoch = 2L }));
  ignore (publish None "clear");
  assert (Option.is_none (dispatch { event with config_epoch = 2L }));
  let reset = publish (Some (instance ~generation:1L 1)) "reset" in
  let reset_handler =
    List.find_map_exn reset ~f:(function
      | W.Op.Bind (_, Some h) -> Some h
      | _ -> None)
  in
  assert (
    List.exists reset ~f:(function
      | W.Op.Set_document_profile (_, { epoch = 4L; instance = Some i }) ->
        Int64.equal i.generation 1L
      | _ -> false));
  let reset_event = { event with config_epoch = 4L; instance_generation = 1L } in
  let dispatch_reset () =
    Reconciler.dispatch
      reconciler
      (W.Event.Document_profile_event
         (window, node, reset_handler, 4L, source_id, reset_event))
  in
  assert (Option.equal String.equal (dispatch_reset ()) (Some "reset"));
  assert (Option.is_none (dispatch event));
  let update = Reconciler.prepare reconciler ~theme:Theme.default None |> ok in
  Reconciler.accept reconciler update |> ok;
  assert (Option.is_none (dispatch_reset ()));
  print_endline
    "latest callback, property epoch, generation rollback, source/revision, clear and \
     unmount fences";
  [%expect
    {| latest callback, property epoch, generation rollback, source/revision, clear and unmount fences |}]
;;
