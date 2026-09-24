open Core
module Asset = Gpuio.Asset
module Image = Gpuio.Image
module Id = Gpuio_protocol.Resource_id

let%expect_test "equal native slots in different applications cannot alias" =
  let owner = Asset.Expert.Owner.create () in
  let foreign = Asset.Expert.Owner.create () in
  let id = Id.create ~slot:7L ~generation:42L |> Or_error.ok_exn in
  let handle = Asset.Expert.handle ~owner ~id ~format:Png in
  let duplicate = Asset.Expert.handle ~owner ~id ~format:Png in
  let other = Asset.Expert.handle ~owner:foreign ~id ~format:Png in
  let next =
    Asset.Expert.handle
      ~owner
      ~id:(Id.create ~slot:7L ~generation:43L |> Or_error.ok_exn)
      ~format:Png
  in
  let config =
    Image.Config.create ~asset:handle ~description:Image.Description.decorative ()
  in
  print_s
    [%sexp
      { same_registration = (Asset.Handle.equal handle duplicate : bool)
      ; foreign_application = (Asset.Handle.equal handle other : bool)
      ; reused_slot = (Asset.Handle.equal handle next : bool)
      ; local = (Image.Expert.check_owner config ~owner : (unit, Image.Error.t) Result.t)
      ; foreign =
          (Image.Expert.check_owner config ~owner:foreign
           : (unit, Image.Error.t) Result.t)
      ; fit = (Image.Config.fit config : Image.Fit.t)
      }];
  [%expect
    {|
    ((same_registration true) (foreign_application false) (reused_slot false)
     (local (Ok ())) (foreign (Error Wrong_application)) (fit Contain))
    |}]
;;

let%expect_test "decorative and meaningful descriptions are explicit and bounded" =
  print_s [%sexp (Image.Expert.label Image.Description.decorative : string option)];
  List.iter
    [ ""
    ; " \n\t"
    ; "a\000b"
    ; "\255"
    ; "Family 👨‍👩‍👧‍👦"
    ; String.make 4096 'a'
    ; String.make 4097 'a'
    ]
    ~f:(fun label ->
      let result = Image.Description.label label in
      print_s [%sexp (String.length label : int), (Result.is_ok result : bool)];
      Result.iter result ~f:(fun description ->
        assert (Option.equal String.equal (Image.Expert.label description) (Some label))));
  [%expect
    {|
    ()
    (0 false)
    (3 false)
    (3 false)
    (1 false)
    (32 true)
    (4096 true)
    (4097 false)
    |}]
;;

let%expect_test "metadata validates total animation footprint as well as dimensions" =
  List.iter
    [ 0, 1, 1
    ; 1, 1, 0
    ; 1, 1, 120
    ; 1, 1, 121
    ; 16384, 1, 1
    ; 16385, 1, 1
    ; 4096, 4096, 1
    ; 4096, 4096, 2
    ; 16384, 16384, 120
    ; Int.max_value, 1, 1
    ]
    ~f:(fun (width_px, height_px, frames) ->
      let result = Image.Metadata.create ~width_px ~height_px ~frames in
      print_s [%sexp (Result.is_ok result : bool)];
      Result.iter result ~f:(fun metadata ->
        assert (Image.Metadata.width_px metadata = width_px);
        assert (Image.Metadata.height_px metadata = height_px);
        assert (Image.Metadata.frames metadata = frames)));
  [%expect
    {|
    false
    false
    true
    false
    true
    false
    true
    false
    false
    false
    |}]
;;

let%expect_test
    "source changes rotate callbacks; restyling preserves the mounted identity"
  =
  let open Gpuio in
  let module W = Gpuio_protocol.Wire in
  let owner = Asset.Expert.Owner.create () in
  let id = Id.create ~slot:2L ~generation:3L |> Or_error.ok_exn in
  let asset = Asset.Expert.handle ~owner ~id ~format:Png in
  let next =
    Asset.Expert.handle
      ~owner
      ~id:(Id.create ~slot:2L ~generation:4L |> Or_error.ok_exn)
      ~format:Png
  in
  let window =
    Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn
  in
  let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn in
  let reconciler = Reconciler.create ~asset_owner:owner window in
  let prepare asset fit label =
    let config =
      Image.Config.create ~asset ~fit ~description:Image.Description.decorative ()
    in
    Reconciler.prepare
      reconciler
      ~theme:Theme.default
      (Some (View.image ~on_change:(fun state -> label, state) config))
    |> Or_error.ok_exn
  in
  let initial = prepare asset Contain "old" in
  let handler =
    match Reconciler.message initial with
    | Some (Apply { operations; _ }) ->
      List.find_map_exn operations ~f:(function
        | W.Op.Create (_, Image, _, Some handler) -> Some handler
        | _ -> None)
    | _ -> assert false
  in
  Reconciler.accept reconciler initial |> Or_error.ok_exn;
  let changed = prepare asset Cover "new" in
  (match Reconciler.message changed with
   | Some
       (Apply
          { operations =
              [ Set_image (same, { source = Reference found; fit = Cover; label = None })
              ]
          ; _
          }) -> assert (Gpuio_protocol.Node_id.equal same node && Id.equal found id)
   | _ -> assert false);
  Reconciler.accept reconciler changed |> Or_error.ok_exn;
  let event handler revision =
    W.Event.Image_state (window, node, handler, revision, Loading)
  in
  assert (
    [%equal: (string * Image.State.t) option]
      (Reconciler.dispatch reconciler (event handler 1L))
      (Some ("new", Loading)));
  let replacement = prepare next Cover "replacement" in
  let replacement_handler =
    match Reconciler.message replacement with
    | Some (Apply { operations; _ }) ->
      List.find_map_exn operations ~f:(function
        | W.Op.Bind (_, Some handler) -> Some handler
        | _ -> None)
    | _ -> assert false
  in
  assert (not (Gpuio_protocol.Handler_id.equal handler replacement_handler));
  Reconciler.accept reconciler replacement |> Or_error.ok_exn;
  assert (Option.is_none (Reconciler.dispatch reconciler (event handler 1L)));
  assert (Option.is_some (Reconciler.dispatch reconciler (event replacement_handler 3L)));
  assert (Option.is_none (Reconciler.dispatch reconciler (event replacement_handler 4L)));
  let foreign = Reconciler.create window in
  let config = Image.Config.create ~asset ~description:Image.Description.decorative () in
  let foreign_update =
    Reconciler.prepare foreign ~theme:Theme.default (Some (View.image config))
    |> Or_error.ok_exn
  in
  (match Reconciler.message foreign_update with
   | Some (Apply { operations; _ }) ->
     assert (
       List.exists operations ~f:(function
         | W.Op.Set_image (_, { source = Unavailable Wrong_application; _ }) -> true
         | _ -> false))
   | _ -> assert false);
  print_endline
    "restyle retains node/handler; latest callback; source rotation rejects late events; \
     foreign source fails locally";
  [%expect
    {| restyle retains node/handler; latest callback; source rotation rejects late events; foreign source fails locally |}]
;;
