open Core
open Gpuio
open Gpuio_protocol

let id = Fn.compose Or_error.ok_exn Choice.Id.of_string

let option ?(disabled = false) key =
  Choice.create ~id:(id key) ~label:key ~disabled () |> Or_error.ok_exn
;;

let config ?(disabled = false) options selected =
  Choice.Config.create
    ~label:"Mode"
    ~options:(Choice.Collection.create options |> Or_error.ok_exn)
    ~selected:(Option.map selected ~f:id)
    ~disabled
    ()
  |> Or_error.ok_exn
;;

let window = Window_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn

let commit t config on_select =
  let update =
    Reconciler.prepare
      t
      ~theme:Theme.default
      (Some (View.radio_group ~config ~on_select ()))
    |> Or_error.ok_exn
  in
  Reconciler.accept t update |> Or_error.ok_exn;
  match Reconciler.message update with
  | Some (Wire.Message.Apply tx) -> tx.operations
  | None -> []
  | Some _ -> assert false
;;

let%expect_test
    "selection events preserve IDs but expire when an option is disabled or removed"
  =
  let t = Reconciler.create window in
  let options = [ option "fast"; option "deep" ] in
  let initial = commit t (config options (Some "fast")) Choice.Id.to_string in
  let node, handler =
    List.find_map_exn initial ~f:(function
      | Wire.Op.Create (node, Radio_group, _, Some handler) -> Some (node, handler)
      | _ -> None)
  in
  let choose key =
    Reconciler.dispatch t (Wire.Event.Choice (window, node, handler, 1L, key))
  in
  print_s [%sexp (choose "deep" : string option)];
  ignore
    (commit
       t
       (config (List.rev options) (Some "fast"))
       (fun id -> "latest:" ^ Choice.Id.to_string id)
     : Wire.Op.t list);
  print_s [%sexp (choose "deep" : string option)];
  ignore
    (commit
       t
       (config [ option "fast"; option ~disabled:true "deep" ] (Some "deep"))
       Choice.Id.to_string
     : Wire.Op.t list);
  assert (Option.is_none (choose "deep"));
  ignore (commit t (config [ option "fast" ] None) Choice.Id.to_string : Wire.Op.t list);
  assert (Option.is_none (choose "deep"));
  assert (Option.is_none (choose "missing"));
  ignore
    (commit t (config ~disabled:true options None) Choice.Id.to_string : Wire.Op.t list);
  assert (Option.is_none (choose "fast"));
  print_endline "disabled and removed options rejected";
  [%expect
    {|
    (deep)
    (latest:deep)
    disabled and removed options rejected
    |}]
;;

let%expect_test
    "configuration validates labels and selected membership; disabled selections are \
     values"
  =
  let options =
    Choice.Collection.create [ option ~disabled:true "deep" ] |> Or_error.ok_exn
  in
  assert (Result.is_error (Choice.Config.create ~label:"" ~options ~selected:None ()));
  assert (
    Result.is_error
      (Choice.Config.create ~label:"Mode" ~options ~selected:(Some (id "missing")) ()));
  let selected =
    Choice.Config.create ~label:"Mode" ~options ~selected:(Some (id "deep")) ()
    |> Or_error.ok_exn
  in
  assert (not (Choice.Config.can_select selected (id "deep")));
  print_s [%sexp (Choice.Config.selected selected : Choice.Id.t option)];
  [%expect {| (deep) |}]
;;

let%expect_test "choice payload decoding rejects malformed IDs" =
  let node = Node_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn in
  let handler = Handler_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn in
  let decode selected =
    let event = Wire.Event.Choice (window, node, handler, 1L, selected) in
    Bin_prot.Utils.bin_dump [%bin_writer: Wire.Event.t list] [ event ]
    |> Bigstring.to_string
    |> Wire.Event.decode
  in
  List.iter
    [ ""; "\255"; "a\000b"; String.make 257 'x' ]
    ~f:(fun id -> assert (Result.is_error (decode id)));
  assert (Result.is_ok (decode "深い"));
  print_endline "CHOICE_PAYLOAD_PASS";
  [%expect {| CHOICE_PAYLOAD_PASS |}]
;;

let%expect_test "select reuses choice routing and replaces a keyed radio identity" =
  let t = Reconciler.create window in
  let options = [ option "fast"; option "deep" ] in
  let c = config options (Some "fast") in
  let before = commit t c Choice.Id.to_string in
  let old_node, old_handler =
    List.find_map_exn before ~f:(function
      | Wire.Op.Create (node, Radio_group, _, Some handler) -> Some (node, handler)
      | _ -> None)
  in
  let update =
    Reconciler.prepare
      t
      ~theme:Theme.default
      (Some (View.select ~config:c ~on_select:Choice.Id.to_string ()))
    |> Or_error.ok_exn
  in
  Reconciler.accept t update |> Or_error.ok_exn;
  let operations =
    match Reconciler.message update with
    | Some (Wire.Message.Apply tx) -> tx.operations
    | None | Some _ -> assert false
  in
  let node, handler =
    List.find_map_exn operations ~f:(function
      | Wire.Op.Create (node, Select, _, Some handler) -> Some (node, handler)
      | _ -> None)
  in
  assert (not (Node_id.equal old_node node));
  assert (
    Option.is_none
      (Reconciler.dispatch
         t
         (Wire.Event.Choice (window, old_node, old_handler, 1L, "deep"))));
  print_s
    [%sexp
      (Reconciler.dispatch t (Wire.Event.Choice (window, node, handler, 2L, "deep"))
       : string option)];
  print_endline "radio replacement expires old selection routing";
  [%expect
    {|
    (deep)
    radio replacement expires old selection routing
  |}]
;;
