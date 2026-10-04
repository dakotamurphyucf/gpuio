open Core
open Gpuio
module C = Document.Config
module D = Document.Defaults

let ok = Or_error.ok_exn

let source slot =
  Text_source.Expert.handle
    ~owner:(Text_source.Expert.Owner.create ())
    (Gpuio_protocol.Resource_id.create ~slot ~generation:1L |> ok)
;;

let config ?(mode = Document.Mode.Markdown) ?layout ?selection_format ?markdown_options ()
  =
  C.create ~source:(source 0L) ~mode ?layout ?selection_format ?markdown_options () |> ok
;;

let describe t =
  print_s
    [%sexp
      (C.appearance t : Document.Appearance.t)
    , (C.layout t : Document.Layout.t)
    , (C.selection_format t : Document.Selection_format.t)
    , (C.max_lines t : int option)
    , (Option.is_some (C.text_style t) : bool)
    , (C.markdown_options t : Document.Markdown_options.t)]
;;

let defaults () =
  D.create
    ~appearance:Dark
    ~selection_format:Markdown
    ~max_lines:6
    ~text_style:(Document.Style.create ~paragraph_gap_rem:1.5 () |> ok)
    ~markdown_options:(Document.Markdown_options.create ~mdx:true ())
    ()
  |> ok
;;

let%expect_test "omission, explicit builtin, reset and restored inheritance" =
  let defaults = defaults () in
  let inherited = config () in
  let resolve t = D.Expert.resolve defaults t |> ok in
  let explicit =
    C.create
      ~source:(C.source inherited)
      ~mode:Markdown
      ~selection_format:Plain_text
      ~markdown_options:Document.Markdown_options.default
      ()
    |> ok
  in
  assert (not (C.equal inherited explicit));
  let metrics =
    D.create
      ~layout:(Viewport 240.)
      ~max_lines:8
      ~line_numbers:false
      ~initially_collapsed:true
      ()
    |> ok
  in
  let scoped = D.Expert.resolve metrics inherited |> ok in
  assert (Document.Layout.equal (C.layout scoped) (Viewport 240.));
  assert (not (C.line_numbers scoped));
  assert (C.initially_collapsed scoped);
  assert (Option.is_none (C.max_lines scoped));
  let reset_metrics =
    C.with_overrides
      inherited
      ~layout:Builtin
      ~line_numbers:Builtin
      ~initially_collapsed:Builtin
      ()
    |> ok
    |> D.Expert.resolve metrics
    |> ok
  in
  assert (Document.Layout.equal (C.layout reset_metrics) Flow);
  assert (C.line_numbers reset_metrics && not (C.initially_collapsed reset_metrics));
  assert (Option.equal Int.equal (C.max_lines reset_metrics) (Some 8));
  describe (resolve inherited);
  describe (resolve explicit);
  let reset =
    C.with_overrides
      inherited
      ~appearance:Builtin
      ~selection_format:Builtin
      ~max_lines:Builtin
      ~text_style:Builtin
      ~markdown_options:Builtin
      ()
    |> ok
  in
  describe (resolve reset);
  let restored =
    C.with_overrides
      reset
      ~appearance:Inherit
      ~selection_format:Inherit
      ~max_lines:Inherit
      ~text_style:Inherit
      ~markdown_options:Inherit
      ()
    |> ok
  in
  assert (C.equal (resolve inherited) (resolve restored));
  (* Resolving does not erase intent or couple an immutable config to an app. *)
  assert (C.equal (D.Expert.resolve D.empty (resolve inherited) |> ok) inherited);
  let changed = C.with_overrides explicit ~max_lines:(Value 3) () |> ok in
  assert (
    Document.Selection_format.equal (C.selection_format (resolve changed)) Plain_text);
  assert (Option.equal Int.equal (C.max_lines (resolve changed)) (Some 3));
  [%expect
    {|
    (Dark Flow Markdown (6) true ((frontmatter Disabled) (mdx true)))
    (Dark Flow Plain_text (6) true ((frontmatter Disabled) (mdx false)))
    (Light Flow Plain_text () false ((frontmatter Disabled) (mdx false)))
  |}]
;;

