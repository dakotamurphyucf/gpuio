open Core
module H = Gpuio.Highlight
module W = Gpuio_protocol.Highlight_wire

let ok = Or_error.ok_exn
let range start_byte end_byte = H.Range.create ~start_byte ~end_byte |> ok
let query = H.Query.create ~whole_word:true "café" |> ok

let fixture : W.Config.t =
  [ { query = Some { text = "café"; case_sensitive = false; whole_word = true }
    ; ranges = [ { start_byte = 0L; end_byte = 5L }; { start_byte = 6L; end_byte = 9L } ]
    ; appearance = { color = 1L; active_color = 2L; radius = 2. }
    ; active_index = Some 130L
    ; match_index_offset = 129L
    }
  ]
;;

let%expect_test "independent config bytes preserve UTF-8 and match units" =
  let t = H.Expert.of_wire fixture |> ok in
  assert (W.Config.equal (H.Expert.to_wire t) fixture);
  let bytes = Bin_prot.Utils.bin_dump W.Config.bin_writer_t fixture in
  let pos_ref = ref 0 in
  let decoded = W.Config.bin_read_t bytes ~pos_ref in
  assert (!pos_ref = Bigstring.length bytes);
  assert (W.Config.equal decoded fixture);
  let hex =
    Bigstring.to_string bytes
    |> String.to_list
    |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
    |> String.concat
  in
  print_endline hex;
  print_s
    [%sexp (H.Query.text query : string), (H.Config.specs H.Config.empty : H.Spec.t list)];
  [%expect
    {|
    010105636166c3a9000102000506090102000000000000004001fe8200fe8100
    ("caf\195\169" ())
    |}]
;;

let%expect_test "validated query, byte endpoints and aggregate bounds" =
  List.iter
    [ ""; "\255"; "a\000b"; String.make 4097 'x' ]
    ~f:(fun text -> assert (Result.is_error (H.Query.create text)));
  List.iter
    [ " "; "İΣﬀ"; String.make 4096 'x' ]
    ~f:(fun text -> assert (Result.is_ok (H.Query.create text)));
  List.iter
    [ -1L, 1L; 0L, 0L; 2L, 1L ]
    ~f:(fun (start_byte, end_byte) ->
      assert (Result.is_error (H.Range.create ~start_byte ~end_byte)));
  let r = range Int64.(max_value - 1L) Int64.max_value in
  assert (Int64.equal (H.Range.start_byte r) Int64.(max_value - 1L));
  assert (Int64.equal (H.Range.end_byte r) Int64.max_value);
  assert (Result.is_error (H.Spec.create ()));
  assert (Result.is_error (H.Spec.create ~query ~active_index:(-1L) ()));
  assert (Result.is_error (H.Spec.create ~query ~match_index_offset:(-1L) ()));
  let spec = H.Spec.create ~query ~active_index:Int64.max_value () |> ok in
  assert (Result.is_ok (H.Config.create (List.init 16 ~f:(fun _ -> spec))));
  assert (Result.is_error (H.Config.create (List.init 17 ~f:(fun _ -> spec))));
  let ranges = List.init 4096 ~f:(fun _ -> r) in
  let full = H.Spec.create ~ranges () |> ok in
  assert (Result.is_error (H.Spec.create ~ranges:(r :: ranges) ()));
  assert (Result.is_ok (H.Config.create [ full; spec ]));
  let one = H.Spec.create ~ranges:[ r ] () |> ok in
  assert (Result.is_error (H.Config.create [ full; one ]));
  assert (H.Config.equal H.Config.empty (H.Config.create [] |> ok));
  print_endline "UTF-8, signed-64 endpoints,16 specs and4096 aggregate ranges";
  [%expect {| UTF-8, signed-64 endpoints,16 specs and4096 aggregate ranges |}]
;;

let%expect_test "appearance resolves tokens and scales existing alpha" =
  let theme =
    Gpuio.Theme.create
      [ "accent", Gpuio.Color.rgba ~red:1 ~green:2 ~blue:3 ~alpha:128 |> ok ]
    |> ok
  in
  let appearance = H.Appearance.create ~theme () |> ok in
  let spec = H.Spec.create ~query ~appearance () |> ok in
  let wire = H.Config.create [ spec ] |> ok |> H.Expert.to_wire |> List.hd_exn in
  print_s [%sexp (wire.appearance : W.Appearance.t)];
  let explicit =
    H.Appearance.create
      ~theme:(Gpuio.Theme.create [] |> ok)
      ~color:(Gpuio.Color.rgb_exn 0xabcdef)
      ~active_color:(Gpuio.Color.rgb_exn 0x123456)
      ()
    |> ok
  in
  assert (not (H.Appearance.equal appearance explicit));
  assert (Result.is_error (H.Appearance.create ~theme:(Gpuio.Theme.create [] |> ok) ()));
  assert (
    Result.is_error (H.Appearance.create ~color:(Gpuio.Color.token_exn "missing") ()));
  List.iter
    [ Float.nan; Float.infinity; Float.neg_infinity; -1.; 64.01 ]
    ~f:(fun radius -> assert (Result.is_error (H.Appearance.create ~radius ())));
  assert (Result.is_ok (H.Appearance.create ~radius:0. ()));
  assert (Result.is_ok (H.Appearance.create ~radius:64. ()));
  [%expect {| ((color 16909094) (active_color 16909139) (radius 2)) |}]
;;

let%expect_test "raw wire values cannot bypass semantic constructors" =
  let base = List.hd_exn fixture in
  List.iter
    [ { base with query = None; ranges = [] }
    ; { base with
        query = Some { text = "\255"; case_sensitive = true; whole_word = false }
      }
    ; { base with ranges = [ { start_byte = 1L; end_byte = 0L } ] }
    ; { base with appearance = { base.appearance with color = -1L } }
    ; { base with appearance = { base.appearance with active_color = 0x1_0000_0000L } }
    ; { base with appearance = { base.appearance with radius = Float.nan } }
    ; { base with active_index = Some (-1L) }
    ; { base with match_index_offset = -1L }
    ]
    ~f:(fun spec -> assert (Result.is_error (H.Expert.of_wire [ spec ])));
  print_endline "all invalid wire configs rejected";
  [%expect {| all invalid wire configs rejected |}]
;;