let%expect_test "inapplicable inherited settings and invalid explicit values" =
  let defaults = defaults () in
  List.iter [ Document.Mode.Html; Code Document.Language.ocaml; Diff ] ~f:(fun mode ->
    let resolved = D.Expert.resolve defaults (config ~mode ()) |> ok in
    assert (
      Document.Markdown_options.equal
        (C.markdown_options resolved)
        Document.Markdown_options.default);
    describe resolved);
  let viewport = D.Expert.resolve defaults (config ~layout:(Viewport 200.) ()) |> ok in
  assert (Option.is_none (C.max_lines viewport));
  List.iter [ 0; 4097 ] ~f:(fun n ->
    assert (Result.is_error (D.create ~max_lines:n ()));
    assert (Result.is_error (C.with_overrides (config ()) ~max_lines:(Value n) ())));
  List.iter [ Float.nan; Float.infinity; 0.; 16385. ] ~f:(fun height ->
    assert (Result.is_error (D.create ~layout:(Viewport height) ())));
  assert (
    Result.is_error
      (C.with_overrides
         (config ~mode:Diff ())
         ~markdown_options:(Value (Document.Markdown_options.create ~mdx:true ()))
         ()));
  assert (Result.is_error (C.with_overrides viewport ~max_lines:(Value 1) ()));
  [%expect
    {|
    (Dark Flow Markdown (6) true ((frontmatter Disabled) (mdx false)))
    (Dark Flow Markdown () false ((frontmatter Disabled) (mdx false)))
    (Dark Flow Markdown () false ((frontmatter Disabled) (mdx false)))
  |}]
;;

let%expect_test "default profile callbacks keep document source context and mode scope" =
  let schema =
    Document.Profile.Schema.create
      ~name:"test.defaults"
      ~version:1
      ~fingerprint:(String.make 64 'a')
    |> ok
  in
  let codec =
    Document.Profile.Codec.create
      ~max_bytes:8
      ~encode:(fun s -> Ok s)
      ~decode:(fun s -> Ok s)
    |> ok
  in
  let definition =
    Document.Profile.Definition.create ~schema ~properties:codec ~events:codec |> ok
  in
  let instance =
    Document.Profile.Instance.create definition ~generation:1L "props" |> ok
  in
  let defaults =
    D.with_profile (defaults ()) instance ~on_event:(fun config event ->
      Text_source.Expert.native_id (C.source config), event.signal)
  in
  let wire : Gpuio_protocol.Document_profile_wire.Event.t =
    { config_epoch = 1L
    ; instance_generation = 1L
    ; source_generation = 2L
    ; source_revision = 3L
    ; signal = Data "clicked"
    }
  in
  List.iter [ 1L; 2L ] ~f:(fun slot ->
    let config =
      C.create ~source:(source slot) ~mode:Markdown ()
      |> ok
      |> D.Expert.resolve defaults
      |> ok
    in
    let _, callback = D.Expert.profile defaults config |> Option.value_exn in
    let id, signal = callback wire |> Option.value_exn in
    assert (
      Gpuio_protocol.Resource_id.equal id (Text_source.Expert.native_id (C.source config)));
    print_s [%sexp (signal : string Document.Profile.Signal.t)]);
  assert (Option.is_none (D.Expert.profile defaults (config ~mode:Diff ())));
  assert (Option.is_none (D.Expert.profile D.empty (config ())));
  [%expect
    {|
    (Data clicked)
    (Data clicked)
  |}]
;;

let%expect_test "reconcilers isolate defaults and fence inherited profile callbacks" =
  let owner = Text_source.Expert.Owner.create () in
  let source_id = Gpuio_protocol.Resource_id.create ~slot:3L ~generation:1L |> ok in
  let source = Text_source.Expert.handle ~owner source_id in
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let schema =
    Document.Profile.Schema.create
      ~name:"test.defaults"
      ~version:1
      ~fingerprint:(String.make 64 'a')
    |> ok
  in
  let codec =
    Document.Profile.Codec.create
      ~max_bytes:8
      ~encode:(fun s -> Ok s)
      ~decode:(fun s -> Ok s)
    |> ok
  in
  let definition =
    Document.Profile.Definition.create ~schema ~properties:codec ~events:codec |> ok
  in
  let instance generation =
    Document.Profile.Instance.create definition ~generation "x" |> ok
  in
  let defaults =
    D.with_profile (defaults ()) (instance 2L) ~on_event:(fun config _ -> C.label config)
  in
  let configured =
    Reconciler.create ~document_owner:owner ~document_defaults:defaults window
  in
  let ordinary = Reconciler.create ~document_owner:owner window in
  let view =
    View.document (C.create ~source ~mode:Markdown ~label:"Inherited source" () |> ok)
  in
  let prepare reconciler view =
    Reconciler.prepare reconciler ~theme:Theme.default (Some view)
  in
  let operations update =
    match Reconciler.message update with
    | Some (Apply tx) -> tx.operations
    | None -> []
    | Some _ -> assert false
  in
  let first = prepare configured view |> ok in
  let other = prepare ordinary view |> ok in
  let dark update =
    List.find_map_exn (operations update) ~f:(function
      | Gpuio_protocol.Wire.Op.Set_document (_, config) -> Some config.dark
      | _ -> None)
  in
  assert (dark first && not (dark other));
  assert (
    not
      (List.exists (operations other) ~f:(function
         | Set_document_profile _ -> true
         | _ -> false)));
  let node, handler =
    List.find_map_exn (operations first) ~f:(function
      | Gpuio_protocol.Wire.Op.Create (node, _, _, Some handler) -> Some (node, handler)
      | _ -> None)
  in
  assert (
    List.exists (operations first) ~f:(function
      | Gpuio_protocol.Wire.Op.Set_document_profile (_, { epoch = 1L; instance = Some _ })
        -> true
      | _ -> false));
  Reconciler.accept configured first |> ok;
  Reconciler.accept ordinary other |> ok;
  let event : Gpuio_protocol.Document_profile_wire.Event.t =
    { config_epoch = 1L
    ; instance_generation = 2L
    ; source_generation = 1L
    ; source_revision = 1L
    ; signal = Data "clicked"
    }
  in
  let dispatch () =
    Reconciler.dispatch
      configured
      (Gpuio_protocol.Wire.Event.Document_profile_event
         (window, node, handler, 1L, source_id, event))
  in
  assert (Option.equal String.equal (dispatch ()) (Some "Inherited source"));
  let unchanged = prepare configured view |> ok in
  assert (Option.is_none (Reconciler.message unchanged));
  Reconciler.accept configured unchanged |> ok;
  let invalid =
    View.with_document_profile view (instance 1L) ~on_event:(fun _ -> "invalid") |> ok
  in
  assert (Result.is_error (prepare configured invalid));
  assert (Option.equal String.equal (dispatch ()) (Some "Inherited source"));
  let clear = prepare configured (View.without_document_profile view |> ok) |> ok in
  assert (
    List.exists (operations clear) ~f:(function
      | Gpuio_protocol.Wire.Op.Set_document_profile (_, { epoch = 2L; instance = None })
        -> true
      | _ -> false));
  Reconciler.accept configured clear |> ok;
  assert (Option.is_none (dispatch ()));
  let restored = prepare configured view |> ok in
  assert (
    List.exists (operations restored) ~f:(function
      | Gpuio_protocol.Wire.Op.Set_document_profile (_, { epoch = 3L; instance = Some _ })
        -> true
      | _ -> false));
  Reconciler.accept configured restored |> ok;
  print_endline
    "isolated defaults, shared subtree, typed callback, failed-prepare rollback, clear \
     and inherit";
  [%expect
    {| isolated defaults, shared subtree, typed callback, failed-prepare rollback, clear and inherit |}]
;;

let%expect_test "shared action handlers yield to local callbacks and clear with actions" =
  let owner = Text_source.Expert.Owner.create () in
  let source_id = Gpuio_protocol.Resource_id.create ~slot:4L ~generation:1L |> ok in
  let source = Text_source.Expert.handle ~owner source_id in
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let actions = Document_actions_test.config () in
  let defaults =
    D.create ~actions ~on_action:(fun config _ -> C.label config ^ " default") () |> ok
  in
  let reconciler =
    Reconciler.create ~document_owner:owner ~document_defaults:defaults window
  in
  let config = C.create ~source ~mode:Markdown ~label:"Shared source" () |> ok in
  let publish view =
    let update = Reconciler.prepare reconciler ~theme:Theme.default (Some view) |> ok in
    let ops =
      match Reconciler.message update with
      | Some (Apply tx) -> tx.operations
      | None -> []
      | _ -> assert false
    in
    Reconciler.accept reconciler update |> ok;
    ops
  in
  let ops = publish (View.document config) in
  let node, handler =
    List.find_map_exn ops ~f:(function
      | Gpuio_protocol.Wire.Op.Create (node, _, _, Some handler) -> Some (node, handler)
      | _ -> None)
  in
  let dispatch () =
    Reconciler.dispatch
      reconciler
      (Gpuio_protocol.Wire.Event.Document_action
         (window, node, handler, 1L, source_id, Document_actions_test.event))
  in
  assert (Option.equal String.equal (dispatch ()) (Some "Shared source default"));
  assert (List.is_empty (publish (View.document ~on_action:(fun _ -> "local") config)));
  assert (Option.equal String.equal (dispatch ()) (Some "local"));
  ignore (publish (View.document (C.with_overrides config ~actions:Builtin () |> ok)));
  assert (Option.is_none (dispatch ()));
  let unhandled =
    Reconciler.create
      ~document_owner:owner
      ~document_defaults:(D.create ~actions () |> ok)
      window
  in
  assert (
    Result.is_error
      (Reconciler.prepare unhandled ~theme:Theme.default (Some (View.document config))));
  print_endline
    "default context, local callback refresh, explicit actions reset, missing-handler \
     rejection";
  [%expect
    {| default context, local callback refresh, explicit actions reset, missing-handler rejection |}]
;;
